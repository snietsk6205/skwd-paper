//! Custom build (M3): one entry point for every transition plan, and the
//! combined transitions (`paper_shaders::plan`).
//!
//! A combination draws each transition into its own offscreen texture (the
//! sketch prelude's target helpers) and composes them on screen in the same
//! command buffer. "A then B" runs the first transition to its half-way
//! point, freezes that frame once and runs the second from it. With "sketch
//! before transition" on, the combination starts from the finished sketch.

use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use ash::vk;
use paper_shaders::plan::{Combo, Mode, Part, Plan};

use super::super::model::Src;
use super::model::{AvvkFrame, WaitSems};
use super::sketch::TargetHandles;
use super::targets::{IDENTITY_UV, TransitionPush};
use crate::vk::combo::ComboTargets;
use crate::vk::sketch::SketchStep;
use crate::vk::{ExportImage, RenderTarget, Renderer};

/// "A then B": the first transition's progress at the hand-over.
const HAND_OVER: f32 = 0.5;
/// Combined-transition textures are freed when none was drawn for this long.
const IDLE_RELEASE: Duration = Duration::from_secs(3);
/// Size of the solid black and white mask sources.
const SOLID: vk::Extent2D = vk::Extent2D { width: 8, height: 8 };

const SET_PAIR: usize = 0;
const SET_BLACK: usize = 1;
const SET_WHITE: usize = 2;

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

impl Renderer {
    /// Draws one frame of any transition plan at linear progress `progress`.
    #[allow(clippy::too_many_arguments)]
    pub fn render_plan_to(
        &mut self,
        rt: &RenderTarget,
        export: &mut ExportImage,
        src_a: &Src,
        uv_a: [f32; 4],
        src_b: &Src,
        uv_b: [f32; 4],
        progress: f32,
        plan: Plan,
    ) -> Result<()> {
        match plan {
            Plan::Single(Part::Sand(style)) => {
                self.render_sand_to(rt, export, src_a, uv_a, src_b, uv_b, progress, style)
            }
            Plan::Single(Part::Effect(fx)) => {
                self.render_effect_to(rt, export, src_a, uv_a, src_b, uv_b, progress, fx)
            }
            Plan::Single(Part::Fade) => {
                self.render_fade_to(rt, export, src_a, uv_a, src_b, uv_b, smooth(progress))
            }
            Plan::Combo(combo) => {
                self.render_combo_to(rt, export, src_a, uv_a, src_b, uv_b, progress, combo)
            }
        }
    }

    /// Builds the pipelines a plan needs ahead of its first frame.
    pub fn prepare_plan(&mut self, plan: Plan) -> Result<()> {
        match plan {
            Plan::Single(part) => self.prepare_part(part),
            Plan::Combo(combo) => {
                self.prepare_part(combo.first)?;
                self.prepare_part(combo.second)
            }
        }
    }

