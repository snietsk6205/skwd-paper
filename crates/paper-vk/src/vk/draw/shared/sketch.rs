//! "Sketch before transition" for every transition path (videos, images,
//! Wallpaper Engine scenes and the Settings preview all end up in
//! `render_fade_to`, `render_effect_to` or `render_sand_to`).
//!
//! On the first frame of a transition the incoming frame is frozen into an
//! offscreen texture and analysed once (edges, pressure, drawing order). Each
//! sketch frame then only reads those cached textures, so a moving video
//! neither shimmers nor costs a full edge detection per frame. When the
//! drawing is complete, the transition that was asked for runs from the
//! finished sketch to the live new wallpaper.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use ash::vk;

use super::super::model::Src;
use super::model::{AvvkFrame, WaitSems};
use super::targets::{IDENTITY_UV, TransitionPush};
use crate::sketch::{Origin, SketchMedia};
use crate::vk::pipelines::SketchPipe;
use crate::vk::sketch::{SketchCache, SketchStep};
use crate::vk::{ExportImage, RenderTarget, Renderer, SceneTarget};

/// A pause this long between transition frames means a new transition.
const NEW_TRANSITION_GAP: Duration = Duration::from_millis(1500);
/// Cached textures are freed when no transition frame came for this long.
const IDLE_RELEASE: Duration = Duration::from_secs(3);
/// The outgoing wallpaper is fully covered by the paper after this much of the drawing.
const COVER_END: f32 = 0.17;

const SET_FROZEN: usize = 0;
const SET_ANALYSIS_A: usize = 1;
const SET_ANALYSIS: usize = 2;

impl crate::sketch::Settings {
    /// Whether transitions need the sketch module at all.
    pub(crate) fn active(self) -> bool {
        self.sketch.is_some() || self.origin != Origin::Default
    }
}

impl Renderer {
    /// Tells the sketch what the current transition is heading to, so the
    /// "also sketch videos / scenes" settings can apply.
    pub fn set_transition_media(&mut self, media: SketchMedia) {
        self.sketch.media = media;
    }

    /// Decides what this transition frame draws. `t` is the linear progress.
    pub(super) fn sketch_step(
        &mut self,
        src_b: &Src,
        uv_b: [f32; 4],
        t: f32,
    ) -> Result<SketchStep> {
        let settings = crate::sketch::settings();
        let t = t.clamp(0.0, 1.0);
        let now = Instant::now();
        let fresh = self.sketch.last.is_none_or(|(last, at)| {
            t + 0.02 < last || now.duration_since(at) > NEW_TRANSITION_GAP
        });
        self.sketch.last = Some((t, now));
        let sketch = settings.sketch.filter(|sketch| sketch.allows(self.sketch.media));
        if fresh {
            self.begin_sketch_transition(src_b, uv_b, sketch.is_some(), settings.origin)?;
        }
        if let Some(skipped) = settings.sketch.filter(|_| sketch.is_none()) {
            // The daemon lengthened the transition for a sketch this media
            // skips: run the transition in its own time, then hold the result.
            let remapped = t / (1.0 - skipped.split).max(0.05);
            return Ok(if remapped >= 1.0 {
                SketchStep::Done
            } else {
                SketchStep::Plain(remapped)
            });
        }
        let Some(sketch) = sketch.filter(|_| self.sketch.drawing) else {
            return Ok(SketchStep::Plain(t));
        };
        if t < sketch.split {
            return Ok(SketchStep::Drawing(t / sketch.split));
        }
        Ok(SketchStep::FromSketch((t - sketch.split) / (1.0 - sketch.split)))
    }

    /// Frees the cached textures once transitions have stopped for a while.
    pub(super) fn release_idle_sketch(&mut self) -> Result<()> {
        if self.sketch.cache.is_none() && self.sketch.density.is_none() {
            return Ok(());
        }
        if self.sketch.last.is_some_and(|(_, at)| at.elapsed() < IDLE_RELEASE) {
            return Ok(());
        }
        self.wait_frame_complete()?;
        let device = self.device.clone();
        self.sketch.release_cache(&device);
        self.sketch.drawing = false;
        Ok(())
    }

