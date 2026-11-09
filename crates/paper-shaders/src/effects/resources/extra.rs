//! Custom build (M3): more Warp, Break, Fade and Wipe transitions.
//!
//! Same dialect as the upstream effects (GLSL 330, `u_tex_old`, `u_tex_new`,
//! `u_progress` 0 → 1). Point-based ones list their start-point edits in
//! `ORIGIN_EFFECTS` (`effects/vulkan.rs`).

/// The picture turns to liquid, flows and settles as the new one.
pub(in crate::effects) const LIQUID_FRAG: &str = "#version 330 core
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
    for (int i = 0; i < 4; i++) {
        v += amp * noise(p);
        p *= 2.1;
        amp *= 0.5;
    }
    return v;
}
void main() {
    float p = u_progress;
    vec2 uv = v_uv;
    float k = sin(p * 3.14159);
    vec2 s = uv * vec2(1.7777, 1.0) * 2.6;
    vec2 flow = vec2(fbm(s + vec2(p * 1.6, 0.0)), fbm(s + vec2(5.2, p * 1.3))) - 0.5;
    vec2 off = flow * 0.14 * k;
    vec3 a = texture(u_tex_old, uv + off).rgb;
    vec3 b = texture(u_tex_new, uv - off).rgb;
    float n = fbm(s * 0.8 + 3.0);
    float m = smoothstep(n - 0.12, n + 0.12, p * 1.3 - 0.15);
    vec3 col = mix(a, b, m);
    float spec = pow(max(flow.x + flow.y + 0.25, 0.0), 3.0);
    col += vec3(0.6, 0.7, 0.8) * spec * k * 0.5;
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// A magnifying lens grows from a point and shows the new picture through it.
pub(in crate::effects) const LENS_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec2 c = v_uv - vec2(0.5);
    vec2 o = v_uv - c;
    vec2 d = c * vec2(1.7777, 1.0);
    float r = length(d);
    float e = p * p * (3.0 - 2.0 * p);
    float radius = e * 1.3;
    float inside = smoothstep(radius, radius - 0.012, r);
    float z = clamp(r / max(radius, 0.001), 0.0, 1.0);
    float mag = mix(0.5 + 0.5 * z * z, 1.0, smoothstep(0.55, 1.0, p));
    vec3 a = texture(u_tex_old, v_uv).rgb;
    vec3 b = texture(u_tex_new, o + c * mag).rgb;
    float fin = 1.0 - smoothstep(0.85, 1.0, p);
    b *= 1.0 - 0.3 * z * z * z * fin;
    vec3 col = mix(a * (1.0 - 0.3 * exp(-max(r - radius, 0.0) * 12.0) * fin), b, inside);
    col += vec3(0.85, 0.92, 1.0) * exp(-abs(r - radius + 0.006) * 90.0) * fin * 0.6;
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// The old picture waves like a flag in the wind and blows away.
pub(in crate::effects) const FLAG_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec2 uv = v_uv;
    float wave = sin(uv.x * 10.0 - p * 16.0 + uv.y * 3.0);
    float amp = (0.02 + 0.06 * p) * uv.x;
    float shift = p * p * 1.7 * (0.3 + uv.x);
    vec2 f = vec2(uv.x - shift, uv.y + wave * amp);
    vec3 b = texture(u_tex_new, uv).rgb;
    vec3 col;
    if (f.x >= 0.0 && f.x <= 1.0 && f.y >= 0.0 && f.y <= 1.0) {
        col = texture(u_tex_old, f).rgb * (0.78 + 0.22 * wave * min(1.0, p * 4.0) + 0.22 * (1.0 - min(1.0, p * 4.0)));
    } else {
        float near = f.x > 1.0 ? 0.0 : exp(-abs(f.x) * 14.0);
        col = b * (1.0 - 0.4 * near);
    }
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// The old picture falls away down a tunnel and the new one rushes out of it.
pub(in crate::effects) const TUNNEL_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    float e = p * p * (3.0 - 2.0 * p);
    vec2 c = v_uv - vec2(0.5);
    vec2 d = c * vec2(1.7777, 1.0);
    float r = length(d);
    float ang = atan(d.y, d.x);
    float rot = e * 1.4;
    vec2 cr = vec2(c.x * cos(rot) - c.y * sin(rot) * 0.5625, c.x * sin(rot) * 1.7777 + c.y * cos(rot));
    vec2 auv = vec2(0.5) + cr * mix(1.0, 30.0, e * e * e);
    vec2 buv = vec2(0.5) + c * mix(0.35, 1.0, e);
    vec3 b = texture(u_tex_new, buv).rgb;
    vec3 col = b;
    if (auv.x >= 0.0 && auv.x <= 1.0 && auv.y >= 0.0 && auv.y <= 1.0) {
        vec3 a = texture(u_tex_old, auv).rgb * (1.0 - 0.5 * e);
        col = mix(a, b, smoothstep(0.55, 0.9, e));
    }
    float k = sin(p * 3.14159) * (1.0 - smoothstep(0.85, 1.0, p));
    float streak = pow(0.5 + 0.5 * sin(ang * 40.0 + p * 6.0), 10.0) * smoothstep(0.15, 0.9, r);
    col += vec3(0.8, 0.9, 1.0) * streak * k * 0.35;
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// The old picture folds up like an accordion towards the right edge.
pub(in crate::effects) const ACCORDION_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec2 uv = v_uv;
    float e = p < 0.5 ? 4.0 * p * p * p : 1.0 - pow(-2.0 * p + 2.0, 3.0) / 2.0;
    vec3 col;
    if (uv.x >= e) {
        float local = (uv.x - e) / max(1.0 - e, 0.0001);
        float f = fract(local * 8.0);
        float squash = 1.0 - 0.06 * e * sin(f * 3.14159);
        float y = (uv.y - 0.5) / squash + 0.5;
        float shade = f < 0.5 ? 1.0 - 0.12 * e : 1.0 - 0.6 * e;
        shade *= 1.0 - 0.25 * e * abs(f - 0.5) * 2.0;
        col = (y >= 0.0 && y <= 1.0) ? texture(u_tex_old, vec2(local, y)).rgb * shade
                                     : texture(u_tex_new, uv).rgb;
    } else {
        col = texture(u_tex_new, uv).rgb * (1.0 - 0.45 * exp(-(e - uv.x) * 30.0) * step(e, 0.999));
    }
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// Cracks run out from a point and the pieces of glass shrink away.
pub(in crate::effects) const GLASS_FRAG: &str = "#version 330 core
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
    vec2 c = v_uv - vec2(0.5);
    vec2 d = c * vec2(1.7777, 1.0);
    float r = length(d);
    float sectors = 15.0;
    float a01 = atan(d.y, d.x) / 6.28318 + 0.5;
    float ring_f = r * 5.5;
    float ring = floor(ring_f);
    float sp = a01 * sectors + hash(vec2(ring, 3.0)) * 0.9;
    float si = floor(sp);
    vec2 sid = vec2(mod(si, sectors), ring);
    float h = hash(sid);
    float crack = smoothstep(0.0, 0.22, p);
    float reach = 1.1;
    float start = 0.12 + h * 0.2 + clamp(r / reach, 0.0, 1.0) * 0.28;
    float t = clamp((p - start) / 0.4, 0.0, 1.0);
    t = t * t;
    float s = 1.0 - t;
    vec2 local = vec2(fract(sp) - 0.5, fract(ring_f) - 0.5);
    float box = max(abs(local.x), abs(local.y)) * 2.0;
    float inside = smoothstep(s + 0.02, s - 0.02, box) * step(0.001, s);
    vec3 a = texture(u_tex_old, v_uv).rgb * (0.85 + 0.3 * h * crack);
    vec3 b = texture(u_tex_new, v_uv).rgb;
    float lines = (1.0 - smoothstep(0.0, 0.035, 0.5 - max(abs(local.x), abs(local.y)) * 1.0))
                * crack * (1.0 - t);
    lines *= smoothstep(crack * reach * 1.3 + 0.02, crack * reach * 1.3 - 0.05, r);
    vec3 col = mix(b, a + vec3(0.9) * lines, inside);
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// The old picture tears across like paper and the halves pull apart.
pub(in crate::effects) const PAPER_TEAR_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
float hash(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}
float noise(float x) {
    float i = floor(x);
    float f = fract(x);
    f = f * f * (3.0 - 2.0 * f);
    return mix(hash(vec2(i, 1.7)), hash(vec2(i + 1.0, 1.7)), f);
}
float tear(float x) {
    return 0.5 + (noise(x * 9.0) - 0.5) * 0.08 + (noise(x * 37.0) - 0.5) * 0.025 + sin(x * 3.0) * 0.02;
}
void main() {
    float p = u_progress;
    vec2 uv = v_uv;
    float e = p * p * (3.0 - 2.0 * p);
    float gap_top = e * 0.62 * (1.0 + 0.35 * (uv.x - 0.5));
    float gap_bottom = e * 0.62 * (1.0 - 0.35 * (uv.x - 0.5));
    float line = tear(uv.x);
    vec3 col = texture(u_tex_new, uv).rgb;
    vec2 top = vec2(uv.x, uv.y + gap_top);
    vec2 bottom = vec2(uv.x, uv.y - gap_bottom);
    float shadow = 0.0;
    if (top.y < line) {
        float fibre = smoothstep(line - 0.012, line, top.y) * step(0.001, e);
        col = mix(texture(u_tex_old, top).rgb, vec3(0.95, 0.94, 0.9), fibre * 0.85);
    } else if (bottom.y > line) {
        float fibre = smoothstep(line + 0.012, line, bottom.y) * step(0.001, e);
        col = mix(texture(u_tex_old, bottom).rgb, vec3(0.95, 0.94, 0.9), fibre * 0.85);
    } else {
        shadow = exp(-(top.y - line) * 30.0) + exp(-(line - bottom.y) * 30.0);
        col *= 1.0 - 0.45 * clamp(shadow, 0.0, 1.0);
    }
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// An old TV switches off into a line and a dot, then on again with the new
/// picture.
pub(in crate::effects) const CRT_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec2 uv = v_uv;
    float t1 = p < 0.5 ? p * 2.0 : (1.0 - p) * 2.0;
    float sy = mix(1.0, 0.004, smoothstep(0.0, 0.7, t1));
    float sx = mix(1.0, 0.0, smoothstep(0.65, 1.0, t1));
    vec2 cuv = (uv - vec2(0.5)) / vec2(max(sx, 0.0005), sy) + vec2(0.5);
    bool inside = cuv.x >= 0.0 && cuv.x <= 1.0 && cuv.y >= 0.0 && cuv.y <= 1.0 && sx > 0.0005;
    vec3 col = vec3(0.0);
    if (inside) {
        vec3 pic = p < 0.5 ? texture(u_tex_old, cuv).rgb : texture(u_tex_new, cuv).rgb;
        col = pic * (1.0 + 2.2 * smoothstep(0.3, 0.9, t1));
    }
    col *= 1.0 - 0.25 * t1 * (0.5 + 0.5 * sin(uv.y * 900.0));
    vec2 dd = (uv - vec2(0.5)) * vec2(1.7777, 1.0);
    col += vec3(0.85, 0.92, 1.0) * exp(-length(dd) * 28.0) * smoothstep(0.75, 1.0, t1);
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// Blocks of the new picture smear over the old one like a broken video
/// stream.
pub(in crate::effects) const DATAMOSH_FRAG: &str = "#version 330 core
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
    vec2 uv = v_uv;
    vec2 blk = floor(uv * vec2(32.0, 18.0));
    float h = hash(blk);
    float k = sin(p * 3.14159);
    vec2 big = floor(uv * vec2(8.0, 4.5));
    vec2 mv = (vec2(hash(big + 1.3), hash(big + 7.1)) - 0.5) * 0.12 * k;
    float on = step(h, p * 1.25 - 0.1);
    float smear = fract(uv.y * 18.0);
    vec3 a = texture(u_tex_old, uv + mv * 0.4).rgb;
    vec3 b = texture(u_tex_new, uv - mv * smear).rgb;
    vec3 col = mix(a, b, on);
    float bleed = k * 0.012;
    col.r = mix(col.r, texture(u_tex_new, uv + vec2(bleed, 0.0)).r, on * k * 0.6);
    col.b = mix(col.b, texture(u_tex_old, uv - vec2(bleed, 0.0)).b, (1.0 - on) * k * 0.6);
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// Shadows first: the dark parts of the new picture appear before the bright
/// ones.
pub(in crate::effects) const LUMA_FADE_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec3 a = texture(u_tex_old, v_uv).rgb;
    vec3 b = texture(u_tex_new, v_uv).rgb;
    float l = dot(b, vec3(0.299, 0.587, 0.114));
    float thr = p * 1.24 - 0.12;
    float m = smoothstep(l - 0.1, l + 0.1, thr);
    frag = vec4(mix(a, b, m), 1.0);
}
";

/// The old picture melts into a soft dreamy blur and the new one comes into
/// focus.
pub(in crate::effects) const DREAM_BLUR_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    float k = sin(p * 3.14159);
    float radius = 0.035 * k;
    vec3 sa = vec3(0.0);
    vec3 sb = vec3(0.0);
    for (int i = 0; i < 16; i++) {
        float fi = float(i);
        float ang = fi * 2.3999632;
        float rr = sqrt((fi + 0.5) / 16.0) * radius;
        vec2 o = vec2(cos(ang) * 0.5625, sin(ang)) * rr;
        sa += texture(u_tex_old, v_uv + o).rgb;
        sb += texture(u_tex_new, v_uv + o).rgb;
    }
    vec3 col = mix(sa, sb, smoothstep(0.3, 0.7, p)) / 16.0;
    col += max(col - vec3(0.55), vec3(0.0)) * 0.9 * k;
    col *= 1.0 + 0.08 * k;
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// A grainy old-film dissolve with flicker and dust.
pub(in crate::effects) const FILM_DISSOLVE_FRAG: &str = "#version 330 core
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
    vec2 uv = v_uv;
    float frame = floor(p * 24.0);
    float k = sin(p * 3.14159);
    vec3 a = texture(u_tex_old, uv).rgb;
    vec3 b = texture(u_tex_new, uv).rgb;
    float g = hash(floor(uv * vec2(480.0, 270.0)) + vec2(frame * 13.0, frame * 7.0));
    float m = smoothstep(g - 0.25, g + 0.25, p * 1.5 - 0.25);
    vec3 col = mix(a, b, m);
    float grey = dot(col, vec3(0.299, 0.587, 0.114));
    col = mix(col, vec3(grey) * vec3(1.08, 0.98, 0.82), 0.35 * k);
    col *= 1.0 + 0.07 * k * sin(p * 95.0);
    col += (g - 0.5) * 0.12 * k;
    float dust = step(0.9985, hash(floor(uv * vec2(160.0, 90.0)) + vec2(frame, 3.0)));
    col = mix(col, vec3(0.05), dust * k);
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// A clock hand sweeps round a point and leaves the new picture behind it.
pub(in crate::effects) const CLOCK_WIPE_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec2 c = v_uv - vec2(0.5);
    vec2 d = c * vec2(1.7777, 1.0);
    float a01 = fract(atan(d.x, -d.y) / 6.28318 + 1.0);
    float e = p * p * (3.0 - 2.0 * p);
    float sweep = e * 1.03;
    float soft = 0.004 + 0.02 / (1.0 + length(d) * 30.0);
    float m = smoothstep(sweep, sweep - soft, a01);
    vec3 a = texture(u_tex_old, v_uv).rgb;
    vec3 b = texture(u_tex_new, v_uv).rgb;
    vec3 col = mix(a, b, m);
    float fin = 1.0 - smoothstep(0.9, 1.0, p);
    float hand = exp(-abs(a01 - sweep) * 120.0 * (0.3 + length(d))) * fin * step(0.001, p);
    col += vec3(1.0, 0.95, 0.85) * hand * 0.5;
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// The old picture splits in the middle and swings open like two doors.
pub(in crate::effects) const BARN_DOORS_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec2 uv = v_uv;
    float e = p * p * (3.0 - 2.0 * p);
    float w = 0.5 * (1.0 - e);
    vec3 col = texture(u_tex_new, uv).rgb;
    float shadow = 0.0;
    if (uv.x < w) {
        float local = uv.x / max(w, 0.0001);
        float grow = 1.0 + 0.18 * e * local;
        float y = (uv.y - 0.5) / grow + 0.5;
        if (y >= 0.0 && y <= 1.0) {
            col = texture(u_tex_old, vec2(local * 0.5, y)).rgb * (1.0 - 0.55 * e * local);
        }
    } else if (uv.x > 1.0 - w) {
        float local = (1.0 - uv.x) / max(w, 0.0001);
        float grow = 1.0 + 0.18 * e * local;
        float y = (uv.y - 0.5) / grow + 0.5;
        if (y >= 0.0 && y <= 1.0) {
            col = texture(u_tex_old, vec2(1.0 - local * 0.5, y)).rgb * (1.0 - 0.55 * e * local);
        }
    } else {
        shadow = exp(-(uv.x - w) * 25.0) + exp(-(1.0 - w - uv.x) * 25.0);
        col *= 1.0 - 0.4 * clamp(shadow, 0.0, 1.0) * (1.0 - e);
    }
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// The old picture gathers into folds like stage curtains and draws open.
pub(in crate::effects) const CURTAIN_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec2 uv = v_uv;
    float e = p < 0.5 ? 4.0 * p * p * p : 1.0 - pow(-2.0 * p + 2.0, 3.0) / 2.0;
    float tie = 1.0 - 0.45 * e * sin(uv.y * 3.14159 * 0.85);
    float w = 0.5 * (1.0 - e) * tie;
    float side = uv.x < 0.5 ? uv.x : 1.0 - uv.x;
    vec3 col = texture(u_tex_new, uv).rgb;
    if (side < w) {
        float local = side / max(w, 0.0001);
        float folds = 5.0 + 9.0 * e;
        float wave = sin(local * 3.14159 * folds + uv.y * 2.0 * e);
        float src = local * 0.5 + wave * 0.012 * e;
        float x = uv.x < 0.5 ? src : 1.0 - src;
        float shade = 1.0 - e * (0.3 + 0.3 * wave) * (0.4 + 0.6 * local);
        col = texture(u_tex_old, vec2(x, uv.y)).rgb * shade;
    } else {
        col *= 1.0 - 0.35 * exp(-(side - w) * 30.0) * step(0.001, e) * (1.0 - e);
    }
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";

/// A wavy edge rolls across the screen with a line of foam on top.
pub(in crate::effects) const WAVE_WIPE_FRAG: &str = "#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
void main() {
    float p = u_progress;
    vec2 uv = v_uv;
    float edge = p * 1.45 - 0.2 + sin(uv.y * 14.0 + p * 10.0) * 0.035 + sin(uv.y * 31.0 - p * 7.0) * 0.012;
    float d = uv.x + uv.y * 0.15 - edge;
    float m = smoothstep(0.008, -0.008, d);
    float lift = exp(-max(d, 0.0) * 22.0) * step(0.0, d);
    vec3 a = texture(u_tex_old, uv - vec2(0.025 * lift, 0.0)).rgb;
    vec3 b = texture(u_tex_new, uv).rgb;
    vec3 col = mix(a, b, m);
    float fin = 1.0 - smoothstep(0.92, 1.0, p);
    col += vec3(0.92, 0.97, 1.0) * exp(-abs(d) * 85.0) * 0.55 * fin;
    col *= 1.0 - 0.25 * exp(-max(-d, 0.0) * 25.0) * step(d, 0.0) * fin;
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
";
