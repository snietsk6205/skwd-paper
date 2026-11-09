//! Custom build (M3): the "Light" family. Glows, flashes and beams.
//!
//! Same dialect as the upstream effects (GLSL 330, `u_tex_old`, `u_tex_new`,
//! `u_progress` 0 → 1). Every extra light fades out before the end so the
//! last frame is exactly the new wallpaper.

/// A warm film light leak sweeps across and leaves the new picture behind it.
pub(in crate::effects) const LIGHT_LEAK_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec2 uv = v_uv;
    vec3 a = texture(u_tex_old, uv).rgb;
    vec3 b = texture(u_tex_new, uv).rgb;
    float x = uv.x * 1.7777;
    float centre = mix(-0.7, 2.5, p);
    float wob = sin(uv.y * 5.0 + p * 6.0) * 0.12 + sin(uv.y * 13.0 - p * 4.0) * 0.05;
    float d = x - centre + wob;
    float m = smoothstep(0.35, -0.35, d);
    vec3 col = mix(a, b, m);
    float strength = sin(p * 3.14159) * (1.0 - smoothstep(0.9, 1.0, p));
    float glow = exp(-d * d * 1.4) * strength;
    float core = exp(-d * d * 10.0) * strength;
    vec3 leak = mix(vec3(1.0, 0.5, 0.12), vec3(1.0, 0.22, 0.42), uv.y);
    vec3 light = leak * glow * 0.9 + vec3(1.0, 0.93, 0.8) * core * 0.8;
    col = vec3(1.0) - (vec3(1.0) - col) * (vec3(1.0) - clamp(light, 0.0, 1.0));
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// A camera flash: everything goes white, the new picture appears with a
/// fading afterimage of the old one's bright parts.
pub(in crate::effects) const FLASHBULB_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec2 uv = v_uv;
    vec3 a = texture(u_tex_old, uv).rgb;
    vec3 b = texture(u_tex_new, uv).rgb;
    float flash = exp(-pow((p - 0.3) * 6.5, 2.0));
    float show = smoothstep(0.26, 0.36, p);
    vec3 col = mix(a, b, show);
    float la = dot(a, vec3(0.299, 0.587, 0.114));
    float decay = show * (1.0 - smoothstep(0.36, 0.95, p));
    col = col * (1.0 - vec3(0.32, 0.18, 0.04) * la * decay) + vec3(0.04, 0.1, 0.16) * la * decay;
    vec2 d = (uv - vec2(0.5)) * vec2(1.7777, 1.0);
    float spot = 1.0 - 0.35 * smoothstep(0.2, 1.2, length(d));
    col = mix(col * (1.0 + 0.5 * flash), vec3(1.0, 0.98, 0.94) * spot, flash * 0.92);
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// Curtains of northern lights drift down the screen; the new picture is
/// behind them.
pub(in crate::effects) const AURORA_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec2 uv = v_uv;
    vec3 a = texture(u_tex_old, uv).rgb;
    vec3 b = texture(u_tex_new, uv).rgb;
    float front = mix(-0.4, 1.4, p);
    float wave = sin(uv.x * 7.0 + p * 5.0) * 0.07 + sin(uv.x * 17.0 - p * 3.0) * 0.025;
    float d = uv.y - front + wave;
    float m = smoothstep(0.06, -0.06, d);
    vec3 col = mix(a * (1.0 - 0.35 * smoothstep(0.0, 0.3, p)), b, m);
    float rays = 0.55 + 0.45 * sin(uv.x * 70.0 + sin(uv.x * 9.0 + p * 4.0) * 3.0);
    float above = max(-d, 0.0);
    float band = exp(-above * 7.0) * smoothstep(0.03, -0.01, d);
    vec3 tint = mix(vec3(0.2, 1.0, 0.6), vec3(0.65, 0.3, 1.0), clamp(above * 4.0, 0.0, 1.0));
    float strength = pow(sin(p * 3.14159), 0.6) * (1.0 - smoothstep(0.88, 1.0, p));
    col += tint * band * rays * strength * 0.85;
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// The colours split apart like light through a prism, then meet again in
/// the new picture.
pub(in crate::effects) const PRISM_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
vec3 hue(float h) {
    return 0.5 + 0.5 * cos(6.28318 * (h + vec3(0.0, 0.33, 0.67)));
}
void main() {
    float p = u_progress;
    vec2 uv = v_uv;
    float k = sin(p * 3.14159);
    vec2 dir = vec2(1.0, 0.3) * k * 0.055;
    vec3 a = vec3(texture(u_tex_old, uv - dir).r, texture(u_tex_old, uv).g,
                  texture(u_tex_old, uv + dir).b);
    vec3 b = vec3(texture(u_tex_new, uv - dir).r, texture(u_tex_new, uv).g,
                  texture(u_tex_new, uv + dir).b);
    float m = smoothstep(0.35, 0.65, p);
    vec3 col = mix(a, b, m);
    float band = (uv.x * 1.7777 + uv.y) * 0.45 - p * 1.6;
    float sheen = exp(-pow(fract(band) - 0.5, 2.0) * 40.0);
    col += hue(band * 2.0) * sheen * 0.22 * k;
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// Beams of light burst out of a point and the new picture grows behind them.
pub(in crate::effects) const GOD_RAYS_FRAG: &str = "#version 330 core
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
    float ang = atan(d.y, d.x);
    float rays = 0.5 + 0.5 * sin(ang * 23.0 + sin(ang * 7.0 + p * 2.0) * 2.0);
    rays = rays * rays * rays;
    float k = sin(p * 3.14159) * (1.0 - smoothstep(0.7, 1.0, p));
    float reach = p * 1.25;
    float edge = reach + rays * 0.16 * k;
    float m = smoothstep(edge, edge - 0.08, r);
    vec3 a = texture(u_tex_old, v_uv).rgb;
    vec3 b = texture(u_tex_new, v_uv).rgb;
    vec3 col = mix(a * (1.0 - 0.25 * k), b, m);
    float beam = rays * exp(-max(r - reach, 0.0) * 2.5) * k;
    col += vec3(1.0, 0.9, 0.7) * beam * 0.55;
    col += vec3(1.0, 0.96, 0.86) * exp(-r * r * 9.0) * k * 0.55;
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// The outlines of the new picture light up like neon tubes, then it fills in.
pub(in crate::effects) const NEON_EDGES_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
float lum(vec3 c) {
    return dot(c, vec3(0.299, 0.587, 0.114));
}
float hash(vec2 p) {
    return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
}
void main() {
    float p = u_progress;
    vec2 uv = v_uv;
    vec3 a = texture(u_tex_old, uv).rgb;
    vec3 b = texture(u_tex_new, uv).rgb;
    vec2 o = vec2(0.0016, 0.0028);
    float gx = lum(texture(u_tex_new, uv + vec2(o.x, 0.0)).rgb)
             - lum(texture(u_tex_new, uv - vec2(o.x, 0.0)).rgb);
    float gy = lum(texture(u_tex_new, uv + vec2(0.0, o.y)).rgb)
             - lum(texture(u_tex_new, uv - vec2(0.0, o.y)).rgb);
    float e = smoothstep(0.04, 0.22, length(vec2(gx, gy)));
    vec2 cell = floor(uv * vec2(24.0, 14.0));
    float order = hash(cell) * 0.6 + uv.x * 0.4;
    float draw = smoothstep(0.05, 0.55, p);
    float lit = e * smoothstep(order, order + 0.08, draw * 1.1);
    float fill = smoothstep(0.55, 0.95, p);
    vec3 neon = mix(vec3(0.15, 0.95, 1.0), vec3(1.0, 0.25, 0.85), uv.y * 0.6 + uv.x * 0.4);
    neon = mix(neon, normalize(b + 0.05) * 1.3, 0.35);
    vec3 lines = neon * lit * 1.5;
    vec3 base = a * (1.0 - 0.85 * smoothstep(0.0, 0.4, p));
    vec3 col = mix(base + lines, b + lines * 0.25 * (1.0 - fill), fill);
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// Dusk falls on the old picture, then a glowing horizon rises and brings the
/// new one up in golden light.
pub(in crate::effects) const SUNRISE_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec2 uv = v_uv;
    vec3 a = texture(u_tex_old, uv).rgb;
    vec3 b = texture(u_tex_new, uv).rgb;
    float rise = smoothstep(0.12, 0.95, p);
    float h = mix(1.12, -0.3, rise);
    float d = uv.y - h + sin(uv.x * 9.0 + p * 3.0) * 0.012;
    float m = smoothstep(-0.02, 0.03, d);
    float dusk = smoothstep(0.0, 0.45, p);
    vec3 night = a * (1.0 - 0.55 * dusk) * mix(vec3(1.0), vec3(0.8, 0.85, 1.15), dusk);
    float fin = 1.0 - smoothstep(0.85, 1.0, p);
    float warm = (1.0 - smoothstep(0.0, 0.45, d)) * fin;
    vec3 day = b * mix(vec3(1.0), vec3(1.3, 0.98, 0.7), warm);
    vec3 col = mix(night, day, m);
    float k = sin(p * 3.14159) * fin;
    col += vec3(1.0, 0.68, 0.32) * exp(-abs(d) * 13.0) * k * 0.85;
    vec2 sun = (uv - vec2(0.5, h)) * vec2(1.7777, 1.0);
    col += vec3(1.0, 0.92, 0.65) * exp(-dot(sun, sun) * 55.0) * k;
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";
