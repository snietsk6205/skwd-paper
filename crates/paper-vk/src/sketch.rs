//! Settings for "sketch before transition" and for the transition start point.
//!
//! The daemon passes them in environment variables when it starts a renderer,
//! the same way it passes the sand settings, so the renderer never reads
//! skwd-wall's config file itself:
//!
//! - `SKWD_PAPER_SKETCH=style=pencil;ms=6000;base=600;videos=1;scenes=1`
//!   (unset or empty: no sketch). `ms` is the sketch time and `base` the
//!   configured transition time; the daemon already lengthened the
//!   transition by `ms`, so the sketch gets `ms / total` of the timeline.
//! - `SKWD_PAPER_TRANSITION_ORIGIN=default|centre|random|dense`.

use std::sync::OnceLock;

const MAX_TRANSITION_MS: f64 = 10_000.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SketchMedia {
    /// A still image (the default when a caller does not say).
    #[default]
    Image,
    Video,
    Scene,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Sketch {
    pub(crate) style: usize,
    /// Share of the transition timeline spent drawing (0..0.95).
    pub(crate) split: f32,
    pub(crate) videos: bool,
    pub(crate) scenes: bool,
    /// Fixed stroke layout (tests); normally every transition draws differently.
    pub(crate) seed: Option<i32>,
}

impl Sketch {
    pub(crate) fn allows(self, media: SketchMedia) -> bool {
        match media {
            SketchMedia::Image => true,
            SketchMedia::Video => self.videos,
            SketchMedia::Scene => self.scenes,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Origin {
    /// Every transition uses its own built-in point.
    #[default]
    Default,
    Centre,
    Random,
    /// Where the new wallpaper has the most lines.
    Dense,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Settings {
    pub(crate) sketch: Option<Sketch>,
    pub(crate) origin: Origin,
}

fn flag(value: &str) -> Option<bool> {
    match value.trim() {
        "1" | "true" | "on" | "yes" => Some(true),
        "0" | "false" | "off" | "no" => Some(false),
        _ => None,
    }
}

pub(crate) fn parse_sketch(text: &str) -> Option<Sketch> {
    let mut style = 0;
    let mut sketch_ms = 6000.0;
    let mut base_ms = 600.0;
    let mut videos = true;
    let mut scenes = true;
    let mut seed = None;
    let mut any = false;
    for part in text.split(';').map(str::trim).filter(|part| !part.is_empty()) {
        let (key, value) = part.split_once('=')?;
        let value = value.trim();
        match key.trim() {
            "style" => style = paper_shaders::sketch_style_index(value),
            "ms" => sketch_ms = value.parse::<f64>().ok().filter(|n| n.is_finite())?,
            "base" => base_ms = value.parse::<f64>().ok().filter(|n| n.is_finite())?,
            "videos" => videos = flag(value)?,
            "scenes" => scenes = flag(value)?,
            "seed" => seed = Some(value.parse::<i32>().ok()? & 0x3ff),
            _ => continue,
        }
        any = true;
    }
    if !any {
        return None;
    }
    let sketch_ms = sketch_ms.clamp(1000.0, 20_000.0);
    let base_ms = base_ms.clamp(50.0, MAX_TRANSITION_MS);
    if base_ms >= MAX_TRANSITION_MS {
        return None;
    }
    let total = (base_ms + sketch_ms).min(MAX_TRANSITION_MS);
    let split = ((total - base_ms) / total).clamp(0.0, 0.95) as f32;
    (split > 0.0).then_some(Sketch { style, split, videos, scenes, seed })
}

pub(crate) fn parse_origin(text: &str) -> Origin {
    match text.trim() {
        "centre" | "center" => Origin::Centre,
        "random" => Origin::Random,
        "dense" | "busiest" => Origin::Dense,
        _ => Origin::Default,
    }
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

/// Settings of this renderer process, read once from its environment.
pub(crate) fn settings() -> Settings {
    static SETTINGS: OnceLock<Settings> = OnceLock::new();
    *SETTINGS.get_or_init(|| {
        let settings = Settings {
            sketch: parse_sketch(&env("SKWD_PAPER_SKETCH")),
            origin: parse_origin(&env("SKWD_PAPER_TRANSITION_ORIGIN")),
        };
        if settings.sketch.is_some() || settings.origin != Origin::Default {
            tracing::info!(?settings, "skwd-wall-vk: transition sketch settings");
        }
        settings
    })
}

/// Inverse of `t * t * (3 - 2t)`, used to recover linear progress from the
/// eased mix value that fade transitions are rendered with.
pub(crate) fn unease(mix: f32) -> f32 {
    let mix = mix.clamp(0.0, 1.0);
    (0.5 - ((1.0 - 2.0 * mix).asin() / 3.0).sin()).clamp(0.0, 1.0)
}

pub(crate) fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A pseudo-random value that differs for every call (time and counter mixed).
/// Custom build (M3): what a `--shader` value asks for (one transition or two
/// combined); "random" and "random-mix" roll here.
pub(crate) fn transition_plan(name: Option<&str>) -> paper_shaders::plan::Plan {
    paper_shaders::plan::plan(name.unwrap_or("fade"), &mut random_u32)
}

pub(crate) fn random_u32() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0x9e37_79b9);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    let mut x = (nanos ^ (nanos >> 29)) as u32 ^ COUNTER.fetch_add(0x6d2b_79f5, Ordering::Relaxed);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

/// A random start point that keeps clear of the screen edges.
pub(crate) fn random_origin() -> [f32; 2] {
    let value = random_u32();
    let x = f32::from((value & 0xffff) as u16) / 65535.0;
    let y = f32::from((value >> 16) as u16) / 65535.0;
    [0.2 + 0.6 * x, 0.2 + 0.6 * y]
}

/// Finds the busiest area of a low-resolution line-density map (one byte per
/// cell, row-major, top row first). The map is blurred so a large detailed
/// area wins over one sharp spot, with a mild pull towards the middle.
/// Returns screen UV (y down), or `None` when the picture has almost no lines.
pub(crate) fn densest(density: &[u8], width: usize, height: usize) -> Option<[f32; 2]> {
    if width < 3 || height < 3 || density.len() < width * height {
        return None;
    }
    let values: Vec<f32> =
        density[..width * height].iter().map(|&v| f32::from(v) / 255.0).collect();
    let radius = (width.max(height) / 10).max(1) as isize;
    let blur = |source: &[f32], horizontal: bool| -> Vec<f32> {
        let mut out = vec![0.0; source.len()];
        for y in 0..height as isize {
            for x in 0..width as isize {
                let mut sum = 0.0;
                let mut count = 0.0;
                for offset in -radius..=radius {
                    let (sx, sy) = if horizontal { (x + offset, y) } else { (x, y + offset) };
                    if sx < 0 || sy < 0 || sx >= width as isize || sy >= height as isize {
                        continue;
                    }
                    sum += source[sy as usize * width + sx as usize];
                    count += 1.0;
                }
                out[y as usize * width + x as usize] = sum / count;
            }
        }
        out
    };
    let blurred = blur(&blur(&values, true), false);
    let total: f32 = values.iter().sum();
    if total / values.len() as f32 <= 0.004 {
        return None;
    }
    let margin_x = width / 8;
    let margin_y = height / 8;
    let mut best = (f32::MIN, width / 2, height / 2);
    for y in margin_y..height - margin_y {
        for x in margin_x..width - margin_x {
            let dx = (x as f32 + 0.5) / width as f32 - 0.5;
            let dy = (y as f32 + 0.5) / height as f32 - 0.5;
            let score = blurred[y * width + x] * (1.0 - 0.6 * (dx * dx + dy * dy));
            if score > best.0 {
                best = (score, x, y);
            }
        }
    }
    Some([(best.1 as f32 + 0.5) / width as f32, (best.2 as f32 + 0.5) / height as f32])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_or_bad_settings_mean_no_sketch() {
        assert_eq!(parse_sketch(""), None);
        assert_eq!(parse_sketch("garbage"), None);
        assert_eq!(parse_sketch("ms=abc"), None);
        assert_eq!(parse_sketch("style=pencil;base=10000"), None);
    }

    #[test]
    fn split_follows_durations_and_cap() {
        let sketch = parse_sketch("style=chalk;ms=6000;base=2000;videos=0;scenes=1").unwrap();
        assert_eq!(sketch.style, 2);
        assert!((sketch.split - 0.75).abs() < 1e-6);
        assert!(!sketch.allows(SketchMedia::Video));
        assert!(sketch.allows(SketchMedia::Scene) && sketch.allows(SketchMedia::Image));
        // 6000 + 6000 is capped at 10000: the transition keeps 6000, the sketch 4000.
        let capped = parse_sketch("ms=6000;base=6000").unwrap();
        assert!((capped.split - 0.4).abs() < 1e-6);
        let defaults = parse_sketch("style=ink-brush").unwrap();
        assert_eq!(defaults.style, 4);
        assert!(defaults.videos && defaults.scenes);
        assert_eq!(defaults.seed, None);
        assert_eq!(parse_sketch("seed=5000").unwrap().seed, Some(5000 & 0x3ff));
    }

    #[test]
    fn origin_names() {
        assert_eq!(parse_origin("dense"), Origin::Dense);
        assert_eq!(parse_origin("center"), Origin::Centre);
        assert_eq!(parse_origin("random"), Origin::Random);
        assert_eq!(parse_origin(""), Origin::Default);
        assert_eq!(parse_origin("whatever"), Origin::Default);
    }

    #[test]
    fn unease_inverts_smoothstep() {
        for step in 0..=20 {
            let t = step as f32 / 20.0;
            assert!((unease(ease(t)) - t).abs() < 1e-3, "t={t}");
        }
    }

    #[test]
    fn densest_finds_the_busy_corner() {
        let (w, h) = (64, 36);
        let mut map = vec![0u8; w * h];
        for y in 6..14 {
            for x in 44..56 {
                map[y * w + x] = 255;
            }
        }
        let [x, y] = densest(&map, w, h).unwrap();
        assert!((0.68..0.88).contains(&x), "x={x}");
        assert!((0.15..0.4).contains(&y), "y={y}");
        assert_eq!(densest(&vec![0u8; w * h], w, h), None);
        assert_eq!(densest(&[1, 2], 1, 2), None);
    }

    #[test]
    fn random_origins_stay_inside() {
        for _ in 0..200 {
            let [x, y] = random_origin();
            assert!((0.2..=0.8).contains(&x) && (0.2..=0.8).contains(&y));
        }
        assert_ne!(random_u32(), random_u32());
    }
}
