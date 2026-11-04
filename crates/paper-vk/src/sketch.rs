//! "Sketch before transition": reads the picker's settings from skwd-wall's
//! config file so every transition path in this renderer can prepend the
//! progressive sketch without changing how transitions are requested.
//!
//! The daemon already lengthens the transition by the sketch duration, so the
//! sketch's share of the timeline is `sketch / (sketch + transition)`.

use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

use serde_json::Value;

const RECHECK: Duration = Duration::from_millis(400);
const DEFAULT_SKETCH_MS: f64 = 6000.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Sketch {
    pub(crate) style: usize,
    pub(crate) split: f32,
}

impl Sketch {
    pub(crate) fn code(self) -> i32 {
        paper_shaders::sketch_code(self.style, self.split)
    }
}

struct Cache {
    checked: Option<Instant>,
    modified: Option<SystemTime>,
    value: Option<Sketch>,
}

static CACHE: Mutex<Cache> = Mutex::new(Cache { checked: None, modified: None, value: None });

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn config_path() -> PathBuf {
    if let Some(dir) = env("SKWD_WALL_V2_CONFIG") {
        return PathBuf::from(dir).join("config.json");
    }
    let base = env("XDG_CONFIG_HOME")
        .unwrap_or_else(|| format!("{}/.config", env("HOME").unwrap_or_else(|| "/".into())));
    PathBuf::from(base).join("skwd-wall-v2").join("config.json")
}

fn number(section: &Value, key: &str) -> Option<f64> {
    let value = section.get(key)?;
    value.as_f64().or_else(|| value.as_str()?.trim().parse().ok()).filter(|n: &f64| n.is_finite())
}

pub(crate) fn parse(text: &str) -> Option<Sketch> {
    let root: Value = serde_json::from_str(text).ok()?;
    let section = root.get("transition")?;
    if !section.get("sketchEnabled").and_then(Value::as_bool).unwrap_or(false) {
        return None;
    }
    if !section.get("enabled").and_then(Value::as_bool).unwrap_or(true) {
        return None;
    }
    let style = paper_shaders::sketch_style_index(
        section.get("sketchStyle").and_then(Value::as_str).unwrap_or("pencil"),
    );
    let sketch_ms = number(section, "sketchDurationMs")
        .unwrap_or(DEFAULT_SKETCH_MS)
        .clamp(1000.0, 20_000.0);
    let base_ms = number(section, "durationMs").unwrap_or(600.0).clamp(50.0, 10_000.0);
    Some(Sketch { style, split: (sketch_ms / (sketch_ms + base_ms)) as f32 })
}

/// Current sketch settings, re-read at most every 400 ms and only when the
/// config file changed. `None` when the feature is off or unreadable.
pub(crate) fn current() -> Option<Sketch> {
    let mut cache = CACHE.lock().unwrap_or_else(PoisonError::into_inner);
    let now = Instant::now();
    if cache.checked.is_some_and(|at| now.duration_since(at) < RECHECK) {
        return cache.value;
    }
    cache.checked = Some(now);
    let path = config_path();
    let modified = std::fs::metadata(&path).and_then(|meta| meta.modified()).ok();
    if modified.is_some() && modified == cache.modified {
        return cache.value;
    }
    cache.modified = modified;
    cache.value = std::fs::read_to_string(&path).ok().and_then(|text| parse(&text));
    cache.value
}

/// Inverse of `t * t * (3 - 2t)`, used to recover linear progress from the
/// eased mix value that fade transitions are rendered with.
pub(crate) fn unease(mix: f32) -> f32 {
    let mix = mix.clamp(0.0, 1.0);
    (0.5 - ((1.0 - 2.0 * mix).asin() / 3.0).sin()).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_or_missing_is_none() {
        assert_eq!(parse("{}"), None);
        assert_eq!(parse(r#"{"transition":{"sketchEnabled":false}}"#), None);
        assert_eq!(parse(r#"{"transition":{"sketchEnabled":true,"enabled":false}}"#), None);
        assert_eq!(parse("not json"), None);
    }

    #[test]
    fn split_follows_durations() {
        let sketch = parse(
            r#"{"transition":{"sketchEnabled":true,"sketchStyle":"chalk","sketchDurationMs":6000,"durationMs":2000}}"#,
        )
        .unwrap();
        assert_eq!(sketch.style, 2);
        assert!((sketch.split - 0.75).abs() < 1e-6);
    }

    #[test]
    fn unease_inverts_smoothstep() {
        for step in 0..=20 {
            let t = step as f32 / 20.0;
            let eased = t * t * (3.0 - 2.0 * t);
            assert!((unease(eased) - t).abs() < 1e-3, "t={t}");
        }
    }
}
