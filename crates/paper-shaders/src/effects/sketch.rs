//! "Sketch before transition" shaders (Vulkan GLSL 450).
//!
//! The renderer freezes the first frame of the incoming wallpaper, runs the
//! analysis pass twice (once per cached texture), optionally renders the
//! density map to pick the start point, and then draws every sketch frame
//! with the cheap compose pass. When the drawing is finished, the chosen
//! transition (sand, fade or any effect) runs from the finished sketch.
//!
//! All passes use the transition pipeline layout and its 80-byte push block:
//! `u_style` is the sketch style, `u_pid_base` the analysis mode,
//! `u_origin`/`u_origin_on` the start point and `u_seed` the stroke layout.

const COMMON: &str = include_str!("sketch/common.glsl");
const ANALYSIS: &str = include_str!("sketch/analysis.glsl");
const COMPOSE: &str = include_str!("sketch/compose.glsl");
const DENSITY: &str = include_str!("sketch/density.glsl");

pub const SKETCH_STYLES: [&str; 7] =
    ["pencil", "digital", "chalk", "blueprint", "ink-brush", "charcoal", "notebook"];

/// Width and height of the density map used for the "busiest area" start point.
pub const SKETCH_DENSITY_SIZE: (u32, u32) = (64, 36);

pub fn sketch_style_index(name: &str) -> usize {
    SKETCH_STYLES.iter().position(|style| *style == name).unwrap_or(0)
}

/// Analysis pass: `u_pid_base` 0 writes analysis A, 1 writes analysis B.
pub fn sketch_analysis_frag_vk() -> String {
    format!("{COMMON}{ANALYSIS}")
}

/// Per-frame compose pass; its alpha is the paper's coverage.
pub fn sketch_compose_frag_vk() -> String {
    format!("{COMMON}{COMPOSE}")
}

/// Low-resolution line-density pass.
pub fn sketch_density_frag_vk() -> String {
    format!("{COMMON}{DENSITY}")
}
