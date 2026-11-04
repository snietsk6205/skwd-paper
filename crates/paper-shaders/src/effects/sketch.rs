//! "Sketch first" prelude for wallpaper transitions.
//!
//! When enabled, a transition first draws the incoming wallpaper as a sketch
//! (outlines, then details, then shading), and the chosen transition then runs
//! from that finished sketch to the real wallpaper. Every effect gets a
//! generated variant whose `u_tex_old` samples are replaced by the finished
//! sketch, so any transition can follow the drawing.
//!
//! The renderer passes the style and the sketch share of the timeline in the
//! `u_pid_base` push constant: `1 + style * 10000 + split_permille`.

pub const SKETCH_LIB: &str = include_str!("sketch/lib.glsl");
pub const SKETCH_REVEAL_FRAG: &str = include_str!("sketch/reveal.frag");

pub const SKETCH_STYLES: [&str; 7] =
    ["pencil", "digital", "chalk", "blueprint", "ink-brush", "charcoal", "notebook"];

const SKETCH_HEADER: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
uniform vec2 u_res;
uniform int u_pid_base;
";

const SKETCH_DISPATCH: &str = "void main() {
    float sk_p = clamp(u_progress, 0.0, 1.0);
    sk_decode();
    if (sk_p < sk_split) {
        frag = vec4(sk_frame(v_uv, sk_p / sk_split), 1.0);
        return;
    }
    fx_p = sk_split >= 0.999 ? 1.0 : clamp((sk_p - sk_split) / (1.0 - sk_split), 0.0, 1.0);
    fx_main();
}
";

const STD_UNIFORMS: [&str; 5] = [
    "uniform sampler2D u_tex_old;",
    "uniform sampler2D u_tex_new;",
    "uniform float u_progress;",
    "uniform vec2 u_res;",
    "uniform int u_pid_base;",
];

pub fn sketch_style_index(name: &str) -> usize {
    SKETCH_STYLES.iter().position(|style| *style == name).unwrap_or(0)
}

fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn replace_word(text: &str, word: &str, replacement: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if text[i..].starts_with(word) {
            let end = i + word.len();
            let before_ok = i == 0 || !is_ident(bytes[i - 1]);
            let after_ok = end >= bytes.len() || !is_ident(bytes[end]);
            if before_ok && after_ok {
                out.push_str(replacement);
                i = end;
                continue;
            }
        }
        let Some(ch) = text[i..].chars().next() else { break };
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Builds the GL 3.3 source of `effect_gl` preceded by the sketch prelude.
pub fn sketch_variant(effect_gl: &str) -> String {
    let mut uniforms = String::new();
    let mut body = String::new();
    for line in effect_gl.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#version")
            || trimmed == "in vec2 v_uv;"
            || trimmed == "out vec4 frag;"
            || STD_UNIFORMS.contains(&trimmed)
        {
            continue;
        }
        if trimmed.starts_with("uniform ") {
            uniforms.push_str(line);
            uniforms.push('\n');
            continue;
        }
        body.push_str(line);
        body.push('\n');
    }
    let body = replace_word(&body, "u_progress", "fx_p")
        .replace("void main()", "void fx_main()")
        .replace("texture(u_tex_old,", "sk_final(");
    format!("{SKETCH_HEADER}{uniforms}{SKETCH_LIB}float fx_p;\n{body}{SKETCH_DISPATCH}")
}

/// Sketch variants in pipeline order: one per entry of `EFFECTS`, then the
/// ink-wash reveal used in place of fades and sand.
pub fn sketch_variant_sources() -> Vec<String> {
    super::EFFECTS
        .iter()
        .map(|(_, src)| sketch_variant(src))
        .chain(std::iter::once(sketch_variant(SKETCH_REVEAL_FRAG)))
        .collect()
}

/// Pipeline index of the sketch variant of effect `fx`.
pub const fn sketch_effect_index(fx: usize) -> usize {
    super::EFFECTS.len() + fx
}

/// Pipeline index of the ink-wash reveal variant.
pub const SKETCH_REVEAL_INDEX: usize = super::EFFECTS.len() * 2;

/// Push-constant code for a sketch prelude, see the module docs.
pub fn sketch_code(style: usize, split: f32) -> i32 {
    let style = style.min(SKETCH_STYLES.len() - 1) as i32;
    let permille = (split.clamp(0.0, 0.95) * 1000.0).round() as i32;
    1 + style * 10_000 + permille
}