    fn begin_sketch_transition(
        &mut self,
        src_b: &Src,
        uv_b: [f32; 4],
        drawing: bool,
        origin: Origin,
    ) -> Result<()> {
        self.sketch.drawing = false;
        self.sketch.finished_ready = false;
        self.sketch.seed = crate::sketch::settings()
            .sketch
            .and_then(|sketch| sketch.seed)
            .unwrap_or((crate::sketch::random_u32() & 0x3ff) as i32);
        self.sketch.origin = match origin {
            Origin::Default => None,
            Origin::Centre => Some([0.5, 0.5]),
            Origin::Random => Some(crate::sketch::random_origin()),
            Origin::Dense => None,
        };
        if !drawing && origin != Origin::Dense {
            return Ok(());
        }
        let started = Instant::now();
        let prepared = self
            .ensure_sketch_resources()
            .and_then(|()| self.freeze_incoming(src_b, uv_b))
            .and_then(|()| self.run_analysis(0));
        if let Err(error) = prepared {
            // A sketch is a nicety: fall back to the plain transition.
            tracing::warn!(
                "skwd-wall-vk: sketch unavailable, using the plain transition: {error:#}"
            );
            return Ok(());
        }
        if origin == Origin::Dense {
            match self.densest_point() {
                Ok(point) => self.sketch.origin = point,
                Err(error) => {
                    tracing::info!("skwd-wall-vk: busiest-area start point failed: {error:#}")
                }
            }
        }
        if drawing {
            if let Err(error) = self.run_analysis(1) {
                tracing::warn!(
                    "skwd-wall-vk: sketch unavailable, using the plain transition: {error:#}"
                );
                return Ok(());
            }
            self.sketch.drawing = true;
        }
        tracing::info!(
            drawing,
            origin = ?self.sketch.origin,
            media = ?self.sketch.media,
            ms = started.elapsed().as_secs_f64() * 1000.0,
            "skwd-wall-vk: sketch prepared"
        );
        Ok(())
    }

    /// The completed drawing, rendered once per transition.
    pub(super) fn sketch_finished_view(&mut self) -> Result<vk::ImageView> {
        if !self.sketch.finished_ready {
            let target = self.sketch_target(|cache| &cache.finished)?;
            self.sketch_pass(target, self.sketch.pipes.compose, SET_ANALYSIS, SET_FROZEN, 1.0, 0)?;
            self.sketch.finished_ready = true;
        }
        let cache = self.sketch.cache.as_ref().ok_or_else(|| anyhow!("sketch cache missing"))?;
        Ok(cache.finished.view)
    }

    /// One frame of the drawing: the outgoing wallpaper (while the paper is
    /// still sliding over it), then the sketch at drawing progress `q`.
    pub(super) fn render_sketch_frame(
        &mut self,
        rt: &RenderTarget,
        export: &mut ExportImage,
        src_a: &Src,
        uv_a: [f32; 4],
        q: f32,
    ) -> Result<()> {
        let q = q.clamp(0.0, 1.0);
        let show_a = q < COVER_END;
        let uv_a = src_a.texture_uv(uv_a);
        self.ensure_src_pipelines(&[src_a])?;
        self.wait_frame_complete()?;
        let avf_a = if show_a { Self::resolve_src(src_a)? } else { None };
        let mut waits = WaitSems::new();
        if let Some(avf) = &avf_a {
            avf.push_wait_sems(&mut waits);
        }
        self.begin_frame_cmd()?;
        if export.direct_render {
            self.acquire_direct_export(export);
        }
        if show_a {
            self.acquire_imports(&[src_a]);
            self.bind_src(self.desc_set, src_a, &avf_a)?;
            if let Some(avf) = &avf_a {
                self.sample_barrier(&[avf.read_barrier()], vk::PipelineStageFlags::FRAGMENT_SHADER);
            }
        }
        self.begin_clear_pass(rt.framebuffer);
        if show_a {
            self.draw_fullscreen(self.present_pipeline(src_a), self.desc_set, &uv_a);
        }
        self.draw_sketch_compose(q);
        unsafe { self.device.cmd_end_render_pass(self.cmd) };
        if show_a {
            self.release_imports(&[src_a]);
        }
        if export.direct_render {
            self.release_direct_export(export);
        } else {
            self.copy_rt_to_export(rt, export);
        }
        self.end_and_submit(&waits)?;
        if show_a {
            Self::note_imports(&[src_a]);
        }
        export.ready = true;
        if let Some(avf) = avf_a {
            avf.commit_sampled();
        }
        Ok(())
    }

