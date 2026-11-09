//! Custom build (M3): the "Shapes" family. Tiles, dots and blades.
//!
//! Same dialect as the upstream effects (GLSL 330, `u_tex_old`, `u_tex_new`,
//! `u_progress` 0 → 1). Point-based ones list their start-point edits in
//! `ORIGIN_EFFECTS` (`effects/vulkan.rs`).

/// Diamond tiles pop open in a wave from a point, with a little overshoot.
pub(in crate::effects) const DIAMONDS_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    const vec2 origin = vec2(0.5);
    float cells = 10.0;
    vec2 q = v_uv * vec2(1.7777, 1.0) * cells;
    vec2 r = vec2(q.x + q.y, q.x - q.y) * 0.70710678;
    vec2 id = floor(r) + 0.5;
    vec2 f = fract(r) - 0.5;
    vec2 cq = vec2(id.x + id.y, id.x - id.y) * 0.70710678;
    vec2 centre = cq / cells / vec2(1.7777, 1.0);
    float reach = length(max(origin, vec2(1.0) - origin) * vec2(1.7777, 1.0)) + 0.1;
    float delay = clamp(length((centre - origin) * vec2(1.7777, 1.0)) / reach, 0.0, 1.0);
    float t = clamp((p - delay * 0.6) / 0.4, 0.0, 1.0);
    float grow = t * (1.0 + 0.3 * sin(t * 3.14159));
    float dd = max(abs(f.x), abs(f.y));
    float edge = grow * 0.54;
    float m = 1.0 - smoothstep(edge - 0.03, edge, dd);
    vec3 a = texture(u_tex_old, v_uv).rgb * (1.0 - 0.3 * t);
    vec3 b = texture(u_tex_new, v_uv).rgb;
    float rim = exp(-abs(dd - edge) * 55.0) * step(0.001, t) * (1.0 - t);
    vec3 col = mix(a, b, m) + vec3(1.0, 0.97, 0.9) * rim * 0.4;
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// Triangle tiles flip over one after another from left to right.
pub(in crate::effects) const TRIANGLES_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    const float H = 0.8660254;
    float cells = 7.0;
    vec2 q = vec2(v_uv.x * 1.7777, v_uv.y) * cells;
    float row = floor(q.y / H);
    float y = fract(q.y / H);
    float shift = mod(row, 2.0) * 0.5;
    float x = q.x + shift;
    float ci = floor(x);
    float fx = fract(x);
    bool up = abs(fx - 0.5) * 2.0 < y;
    vec2 cen;
    if (up) {
        cen = vec2(ci + 0.5, row + 2.0 / 3.0);
    } else {
        cen = vec2(fx < 0.5 ? ci : ci + 1.0, row + 1.0 / 3.0);
    }
    vec2 lp = vec2(x - cen.x, (q.y / H - cen.y) * H);
    if (!up) {
        lp.y = -lp.y;
    }
    float rin = H / 3.0;
    float dt = max(lp.y, max(dot(lp, vec2(0.8660254, -0.5)), dot(lp, vec2(-0.8660254, -0.5)))) / rin;
    vec2 cen_screen = vec2((cen.x - shift) / cells / 1.7777, cen.y * H / cells);
    float delay = clamp((cen_screen.x + cen_screen.y * 0.25) / 1.25, 0.0, 1.0);
    float t = clamp((p - delay * 0.55) / 0.45, 0.0, 1.0);
    float k = abs(cos(t * 3.14159));
    bool inside = dt < k * 1.06;
    vec3 pic = t < 0.5 ? texture(u_tex_old, v_uv).rgb : texture(u_tex_new, v_uv).rgb;
    float shade = 0.55 + 0.45 * k;
    vec3 gap = mix(texture(u_tex_old, v_uv).rgb, texture(u_tex_new, v_uv).rgb, t) * 0.12;
    vec3 col = inside ? pic * shade : gap;
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// A checkerboard: the dark squares open first, then the light ones.
pub(in crate::effects) const CHECKERBOARD_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
float hash(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}
void main() {
    float p = u_progress;
    vec2 q = vec2(v_uv.x * 1.7777, v_uv.y) * 8.0;
    vec2 id = floor(q);
    vec2 f = fract(q) - 0.5;
    float parity = mod(id.x + id.y, 2.0);
    float start = parity * 0.4 + hash(id) * 0.15;
    float t = clamp((p - start) / 0.45, 0.0, 1.0);
    t = t * t * (3.0 - 2.0 * t);
    float half_size = t * 0.58;
    float rr = 0.18 * (1.0 - t);
    vec2 box = abs(f) - vec2(half_size - rr);
    float sd = length(max(box, 0.0)) + min(max(box.x, box.y), 0.0) - rr;
    float m = smoothstep(0.02, -0.02, sd) * step(0.001, t);
    vec3 a = texture(u_tex_old, v_uv).rgb;
    vec3 b = texture(u_tex_new, v_uv).rgb;
    vec3 col = mix(a * (1.0 - 0.2 * p), b, m);
    frag = vec4(col, 1.0);
}
";

/// Vertical stripes slide up and down in turn and swap the pictures.
pub(in crate::effects) const STRIPES_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec2 uv = v_uv;
    float n = 12.0;
    float idx = floor(uv.x * n);
    float dir = mod(idx, 2.0) > 0.5 ? -1.0 : 1.0;
    float t = clamp((p - idx / n * 0.35) / 0.65, 0.0, 1.0);
    float e = t < 0.5 ? 4.0 * t * t * t : 1.0 - pow(-2.0 * t + 2.0, 3.0) / 2.0;
    float y_new = uv.y + dir * (1.0 - e);
    vec3 col;
    if (y_new >= 0.0 && y_new <= 1.0) {
        col = texture(u_tex_new, vec2(uv.x, y_new)).rgb;
    } else {
        col = texture(u_tex_old, vec2(uv.x, uv.y - dir * e)).rgb * (1.0 - 0.3 * e);
    }
    float seam = dir > 0.0 ? uv.y - e : uv.y - (1.0 - e);
    col *= 1.0 - 0.35 * exp(-abs(seam) * 45.0) * step(0.001, e) * step(e, 0.999);
    frag = vec4(col, 1.0);
}
";

