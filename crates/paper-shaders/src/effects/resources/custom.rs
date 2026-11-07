//! Custom build: extra wallpaper transitions.
//!
//! Same dialect as the upstream effects: GLSL 330 with `u_tex_old`,
//! `u_tex_new` and `u_progress` (0 → 1); `effect_frag_vk` turns them into
//! Vulkan shaders. Point-based ones list their start-point edits in
//! `ORIGIN_EFFECTS` (`effects/vulkan.rs`).

/// Water rings spread from a point and carry the new picture in.
pub(in crate::effects) const RIPPLE_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec2 uv = v_uv;
    vec2 d = (uv - vec2(0.5)) * vec2(1.7777, 1.0);
    float dist = length(d);
    float front = p * 1.45;
    float ring = front - dist;
    float amp = sin(p * 3.14159);
    float wave = sin(dist * 46.0 - p * 34.0) * exp(-abs(ring) * 5.0) * amp;
    vec2 dir = dist > 0.0001 ? d / dist : vec2(0.0);
    vec2 off = dir * wave * 0.022 / vec2(1.7777, 1.0);
    vec4 a = texture(u_tex_old, uv + off);
    vec4 b = texture(u_tex_new, uv - off * 0.6);
    float m = smoothstep(0.0, 0.2, ring);
    vec4 col = mix(a, b, m);
    col.rgb += vec3(0.07) * max(wave, 0.0);
    frag = vec4(clamp(col.rgb, 0.0, 1.0), 1.0);
}
";

