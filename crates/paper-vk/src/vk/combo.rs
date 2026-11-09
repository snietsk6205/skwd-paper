//! Custom build (M3): per-renderer state of combined transitions
//! (`paper_shaders::plan::Combo`). Like the sketch prelude it keeps a few
//! offscreen textures at the renderer's size: each of the two transitions
//! draws into its own one, a compose pass mixes them on screen. "A then B"
//! freezes the first transition's half-way frame into `first` once and runs
//! the second from it. The drawing code lives in `draw/shared/combo.rs`.

use ash::vk;
use std::time::Instant;

use super::SceneTarget;
use super::sketch::destroy_target;

/// Offscreen textures of combined transitions.
pub(crate) struct ComboTargets {
    /// The first transition's frame (or its black and white mask).
    pub(crate) first: SceneTarget,
    /// The second transition's frame.
    pub(crate) second: SceneTarget,
    /// Tiny solid sources for the mask pass: "old" is black, "new" is white.
    pub(crate) black: SceneTarget,
    pub(crate) white: SceneTarget,
    pub(crate) extent: vk::Extent2D,
}

#[derive(Default)]
pub(crate) struct ComboState {
    pub(crate) targets: Option<ComboTargets>,
    /// Render pass of the offscreen targets (renderer format).
    pub(crate) pass: vk::RenderPass,
    pub(crate) compose: vk::Pipeline,
    pub(crate) pool: vk::DescriptorPool,
    /// (first, second), (black, black), (white, white).
    pub(crate) sets: [vk::DescriptorSet; 3],
    /// "A then B": the hand-over frame is already in `first`.
    pub(crate) held: bool,
    /// When the last combined frame was drawn, to free the textures later.
    pub(crate) last: Option<Instant>,
}

impl ComboState {
    /// Frees the textures; the caller has waited for the GPU.
    pub(crate) fn release_targets(&mut self, device: &ash::Device) {
        if let Some(targets) = self.targets.take() {
            for target in [targets.first, targets.second, targets.black, targets.white] {
                destroy_target(device, target);
            }
        }
        self.held = false;
    }

    /// Frees everything; called when the renderer is dropped.
    pub(crate) fn destroy(&mut self, device: &ash::Device) {
        self.release_targets(device);
        unsafe {
            if self.compose != vk::Pipeline::null() {
                device.destroy_pipeline(self.compose, None);
            }
            if self.pass != vk::RenderPass::null() {
                device.destroy_render_pass(self.pass, None);
            }
            if self.pool != vk::DescriptorPool::null() {
                device.destroy_descriptor_pool(self.pool, None);
            }
        }
        *self = Self::default();
    }
}