/// The new picture is printed as halftone dots, darkest parts first, until
/// the dots merge.
pub(in crate::effects) const HALFTONE_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    float n = 34.0;
    vec2 q = vec2(v_uv.x * 1.7777, v_uv.y) * n;
    float row = floor(q.y);
    q.x += mod(row, 2.0) * 0.5;
    vec2 id = floor(q);
    vec2 f = fract(q) - 0.5;
    vec2 cuv = vec2((id.x + 0.5 - mod(row, 2.0) * 0.5) / n / 1.7777, (id.y + 0.5) / n);
    float l = dot(texture(u_tex_new, cuv).rgb, vec3(0.299, 0.587, 0.114));
    float delay = (1.0 - l) * 0.5;
    float t = clamp((p - delay) / 0.5, 0.0, 1.0);
    float r = t * 0.78;
    float m = smoothstep(r, r - 0.08, length(f)) * step(0.001, t);
    vec3 a = texture(u_tex_old, v_uv).rgb * (1.0 - 0.2 * p);
    vec3 b = texture(u_tex_new, v_uv).rgb;
    frag = vec4(mix(a, b, m), 1.0);
}
";

/// Square tiles turn up in a spiral from the middle of the screen.
pub(in crate::effects) const SPIRAL_TILES_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    float n = 9.0;
    vec2 q = vec2(v_uv.x * 1.7777, v_uv.y) * n;
    vec2 id = floor(q);
    vec2 f = fract(q) - 0.5;
    vec2 mid = floor(vec2(1.7777, 1.0) * n * 0.5);
    vec2 d = id - mid;
    float ring = max(abs(d.x), abs(d.y));
    float ang = atan(d.y, d.x) / 6.28318 + 0.5;
    float max_ring = ceil(1.7777 * n * 0.5);
    float order = (ring + ang) / (max_ring + 1.0);
    float t = clamp((p - order * 0.7) / 0.3, 0.0, 1.0);
    t = t * t * (3.0 - 2.0 * t);
    float rot = (1.0 - t) * 1.4;
    vec2 g = vec2(f.x * cos(rot) - f.y * sin(rot), f.x * sin(rot) + f.y * cos(rot));
    float half_size = t * 0.6;
    float sd = max(abs(g.x), abs(g.y)) - half_size;
    float m = smoothstep(0.02, -0.02, sd) * step(0.001, t);
    vec3 a = texture(u_tex_old, v_uv).rgb * (1.0 - 0.25 * p);
    vec3 b = texture(u_tex_new, v_uv).rgb;
    float rim = exp(-abs(sd) * 70.0) * step(0.001, t) * (1.0 - t);
    frag = vec4(clamp(mix(a, b, m) + vec3(rim * 0.35), 0.0, 1.0), 1.0);
}
";

/// Camera aperture blades close over the old picture and open on the new one.
pub(in crate::effects) const SHUTTER_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec2 c = v_uv - vec2(0.5);
    vec2 d = c * vec2(1.7777, 1.0);
    float closed = p < 0.5 ? smoothstep(0.0, 0.5, p) : 1.0 - smoothstep(0.5, 1.0, p);
    float reach = 1.25;
    float radius = mix(reach, 0.0, closed);
    float seg = 6.28318 / 6.0;
    float ang = atan(d.y, d.x) + closed * 1.3;
    float a2 = mod(ang, seg) - seg * 0.5;
    float poly = length(d) * cos(a2) / cos(seg * 0.5);
    float inside = smoothstep(radius + 0.004, radius - 0.004, poly);
    vec3 pic = p < 0.5 ? texture(u_tex_old, v_uv).rgb : texture(u_tex_new, v_uv).rgb;
    float blade = fract(ang / seg);
    vec3 metal = vec3(0.1, 0.1, 0.12) + vec3(0.1) * blade;
    metal *= 0.7 + 0.3 * smoothstep(0.0, 0.04, blade);
    metal += vec3(0.25) * exp(-abs(poly - radius) * 40.0);
    frag = vec4(clamp(mix(metal, pic, inside), 0.0, 1.0), 1.0);
}
";

/// The old picture folds into a turning kaleidoscope that unfolds into the
/// new one.
pub(in crate::effects) const KALEIDOSCOPE_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    float k = sin(p * 3.14159);
    float w = k * k;
    w = w * w * (3.0 - 2.0 * w);
    vec2 d = (v_uv - vec2(0.5)) * vec2(1.7777, 1.0);
    float r = length(d);
    float ang = atan(d.y, d.x) + (p - 0.5) * 2.4 * w;
    float seg = 6.28318 / 8.0;
    float fa = mod(ang, seg);
    fa = min(fa, seg - fa);
    vec2 kd = vec2(cos(fa), sin(fa)) * r * (1.0 - 0.25 * w);
    vec2 kuv = kd / vec2(1.7777, 1.0) + vec2(0.5);
    vec2 uv = mix(v_uv, kuv, w);
    vec3 a = texture(u_tex_old, uv).rgb;
    vec3 b = texture(u_tex_new, uv).rgb;
    vec3 col = mix(a, b, smoothstep(0.38, 0.62, p));
    col *= 1.0 + 0.15 * w * (0.5 + 0.5 * cos(fa * 30.0));
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";