    fn prepare_part(&mut self, part: Part) -> Result<()> {
        match part {
            Part::Effect(fx) => self.effect_pipeline(fx).map(|_| ()),
            Part::Sand(_) => self.ensure_transition_pipelines(),
            Part::Fade => {
                self.ensure_transition_pipelines()?;
                self.effect_pipeline(paper_shaders::plan::crossfade_effect()).map(|_| ())
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_combo_to(
        &mut self,
        rt: &RenderTarget,
        export: &mut ExportImage,
        src_a: &Src,
        uv_a: [f32; 4],
        src_b: &Src,
        uv_b: [f32; 4],
        progress: f32,
        combo: Combo,
    ) -> Result<()> {
        if !crate::sketch::settings().active() {
            return self.render_combo_plain(rt, export, src_a, uv_a, src_b, uv_b, progress, combo);
        }
        match self.sketch_step(src_b, uv_b, progress)? {
            SketchStep::Plain(t) => {
                self.render_combo_plain(rt, export, src_a, uv_a, src_b, uv_b, t, combo)
            }
            SketchStep::Drawing(q) => self.render_sketch_frame(rt, export, src_a, uv_a, q),
            SketchStep::FromSketch(t) => {
                let from = Src::Rgba(self.sketch_finished_view()?);
                self.render_combo_plain(rt, export, &from, IDENTITY_UV, src_b, uv_b, t, combo)
            }
            SketchStep::Done => self.render_to(rt, export, src_b, uv_b),
        }
    }

    /// One part drawn straight to the screen (no offscreen pass).
    #[allow(clippy::too_many_arguments)]
    fn render_part_plain(
        &mut self,
        rt: &RenderTarget,
        export: &mut ExportImage,
        src_a: &Src,
        uv_a: [f32; 4],
        src_b: &Src,
        uv_b: [f32; 4],
        t: f32,
        part: Part,
    ) -> Result<()> {
        match part {
            Part::Sand(style) => {
                self.render_sand_plain(rt, export, src_a, uv_a, src_b, uv_b, t, style)
            }
            Part::Effect(fx) => {
                self.render_effect_plain(rt, export, src_a, uv_a, src_b, uv_b, t, fx)
            }
            Part::Fade => self.render_fade_plain(rt, export, src_a, uv_a, src_b, uv_b, smooth(t)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_combo_plain(
        &mut self,
        rt: &RenderTarget,
        export: &mut ExportImage,
        src_a: &Src,
        uv_a: [f32; 4],
        src_b: &Src,
        uv_b: [f32; 4],
        t: f32,
        combo: Combo,
    ) -> Result<()> {
        let t = t.clamp(0.0, 1.0);
        self.combo.last = Some(Instant::now());
        if let Err(error) = self.ensure_combo_resources() {
            // Combining is a nicety: fall back to the second transition alone.
            tracing::warn!("skwd-wall-vk: combined transition unavailable: {error:#}");
            return self.render_part_plain(rt, export, src_a, uv_a, src_b, uv_b, t, combo.second);
        }
        if combo.mode == Mode::Then {
            return self.render_then(rt, export, src_a, uv_a, src_b, uv_b, t, combo);
        }
        self.prepare_part(combo.first)?;
        self.prepare_part(combo.second)?;
        let tex_a = src_a.texture_uv(uv_a);
        let tex_b = src_b.texture_uv(uv_b);
        self.ensure_src_pipelines(&[src_a, src_b])?;
        self.wait_frame_complete()?;
        let (avf_a, avf_b) = (Self::resolve_src(src_a)?, Self::resolve_src(src_b)?);
        let mut waits = WaitSems::new();
        for avf in avf_a.iter().chain(&avf_b) {
            avf.push_wait_sems(&mut waits);
        }
        let (first, second) = self.combo_handles()?;
        self.begin_frame_cmd()?;
        if export.direct_render {
            self.acquire_direct_export(export);
        }
        self.acquire_imports(&[src_a, src_b]);
        self.bind_src(self.desc_set, src_a, &avf_a)?;
        self.bind_src(self.desc_set_b, src_b, &avf_b)?;
        let barriers: Vec<_> = avf_a.iter().chain(&avf_b).map(AvvkFrame::read_barrier).collect();
        if !barriers.is_empty() {
            self.sample_barrier(
                &barriers,
                vk::PipelineStageFlags::VERTEX_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER,
            );
        }
        let pair = [self.desc_set, self.desc_set_b];
        if combo.mode == Mode::Mask {
            let solid = [self.combo.sets[SET_BLACK], self.combo.sets[SET_WHITE]];
            let push = self.solid_push(t, combo.first);
            self.record_part(first, combo.first, solid, &push)?;
        } else {
            let push = self.part_push(src_a, tex_a, src_b, tex_b, t, combo.first);
            self.record_part(first, combo.first, pair, &push)?;
        }
        let push = self.part_push(src_a, tex_a, src_b, tex_b, t, combo.second);
        self.record_part(second, combo.second, pair, &push)?;
        self.begin_clear_pass(rt.framebuffer);
        let push = self.transition_push(
            src_a,
            tex_a,
            src_a,
            tex_a,
            t,
            combo.mode.shader_id(),
            combo.mode.split_id(),
        );
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                self.cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout_sand,
                0,
                &[self.combo.sets[SET_PAIR], self.desc_set],
                &[],
            );
        }
        self.push_transition(&push);
        unsafe {
            self.device.cmd_bind_pipeline(
                self.cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.combo.compose,
            );
            self.device.cmd_draw(self.cmd, 3, 1, 0, 0);
            self.device.cmd_end_render_pass(self.cmd);
        }
        self.release_imports(&[src_a, src_b]);
        if export.direct_render {
            self.release_direct_export(export);
        } else {
            self.copy_rt_to_export(rt, export);
        }
        self.end_and_submit(&waits)?;
        Self::note_imports(&[src_a, src_b]);
        export.ready = true;
        for avf in avf_a.into_iter().chain(avf_b) {
            avf.commit_sampled();
        }
        Ok(())
    }

    /// "A then B": the first half runs the first transition, its half-way
    /// frame is frozen once, the second half runs the second one from it.
    #[allow(clippy::too_many_arguments)]
    fn render_then(
        &mut self,
        rt: &RenderTarget,
        export: &mut ExportImage,
        src_a: &Src,
        uv_a: [f32; 4],
        src_b: &Src,
        uv_b: [f32; 4],
        t: f32,
        combo: Combo,
    ) -> Result<()> {
        if t < 0.5 {
            self.combo.held = false;
            let first = t / 0.5 * HAND_OVER;
            return self.render_part_plain(
                rt,
                export,
                src_a,
                uv_a,
                src_b,
                uv_b,
                first,
                combo.first,
            );
        }
        if !self.combo.held {
            self.freeze_first(src_a, uv_a, src_b, uv_b, combo.first)?;
            self.combo.held = true;
        }
        let held = self.combo.targets.as_ref().map(|targets| targets.first.view);
        let held = Src::Rgba(held.ok_or_else(|| anyhow!("combo targets missing"))?);
        let second = (t - 0.5) / 0.5;
        self.render_part_plain(rt, export, &held, IDENTITY_UV, src_b, uv_b, second, combo.second)
    }

    /// Draws the first transition at the hand-over point into `first`.
    fn freeze_first(
        &mut self,
        src_a: &Src,
        uv_a: [f32; 4],
        src_b: &Src,
        uv_b: [f32; 4],
        part: Part,
    ) -> Result<()> {
        self.prepare_part(part)?;
        let tex_a = src_a.texture_uv(uv_a);
        let tex_b = src_b.texture_uv(uv_b);
        self.ensure_src_pipelines(&[src_a, src_b])?;
        self.wait_frame_complete()?;
        let (avf_a, avf_b) = (Self::resolve_src(src_a)?, Self::resolve_src(src_b)?);
        let mut waits = WaitSems::new();
        for avf in avf_a.iter().chain(&avf_b) {
            avf.push_wait_sems(&mut waits);
        }
        let (first, _) = self.combo_handles()?;
        self.begin_frame_cmd()?;
        self.acquire_imports(&[src_a, src_b]);
        self.bind_src(self.desc_set, src_a, &avf_a)?;
        self.bind_src(self.desc_set_b, src_b, &avf_b)?;
        let barriers: Vec<_> = avf_a.iter().chain(&avf_b).map(AvvkFrame::read_barrier).collect();
        if !barriers.is_empty() {
            self.sample_barrier(
                &barriers,
                vk::PipelineStageFlags::VERTEX_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER,
            );
        }
        let push = self.part_push(src_a, tex_a, src_b, tex_b, HAND_OVER, part);
        self.record_part(first, part, [self.desc_set, self.desc_set_b], &push)?;
        self.release_imports(&[src_a, src_b]);
        self.end_and_submit(&waits)?;
        Self::note_imports(&[src_a, src_b]);
        for avf in avf_a.into_iter().chain(avf_b) {
            avf.commit_sampled();
        }
        self.wait_frame_complete()
    }

    /// Push block of one part, from the real wallpapers.
    fn part_push(
        &self,
        src_a: &Src,
        tex_a: [f32; 4],
        src_b: &Src,
        tex_b: [f32; 4],
        t: f32,
        part: Part,
    ) -> TransitionPush {
        let (style, pid_base) = part_ids(part, t);
        self.transition_push(src_a, tex_a, src_b, tex_b, t, style, pid_base)
    }

    /// Push block of one part drawn from black ("old") to white ("new"): its
    /// picture is the transition's mask.
    fn solid_push(&self, t: f32, part: Part) -> TransitionPush {
        let (style, pid_base) = part_ids(part, t);
        let solid = Src::Rgba(vk::ImageView::null());
        let mut push =
            self.transition_push(&solid, IDENTITY_UV, &solid, IDENTITY_UV, t, style, pid_base);
        push.fill = 0;
        push
    }

    /// Records one part into an offscreen target in the current command buffer.
    fn record_part(
        &mut self,
        target: TargetHandles,
        part: Part,
        sets: [vk::DescriptorSet; 2],
        push: &TransitionPush,
    ) -> Result<()> {
        let pipelines: Vec<(vk::Pipeline, u32)> = match part {
            Part::Effect(fx) => vec![(self.effect_pipeline(fx)?, 3)],
            Part::Fade => vec![(self.effect_pipeline(paper_shaders::plan::crossfade_effect())?, 3)],
            Part::Sand(style) => {
                let (_, count) = paper_shaders::sand_window(style, push.progress);
                vec![(self.pipeline_base, 3), (self.pipeline_sand, count as u32)]
            }
        };
        self.begin_target_pass(target);
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                self.cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout_sand,
                0,
                &sets,
                &[],
            );
        }
        self.push_transition(push);
        for (pipeline, vertices) in pipelines {
            unsafe {
                self.device.cmd_bind_pipeline(self.cmd, vk::PipelineBindPoint::GRAPHICS, pipeline);
                self.device.cmd_draw(self.cmd, vertices, 1, 0, 0);
            }
        }
        unsafe { self.device.cmd_end_render_pass(self.cmd) };
        Ok(())
    }

    fn combo_handles(&self) -> Result<(TargetHandles, TargetHandles)> {
        let targets =
            self.combo.targets.as_ref().ok_or_else(|| anyhow!("combo targets missing"))?;
        Ok((TargetHandles::of(&targets.first), TargetHandles::of(&targets.second)))
    }

    /// Frees the combined-transition textures once none was drawn for a while.
    pub(super) fn release_idle_combo(&mut self) -> Result<()> {
        if self.combo.targets.is_none()
            || self.combo.last.is_some_and(|at| at.elapsed() < IDLE_RELEASE)
        {
            return Ok(());
        }
        self.wait_frame_complete()?;
        let device = self.device.clone();
        self.combo.release_targets(&device);
        Ok(())
    }

    /// Creates the render pass, compose pipeline, descriptor sets and
    /// textures, or reuses them from the last combined transition.
    fn ensure_combo_resources(&mut self) -> Result<()> {
        let device = self.device.clone();
        if self.combo.targets.as_ref().is_some_and(|targets| targets.extent != self.extent) {
            self.wait_frame_complete()?;
            self.combo.release_targets(&device);
        }
        if self.combo.pass == vk::RenderPass::null() {
            self.combo.pass = crate::vk::pipelines::create_scene_render_pass(&device, self.format)?;
        }
        if self.combo.compose == vk::Pipeline::null() {
            self.ensure_transition_pipelines()?;
            self.combo.compose = self.build_combo_pipeline()?;
        }
        if self.combo.pool == vk::DescriptorPool::null() {
            let sizes = [vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 6,
            }];
            unsafe {
                self.combo.pool = device.create_descriptor_pool(
                    &vk::DescriptorPoolCreateInfo::default().max_sets(3).pool_sizes(&sizes),
                    None,
                )?;
                let layouts = [self.desc_layout; 3];
                let sets = device.allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfo::default()
                        .descriptor_pool(self.combo.pool)
                        .set_layouts(&layouts),
                )?;
                self.combo.sets = [sets[0], sets[1], sets[2]];
            }
        }
        if self.combo.targets.is_none() {
            let (extent, format, pass) = (self.extent, self.format, self.combo.pass);
            let targets = ComboTargets {
                first: self.create_sketch_target(extent, format, pass)?,
                second: self.create_sketch_target(extent, format, pass)?,
                black: self.create_sketch_target(SOLID, format, pass)?,
                white: self.create_sketch_target(SOLID, format, pass)?,
                extent,
            };
            let sets = self.combo.sets;
            self.write_view_set(sets[SET_PAIR], targets.first.view, targets.second.view);
            self.write_view_set(sets[SET_BLACK], targets.black.view, targets.black.view);
            self.write_view_set(sets[SET_WHITE], targets.white.view, targets.white.view);
            let (black, white) =
                (TargetHandles::of(&targets.black), TargetHandles::of(&targets.white));
            self.combo.targets = Some(targets);
            self.combo.held = false;
            self.clear_solid(black, [0.0, 0.0, 0.0, 1.0])?;
            self.clear_solid(white, [1.0, 1.0, 1.0, 1.0])?;
        }
        Ok(())
    }

    /// Fills a mask source with one colour (once, when it is created).
    fn clear_solid(&mut self, target: TargetHandles, colour: [f32; 4]) -> Result<()> {
        self.wait_frame_complete()?;
        self.begin_frame_cmd()?;
        let clear = [vk::ClearValue { color: vk::ClearColorValue { float32: colour } }];
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
            self.device.cmd_end_render_pass(self.cmd);
        }
        self.end_and_submit(&WaitSems::new())?;
        self.wait_frame_complete()
    }
}

/// `u_style` and `u_pid_base` of one part at progress `t`.
fn part_ids(part: Part, t: f32) -> (i32, i32) {
    match part {
        Part::Effect(fx) => (fx as i32, 0),
        Part::Fade => (paper_shaders::plan::crossfade_effect() as i32, 0),
        Part::Sand(style) => (style, paper_shaders::sand_window(style, t).0),
    }
}