    fn draw_sketch_compose(&self, q: f32) {
        let push = self.sketch_push(q, 0, self.extent);
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                self.cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout_sand,
                0,
                &[self.sketch.sets[SET_ANALYSIS], self.sketch.sets[SET_FROZEN]],
                &[],
            );
        }
        self.push_transition(&push);
        unsafe {
            self.device.cmd_bind_pipeline(
                self.cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.sketch.pipes.compose,
            );
            self.device.cmd_draw(self.cmd, 3, 1, 0, 0);
        }
    }

    fn sketch_push(&self, progress: f32, mode: i32, extent: vk::Extent2D) -> TransitionPush {
        let (origin, origin_on) = self.sketch.origin.map_or(([0.5, 0.45], 0), |origin| (origin, 1));
        let style = crate::sketch::settings().sketch.map_or(0, |sketch| sketch.style);
        TransitionPush {
            res: [extent.width as f32, extent.height as f32],
            progress,
            style: style as i32,
            pid_base: mode,
            fill: 0,
            rgba_a: 1,
            rgba_b: 1,
            uv_a: IDENTITY_UV,
            uv_b: IDENTITY_UV,
            origin,
            origin_on,
            seed: self.sketch.seed,
        }
    }

    /// Copies the first frame of the incoming wallpaper, already fitted to the
    /// screen, into the frozen texture.
    fn freeze_incoming(&mut self, src: &Src, uv: [f32; 4]) -> Result<()> {
        let target = self.sketch_target(|cache| &cache.frozen)?;
        let uv = src.texture_uv(uv);
        self.ensure_src_pipelines(&[src])?;
        self.wait_frame_complete()?;
        let avf: Option<AvvkFrame<'_>> = Self::resolve_src(src)?;
        let mut waits = WaitSems::new();
        if let Some(avf) = &avf {
            avf.push_wait_sems(&mut waits);
        }
        self.begin_frame_cmd()?;
        self.acquire_imports(&[src]);
        self.bind_src(self.desc_set, src, &avf)?;
        if let Some(avf) = &avf {
            self.sample_barrier(&[avf.read_barrier()], vk::PipelineStageFlags::FRAGMENT_SHADER);
        }
        self.begin_target_pass(target);
        self.draw_fullscreen(self.present_pipeline(src), self.desc_set, &uv);
        unsafe { self.device.cmd_end_render_pass(self.cmd) };
        self.release_imports(&[src]);
        self.end_and_submit(&waits)?;
        Self::note_imports(&[src]);
        if let Some(avf) = avf {
            avf.commit_sampled();
        }
        self.wait_frame_complete()
    }

    fn run_analysis(&mut self, mode: i32) -> Result<()> {
        let target = self
            .sketch_target(|cache| if mode == 0 { &cache.analysis_a } else { &cache.analysis_b })?;
        self.sketch_pass(target, self.sketch.pipes.analysis, SET_FROZEN, SET_FROZEN, 0.0, mode)
    }

    /// Renders the line-density map, reads it back and picks the busiest area.
    fn densest_point(&mut self) -> Result<Option<[f32; 2]>> {
        let (width, height) = paper_shaders::SKETCH_DENSITY_SIZE;
        let target = {
            let density =
                self.sketch.density.as_ref().ok_or_else(|| anyhow!("no density target"))?;
            TargetHandles::of(density)
        };
        self.sketch_pass(
            target,
            self.sketch.pipes.density,
            SET_ANALYSIS_A,
            SET_ANALYSIS_A,
            0.0,
            0,
        )?;
        let image = self.sketch.density.as_ref().map(|density| density.image).unwrap_or_default();
        let readback = self.create_readback_buf(u64::from(width * height * 4))?;
        self.begin_frame_cmd()?;
        let range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };
        unsafe {
            let to_src = [vk::ImageMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::SHADER_READ)
                .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
                .old_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(range)];
            self.device.cmd_pipeline_barrier(
                self.cmd,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &to_src,
            );
            self.device.cmd_copy_image_to_buffer(
                self.cmd,
                image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                readback.buffer,
                &[vk::BufferImageCopy {
                    buffer_offset: 0,
                    buffer_row_length: 0,
                    buffer_image_height: 0,
                    image_subresource: vk::ImageSubresourceLayers {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        mip_level: 0,
                        base_array_layer: 0,
                        layer_count: 1,
                    },
                    image_offset: vk::Offset3D::default(),
                    image_extent: vk::Extent3D { width, height, depth: 1 },
                }],
            );
            let to_host = [vk::BufferMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::HOST_READ)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .buffer(readback.buffer)
                .offset(0)
                .size(vk::WHOLE_SIZE)];
            self.device.cmd_pipeline_barrier(
                self.cmd,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::HOST,
                vk::DependencyFlags::empty(),
                &[],
                &to_host,
                &[],
            );
        }
        self.end_and_submit(&WaitSems::new())?;
        self.wait_frame_complete()?;
        let pixels =
            unsafe { std::slice::from_raw_parts(readback.ptr, (width * height * 4) as usize) };
        let red: Vec<u8> = pixels.chunks_exact(4).map(|pixel| pixel[0]).collect();
        Ok(crate::sketch::densest(&red, width as usize, height as usize))
    }

    /// A full-screen sketch pass into one of the cached targets.
    fn sketch_pass(
        &mut self,
        target: TargetHandles,
        pipeline: vk::Pipeline,
        set_a: usize,
        set_b: usize,
        progress: f32,
        mode: i32,
    ) -> Result<()> {
        let push = self.sketch_push(progress, mode, target.extent);
        self.wait_frame_complete()?;
        self.begin_frame_cmd()?;
        self.begin_target_pass(target);
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                self.cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout_sand,
                0,
                &[self.sketch.sets[set_a], self.sketch.sets[set_b]],
                &[],
            );
        }
        self.push_transition(&push);
        unsafe {
            self.device.cmd_bind_pipeline(self.cmd, vk::PipelineBindPoint::GRAPHICS, pipeline);
            self.device.cmd_draw(self.cmd, 3, 1, 0, 0);
            self.device.cmd_end_render_pass(self.cmd);
        }
        self.end_and_submit(&WaitSems::new())?;
        self.wait_frame_complete()
    }

    pub(super) fn begin_target_pass(&self, target: TargetHandles) {
        let clear =
            [vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } }];
        unsafe {
            self.device.cmd_begin_render_pass(
                self.cmd,
                &vk::RenderPassBeginInfo::default()
                    .render_pass(target.render_pass)
                    .framebuffer(target.framebuffer)
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D::default(),
                        extent: target.extent,
                    })
                    .clear_values(&clear),
                vk::SubpassContents::INLINE,
            );
        }
    }

    fn sketch_target(&self, pick: impl Fn(&SketchCache) -> &SceneTarget) -> Result<TargetHandles> {
        let cache = self.sketch.cache.as_ref().ok_or_else(|| anyhow!("sketch cache missing"))?;
        Ok(TargetHandles::of(pick(cache)))
    }

    /// Creates the pipelines, descriptor sets and textures this renderer
    /// needs, or reuses them from the last transition.
    fn ensure_sketch_resources(&mut self) -> Result<()> {
        let device = self.device.clone();
        if self.sketch.cache.as_ref().is_some_and(|cache| cache.extent != self.extent) {
            self.wait_frame_complete()?;
            self.sketch.release_cache(&device);
        }
        if self.sketch.pipes.pass == vk::RenderPass::null() {
            self.sketch.pipes.pass =
                crate::vk::pipelines::create_scene_render_pass(&device, self.format)?;
            self.sketch.pipes.density_pass = crate::vk::pipelines::create_scene_render_pass(
                &device,
                vk::Format::R8G8B8A8_UNORM,
            )?;
        }
        let (dw, dh) = paper_shaders::SKETCH_DENSITY_SIZE;
        let density_extent = vk::Extent2D { width: dw, height: dh };
        if self.sketch.pipes.analysis == vk::Pipeline::null() {
            let pass = self.sketch.pipes.pass;
            self.sketch.pipes.analysis =
                self.build_sketch_pipeline(SketchPipe::Analysis, pass, self.extent)?;
            self.sketch.pipes.compose =
                self.build_sketch_pipeline(SketchPipe::Compose, pass, self.extent)?;
            let density_pass = self.sketch.pipes.density_pass;
            self.sketch.pipes.density =
                self.build_sketch_pipeline(SketchPipe::Density, density_pass, density_extent)?;
        }
        if self.sketch.pool == vk::DescriptorPool::null() {
            let sizes = [vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 6,
            }];
            unsafe {
                self.sketch.pool = device.create_descriptor_pool(
                    &vk::DescriptorPoolCreateInfo::default().max_sets(3).pool_sizes(&sizes),
                    None,
                )?;
                let layouts = [self.desc_layout; 3];
                let sets = device.allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfo::default()
                        .descriptor_pool(self.sketch.pool)
                        .set_layouts(&layouts),
                )?;
                self.sketch.sets = [sets[0], sets[1], sets[2]];
            }
        }
        if self.sketch.cache.is_none() {
            let extent = self.extent;
            let pass = self.sketch.pipes.pass;
            let format = self.format;
            let cache = SketchCache {
                frozen: self.create_sketch_target(extent, format, pass)?,
                analysis_a: self.create_sketch_target(extent, format, pass)?,
                analysis_b: self.create_sketch_target(extent, format, pass)?,
                finished: self.create_sketch_target(extent, format, pass)?,
                extent,
            };
            let sets = self.sketch.sets;
            self.write_view_set(sets[SET_FROZEN], cache.frozen.view, cache.frozen.view);
            self.write_view_set(sets[SET_ANALYSIS_A], cache.analysis_a.view, cache.analysis_a.view);
            self.write_view_set(sets[SET_ANALYSIS], cache.analysis_a.view, cache.analysis_b.view);
            self.sketch.cache = Some(cache);
        }
        if self.sketch.density.is_none() {
            let pass = self.sketch.pipes.density_pass;
            self.sketch.density = Some(self.create_sketch_target(
                density_extent,
                vk::Format::R8G8B8A8_UNORM,
                pass,
            )?);
        }
        Ok(())
    }

    pub(super) fn create_sketch_target(
        &self,
        extent: vk::Extent2D,
        format: vk::Format,
        render_pass: vk::RenderPass,
    ) -> Result<SceneTarget> {
        let device = &self.device;
        unsafe {
            let image = device
                .create_image(
                    &vk::ImageCreateInfo::default()
                        .image_type(vk::ImageType::TYPE_2D)
                        .format(format)
                        .extent(vk::Extent3D {
                            width: extent.width,
                            height: extent.height,
                            depth: 1,
                        })
                        .mip_levels(1)
                        .array_layers(1)
                        .samples(vk::SampleCountFlags::TYPE_1)
                        .tiling(vk::ImageTiling::OPTIMAL)
                        .usage(
                            vk::ImageUsageFlags::COLOR_ATTACHMENT
                                | vk::ImageUsageFlags::SAMPLED
                                | vk::ImageUsageFlags::TRANSFER_SRC,
                        )
                        .sharing_mode(vk::SharingMode::EXCLUSIVE)
                        .initial_layout(vk::ImageLayout::UNDEFINED),
                    None,
                )
                .context("sketch image")?;
            let reqs = device.get_image_memory_requirements(image);
            let memory = match self.device_local_index(reqs).and_then(|index| {
                device
                    .allocate_memory(
                        &vk::MemoryAllocateInfo::default()
                            .allocation_size(reqs.size)
                            .memory_type_index(index),
                        None,
                    )
                    .context("sketch memory")
            }) {
                Ok(memory) => memory,
                Err(error) => {
                    device.destroy_image(image, None);
                    return Err(error);
                }
            };
            device.bind_image_memory(image, memory, 0)?;
            let view = device.create_image_view(
                &vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    }),
                None,
            )?;
            let attachments = [view];
            let framebuffer = device.create_framebuffer(
                &vk::FramebufferCreateInfo::default()
                    .render_pass(render_pass)
                    .attachments(&attachments)
                    .width(extent.width)
                    .height(extent.height)
                    .layers(1),
                None,
            )?;
            Ok(SceneTarget {
                image,
                memory,
                view,
                sampler: self.sampler,
                framebuffer,
                extent,
                allocation_bytes: reqs.size,
                repeat: false,
                format,
                render_pass,
            })
        }
    }
}

/// The handles a pass needs from a target, copied so the target can stay
/// borrowed inside the renderer while the pass records.
#[derive(Clone, Copy)]
pub(super) struct TargetHandles {
    pub(super) framebuffer: vk::Framebuffer,
    pub(super) render_pass: vk::RenderPass,
    pub(super) extent: vk::Extent2D,
}

impl TargetHandles {
    pub(super) fn of(target: &SceneTarget) -> Self {
        Self {
            framebuffer: target.framebuffer,
            render_pass: target.render_pass,
            extent: target.extent,
        }
    }
}