/// Hexagon tiles turn over in a wave that starts at a point.
pub(in crate::effects) const HONEYCOMB_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
float hex_dist(vec2 q) {
    q = abs(q);
    return max(dot(q, vec2(0.5, 0.8660254)), q.x);
}
void main() {
    float p = u_progress;
    const vec2 origin = vec2(0.5);
    float cells = 9.0;
    vec2 q = v_uv * vec2(1.7777, 1.0) * cells;
    vec2 r = vec2(1.0, 1.7320508);
    vec2 h = r * 0.5;
    vec2 a = mod(q, r) - h;
    vec2 b = mod(q - h, r) - h;
    vec2 gv = dot(a, a) < dot(b, b) ? a : b;
    vec2 id = q - gv;
    vec2 centre = id / cells / vec2(1.7777, 1.0);
    float reach = length(max(origin, vec2(1.0) - origin) * vec2(1.7777, 1.0)) + 0.06;
    float delay = clamp(length((centre - origin) * vec2(1.7777, 1.0)) / reach, 0.0, 1.0);
    float t = clamp((p - delay * 0.55) / 0.45, 0.0, 1.0);
    t = t * t * (3.0 - 2.0 * t);
    float hd = hex_dist(gv);
    float edge = t * 0.53;
    float m = 1.0 - smoothstep(edge - 0.02, edge, hd);
    vec4 oldc = texture(u_tex_old, v_uv);
    vec4 newc = texture(u_tex_new, v_uv);
    oldc.rgb *= 1.0 - 0.3 * t;
    vec3 col = mix(oldc.rgb, newc.rgb, m);
    float glow = exp(-abs(hd - edge) * 70.0) * step(0.001, t) * (1.0 - t);
    col += vec3(0.9, 0.95, 1.0) * glow * 0.45;
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// The old picture burns away along glowing embers.
pub(in crate::effects) const BURN_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
float hash(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}
float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), f.x),
               mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), f.x), f.y);
}
float fbm(vec2 p) {
    float v = 0.0;
    float amp = 0.5;
    for (int i = 0; i < 5; i++) {
        v += amp * noise(p);
        p *= 2.03;
        amp *= 0.5;
    }
    return v;
}
void main() {
    float p = u_progress;
    vec2 uv = v_uv;
    float n = fbm(uv * vec2(1.7777, 1.0) * 3.2) * 0.85 + (1.0 - uv.y) * 0.15;
    float edge = p * 1.3 - 0.2;
    float d = n - edge;
    vec4 oldc = texture(u_tex_old, uv);
    vec4 newc = texture(u_tex_new, uv);
    float char_amount = 1.0 - smoothstep(0.0, 0.14, d);
    oldc.rgb *= 1.0 - 0.75 * char_amount;
    float shown = smoothstep(0.0, -0.015, d);
    vec3 col = mix(oldc.rgb, newc.rgb, shown);
    float glow = smoothstep(0.06, 0.0, abs(d + 0.008));
    vec3 fire = mix(vec3(1.0, 0.35, 0.05), vec3(1.0, 0.85, 0.5), smoothstep(0.03, 0.0, abs(d)));
    col += fire * glow * 1.4;
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// The old picture curls away like a page and shows the new one under it.
pub(in crate::effects) const PAGE_CURL_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
const float AR = 1.7777;
const float PI = 3.14159265;
bool on_page(vec2 q) {
    return q.x >= 0.0 && q.x <= AR && q.y >= 0.0 && q.y <= 1.0;
}
vec3 old_at(vec2 q) {
    return texture(u_tex_old, vec2(q.x / AR, q.y)).rgb;
}
void main() {
    float p = u_progress;
    vec2 n = normalize(vec2(1.0, 0.32));
    float radius = 0.13;
    vec2 q = vec2(v_uv.x * AR, v_uv.y);
    float s = dot(q, n);
    float s_max = dot(vec2(AR, 1.0), n);
    float e = p * p * (3.0 - 2.0 * p);
    float c = mix(s_max + 0.01, -PI * radius * 0.5 - 0.03, e);
    vec3 newc = texture(u_tex_new, v_uv).rgb;
    vec3 col = newc;
    bool done = false;
    // Flat part of the page that has already turned over (its back side).
    if (s <= c) {
        float u = 2.0 * c + PI * radius - s;
        vec2 page = q + n * (u - s);
        if (u >= c + PI * radius && on_page(page)) {
            vec3 back = old_at(page);
            float grey = dot(back, vec3(0.299, 0.587, 0.114));
            col = mix(back, vec3(grey), 0.55) * 0.55 + vec3(0.36);
            done = true;
        }
    }
    // The roll: its top shows the back of the page, its bottom the front.
    if (!done && s >= c && s <= c + radius) {
        float k = clamp((s - c) / radius, 0.0, 1.0);
        float ang = asin(k);
        float u_top = c + radius * (PI - ang);
        vec2 top = q + n * (u_top - s);
        if (on_page(top)) {
            vec3 back = old_at(top);
            float grey = dot(back, vec3(0.299, 0.587, 0.114));
            float light = 0.75 + 0.25 * cos(ang);
            col = (mix(back, vec3(grey), 0.55) * 0.55 + vec3(0.36)) * light;
            done = true;
        } else {
            float u_bottom = c + radius * ang;
            vec2 bottom = q + n * (u_bottom - s);
            if (on_page(bottom)) {
                col = old_at(bottom) * (1.0 - 0.45 * k * k);
                done = true;
            }
        }
    }
    // The page still lying flat.
    if (!done && s < c) {
        col = old_at(q);
        done = true;
    }
    // Shadow of the roll on the new picture.
    if (!done && s > c + radius) {
        col = newc * (1.0 - 0.45 * exp(-(s - c - radius) * 22.0) * (1.0 - smoothstep(0.8, 1.0, e)));
    }
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// Slanted blinds open one after another.
pub(in crate::effects) const BLINDS_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec2 q = vec2(v_uv.x * 1.7777, v_uv.y);
    vec2 across = normalize(vec2(1.0, -0.45));
    float w = dot(q, across);
    float w_min = dot(vec2(0.0, 1.0), across);
    float w_max = dot(vec2(1.7777, 0.0), across);
    float count = 14.0;
    float pos = (w - w_min) / (w_max - w_min) * count;
    float index = floor(pos);
    float f = fract(pos);
    float delay = index / count;
    float t = clamp((p - delay * 0.5) / 0.5, 0.0, 1.0);
    t = t * t * (3.0 - 2.0 * t);
    float half_open = abs(f - 0.5) * 2.0;
    float m = smoothstep(half_open - 0.04, half_open, t * 1.08 - 0.04);
    vec4 oldc = texture(u_tex_old, v_uv);
    vec4 newc = texture(u_tex_new, v_uv);
    float shade = 1.0 - 0.35 * t * (1.0 - m);
    vec3 col = mix(oldc.rgb * shade, newc.rgb, m);
    frag = vec4(col, 1.0);
}
";

/// The new picture slides in over the old one, which falls back into shadow.
pub(in crate::effects) const SLIDE_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    float e = p < 0.5 ? 4.0 * p * p * p : 1.0 - pow(-2.0 * p + 2.0, 3.0) / 2.0;
    float edge = 1.0 - e;
    vec2 uv = v_uv;
    vec3 col;
    if (uv.x > edge) {
        vec2 b = vec2(uv.x - edge, uv.y);
        col = texture(u_tex_new, b).rgb;
    } else {
        float zoom = 1.0 - 0.06 * e;
        vec2 a = (vec2(uv.x + 0.3 * e, uv.y) - vec2(0.5)) * zoom + vec2(0.5);
        col = texture(u_tex_old, a).rgb * (1.0 - 0.55 * e);
        col *= 1.0 - 0.6 * exp(-(edge - uv.x) * 28.0) * step(0.0001, e);
    }
    frag = vec4(col, 1.0);
}
";

/// The old picture swirls into a point and the new one swirls out of it.
pub(in crate::effects) const VORTEX_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec2 c = v_uv - vec2(0.5);
    vec2 d = c * vec2(1.7777, 1.0);
    float r = length(d);
    float e = p * p * (3.0 - 2.0 * p);
    float twist = (1.0 - smoothstep(0.0, 1.1, r)) * 8.0;
    float ang_a = twist * e;
    float ang_b = -twist * (1.0 - e);
    vec2 da = vec2(d.x * cos(ang_a) - d.y * sin(ang_a), d.x * sin(ang_a) + d.y * cos(ang_a));
    vec2 db = vec2(d.x * cos(ang_b) - d.y * sin(ang_b), d.x * sin(ang_b) + d.y * cos(ang_b));
    vec4 a = texture(u_tex_old, v_uv + (da - d) / vec2(1.7777, 1.0));
    vec4 b = texture(u_tex_new, v_uv + (db - d) / vec2(1.7777, 1.0));
    float m = smoothstep(0.35, 0.65, p);
    frag = vec4(mix(a.rgb, b.rgb, m), 1.0);
}
";
