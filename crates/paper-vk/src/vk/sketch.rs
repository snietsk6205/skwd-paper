//! Per-renderer state of the "sketch before transition" prelude: the cached
//! textures of the current transition and the pipelines that draw them.
//! The drawing code lives in `draw/shared/sketch.rs`.

use ash::vk;
use std::time::Instant;

use super::SceneTarget;
use crate::sketch::SketchMedia;

/// Offscreen textures of one transition, all at the renderer's size.
pub(crate) struct SketchCache {
    /// The incoming wallpaper's first frame (videos stop moving here).
    pub(crate) frozen: SceneTarget,
    /// (outline, detail, pressure, glow).
    pub(crate) analysis_a: SceneTarget,
    /// (outline time, detail time, shading time, shade / 2).
    pub(crate) analysis_b: SceneTarget,
    /// The completed drawing; the chosen transition starts from it.
    pub(crate) finished: SceneTarget,
    pub(crate) extent: vk::Extent2D,
}

#[derive(Default)]
pub(crate) struct SketchPipes {
    pub(crate) analysis: vk::Pipeline,
    pub(crate) compose: vk::Pipeline,
    pub(crate) density: vk::Pipeline,
    /// Render pass of the full-size targets (renderer format).
    pub(crate) pass: vk::RenderPass,
    /// Render pass and target of the small RGBA density map.
    pub(crate) density_pass: vk::RenderPass,
}

/// What the transition functions should draw for this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SketchStep {
    /// Draw the transition as usual, at this (possibly remapped) progress.
    Plain(f32),
    /// Draw the sketch at this drawing progress (0..1).
    Drawing(f32),
    /// Draw the chosen transition from the finished sketch at this progress.
    FromSketch(f32),
    /// The transition is visually complete: show the new wallpaper.
    Done,
}

#[derive(Default)]
pub(crate) struct SketchState {
    /// What the current transition is heading to; callers that know set it.
    pub(crate) media: SketchMedia,
    /// Progress and time of the last transition frame, to notice new ones.
    pub(crate) last: Option<(f32, Instant)>,
    pub(crate) cache: Option<SketchCache>,
    pub(crate) density: Option<SceneTarget>,
    pub(crate) pipes: SketchPipes,
    pub(crate) pool: vk::DescriptorPool,
    /// (frozen, frozen), (analysis A, analysis A), (analysis A, analysis B).
    pub(crate) sets: [vk::DescriptorSet; 3],
    /// Start point of the current transition in screen UV (y down).
    pub(crate) origin: Option<[f32; 2]>,
    pub(crate) seed: i32,
    /// Whether this transition draws a sketch first.
    pub(crate) drawing: bool,
    pub(crate) finished_ready: bool,
}

pub(crate) fn destroy_target(device: &ash::Device, target: SceneTarget) {
    unsafe {
        device.destroy_framebuffer(target.framebuffer, None);
        device.destroy_image_view(target.view, None);
        device.destroy_image(target.image, None);
        device.free_memory(target.memory, None);
    }
}

impl SketchState {
    /// Frees the cached textures; the caller has waited for the GPU.
    pub(crate) fn release_cache(&mut self, device: &ash::Device) {
        if let Some(cache) = self.cache.take() {
            for target in [cache.frozen, cache.analysis_a, cache.analysis_b, cache.finished] {
                destroy_target(device, target);
            }
        }
        if let Some(density) = self.density.take() {
            destroy_target(device, density);
        }
        self.finished_ready = false;
    }

    /// Frees everything; called when the renderer is dropped.
    pub(crate) fn destroy(&mut self, device: &ash::Device) {
        self.release_cache(device);
        unsafe {
            for pipe in [self.pipes.analysis, self.pipes.compose, self.pipes.density] {
                if pipe != vk::Pipeline::null() {
                    device.destroy_pipeline(pipe, None);
                }
            }
            for pass in [self.pipes.pass, self.pipes.density_pass] {
                if pass != vk::RenderPass::null() {
                    device.destroy_render_pass(pass, None);
                }
            }
            if self.pool != vk::DescriptorPool::null() {
                device.destroy_descriptor_pool(self.pool, None);
            }
        }
        self.pipes = SketchPipes::default();
        self.pool = vk::DescriptorPool::null();
        self.sets = [vk::DescriptorSet::null(); 3];
    }
}
