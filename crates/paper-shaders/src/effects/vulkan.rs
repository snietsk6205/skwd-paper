const VK_EFFECT_HEADER: &str = "#version 450
layout(push_constant) uniform PC {
    vec2 u_res;
    float u_progress;
    int u_style;
    int u_pid_base;
    int u_fill;
    int u_rgba_a;
    int u_rgba_b;
    vec4 u_uv_a;
    vec4 u_uv_b;
    vec2 u_origin;
    int u_origin_on;
    int u_seed;
};
layout(set = 0, binding = 0) uniform sampler2D luma_a;
layout(set = 0, binding = 1) uniform sampler2D chroma_a;
layout(set = 1, binding = 0) uniform sampler2D luma_b;
layout(set = 1, binding = 1) uniform sampler2D chroma_b;
layout(location = 0) in vec2 v_uv;
layout(location = 0) out vec4 frag;
vec3 nv12_fx(float y, vec2 c) {
    float yf = (y - 16.0 / 255.0) * (255.0 / 219.0);
    float u = c.x - 0.5;
    float v = c.y - 0.5;
    return clamp(vec3(yf + 1.5748 * v, yf - 0.1873 * u - 0.4681 * v, yf + 1.8556 * u), 0.0, 1.0);
}
bool fill_oob(vec2 uv) {
    return u_fill == 1 && (any(lessThan(uv, vec2(0.0))) || any(greaterThan(uv, vec2(1.0))));
}
vec2 fill_wrap(vec2 uv) {
    return u_fill == 2 ? fract(uv) : uv;
}
vec4 texA(vec2 p) {
    vec2 uv = clamp(p, vec2(0.0), vec2(1.0)) * u_uv_a.xy + u_uv_a.zw;
    if (fill_oob(uv)) return vec4(0.0, 0.0, 0.0, 1.0);
    uv = fill_wrap(uv);
    if (u_rgba_a == 1) return texture(luma_a, uv);
    return vec4(nv12_fx(texture(luma_a, uv).r, texture(chroma_a, uv).rg), 1.0);
}
vec4 texB(vec2 p) {
    vec2 uv = clamp(p, vec2(0.0), vec2(1.0)) * u_uv_b.xy + u_uv_b.zw;
    if (fill_oob(uv)) return vec4(0.0, 0.0, 0.0, 1.0);
    uv = fill_wrap(uv);
    if (u_rgba_b == 1) return texture(luma_b, uv);
    return vec4(nv12_fx(texture(luma_b, uv).r, texture(chroma_b, uv).rg), 1.0);
}
float vk_hash1(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}
// Start point chosen by the renderer (screen UV, y down), or the effect's own.
vec2 fx_origin(vec2 fallback) {
    return u_origin_on != 0 ? u_origin : fallback;
}
float fx_reach(vec2 o) {
    vec2 far = max(o, vec2(1.0) - o) * vec2(1.7777, 1.0);
    return length(far);
}
// Grows a reveal radius so it still covers the far corner from a moved origin.
float fx_scale(vec2 fallback) {
    return u_origin_on != 0 ? max(1.0, fx_reach(u_origin) / fx_reach(fallback)) : 1.0;
}
vec2 vk_hash2(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * vec3(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}
";

fn desin_hashes(body: &str) -> String {
    let two = regex_lite::Regex::new(
        r"fract\(sin\(vec2\(dot\((\w[\w.]*),\s*vec2\([\d.]+,\s*[\d.]+\)\),\s*dot\(\w[\w.]*,\s*vec2\([\d.]+,\s*[\d.]+\)\)\)\)\s*\*\s*[\d.]+\)",
    )
    .unwrap();
    let body = two.replace_all(body, "vk_hash2($1)");
    let one = regex_lite::Regex::new(
        r"fract\(sin\(dot\((\w+)(?:\.xy)?\s*,\s*vec2\([\d.]+\s*,\s*[\d.]+\)\)\)\s*\*\s*[\d.]+\)",
    )
    .unwrap();
    one.replace_all(&body, "vk_hash1($1)").into_owned()
}

/// Effects that grow from a point, and the source edits that let the
/// renderer move that point (`fx_origin`). Each edit must match exactly once
/// in that effect's source; the tests fail if upstream changes them.
pub const ORIGIN_EFFECTS: [(&str, &[(&str, &str)]); 11] = [
    (
        "iris",
        &[
            ("vec2 c = v_uv - vec2(0.5);", "vec2 c = v_uv - fx_origin(vec2(0.5));"),
            ("float r = u_progress * 1.2;", "float r = u_progress * 1.2 * fx_scale(vec2(0.5));"),
        ],
    ),
    (
        "ink-splash",
        &[
            ("vec2 c = v_uv - vec2(0.5);", "vec2 c = v_uv - fx_origin(vec2(0.5));"),
            (
                "float boundary = p * 1.7 - 0.15;",
                "float boundary = p * 1.7 * fx_scale(vec2(0.5)) - 0.15;",
            ),
        ],
    ),
    (
        "inkwell-drop",
        &[
            ("vec2 impact = vec2(0.35, 0.4);", "vec2 impact = fx_origin(vec2(0.35, 0.4));"),
            ("float front = p * 1.5;", "float front = p * 1.5 * fx_scale(vec2(0.35, 0.4));"),
        ],
    ),
    ("chromatic-bloom", &[("vec2 c = v_uv - vec2(0.5);", "vec2 c = v_uv - fx_origin(vec2(0.5));")]),
    ("smoke", &[("vec2 center = uv - 0.5;", "vec2 center = uv - fx_origin(vec2(0.5));")]),
    (
        "circle-crop",
        &[
            (
                "float dist = length((vec2(p) - 0.5) * ratio2);",
                "float dist = length((vec2(p) - fx_origin(vec2(0.5))) * ratio2);",
            ),
            ("step(s, dist)", "step(s * fx_scale(vec2(0.5)), dist)"),
        ],
    ),
    ("crosshatch", &[("const vec2 center = vec2(0.5);", "vec2 center = fx_origin(vec2(0.5));")]),
    (
        "polka-dots-curtain",
        &[("const vec2 center = vec2(0, 0);", "vec2 center = fx_origin(vec2(0.0));")],
    ),
    // Custom build.
    (
        "ripple",
        &[
            (
                "vec2 d = (uv - vec2(0.5)) * vec2(1.7777, 1.0);",
                "vec2 d = (uv - fx_origin(vec2(0.5))) * vec2(1.7777, 1.0);",
            ),
            ("float front = p * 1.45;", "float front = p * 1.45 * fx_scale(vec2(0.5));"),
        ],
    ),
    ("honeycomb", &[("const vec2 origin = vec2(0.5);", "vec2 origin = fx_origin(vec2(0.5));")]),
    ("vortex", &[("vec2 c = v_uv - vec2(0.5);", "vec2 c = v_uv - fx_origin(vec2(0.5));")]),
];

fn origin_edits(gl_src: &str) -> Option<&'static [(&'static str, &'static str)]> {
    ORIGIN_EFFECTS.iter().find_map(|(name, edits)| {
        let (_, src) = super::EFFECTS.iter().find(|(effect, _)| effect == name)?;
        (*src == gl_src).then_some(*edits)
    })
}

pub fn effect_frag_vk(gl_src: &str) -> String {
    let mut body = String::new();
    let mut skip_fn = false;
    for line in gl_src.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#version") || trimmed.starts_with("uniform ") {
            continue;
        }
        if trimmed == "in vec2 v_uv;" || trimmed == "out vec4 frag;" {
            continue;
        }
        if trimmed.starts_with("vec4 sample_thumb(") {
            skip_fn = true;
            continue;
        }
        if skip_fn {
            if trimmed == "}" {
                skip_fn = false;
            }
            continue;
        }
        body.push_str(line);
        body.push('\n');
    }
    let body = body
        .replace("texture(u_tex_old,", "texA(")
        .replace("texture(u_tex_new,", "texB(")
        .replace("sample_thumb(idx, mrp)", "texB(mrp)")
        .replace("u_thumb_count", "0");
    let mut body = desin_hashes(&body);
    for (from, to) in origin_edits(gl_src).unwrap_or_default() {
        body = body.replacen(from, to, 1);
    }
    format!("{VK_EFFECT_HEADER}{body}")
}
