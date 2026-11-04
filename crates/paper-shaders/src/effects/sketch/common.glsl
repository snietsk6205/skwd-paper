#version 450
// Shared header of the "sketch before transition" passes. The push-constant
// block matches the transition layout (paper-vk `TransitionPush`, 80 bytes).
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
layout(set = 0, binding = 0) uniform sampler2D sk_tex0;
layout(set = 0, binding = 1) uniform sampler2D sk_tex1;
layout(set = 1, binding = 0) uniform sampler2D sk_tex2;
layout(set = 1, binding = 1) uniform sampler2D sk_tex3;
layout(location = 0) in vec2 v_uv;
layout(location = 0) out vec4 frag;

float sk_hash(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}
float sk_noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(sk_hash(i), sk_hash(i + vec2(1.0, 0.0)), f.x),
               mix(sk_hash(i + vec2(0.0, 1.0)), sk_hash(i + vec2(1.0, 1.0)), f.x), f.y);
}
float sk_fbm(vec2 p) {
    float v = 0.0;
    float amp = 0.5;
    for (int i = 0; i < 4; i++) {
        v += amp * sk_noise(p);
        p *= 2.03;
        amp *= 0.5;
    }
    return v;
}
vec2 sk_res() {
    return max(u_res, vec2(1.0));
}
vec2 sk_aspect() {
    vec2 res = sk_res();
    return vec2(res.x / res.y, 1.0);
}
// Where the drawing starts, in aspect-corrected position space.
vec2 sk_focus() {
    vec2 focus = u_origin_on != 0 ? u_origin : vec2(0.5, 0.45);
    return focus * sk_aspect();
}
float sk_seed() {
    return float(u_seed & 1023) * 0.7313;
}
float sk_width() {
    if (u_style == 4 || u_style == 5) return 1.8;
    if (u_style == 2) return 1.4;
    return 1.0;
}
float sk_px_scale() {
    return max(1.0, sk_res().y / 1080.0);
}
vec3 sk_ground(vec2 uv, vec2 pos, vec2 px, vec2 res, float s) {
    float grain = sk_noise(px / 2.2);
    float fibre = sk_fbm(pos * 9.0);
    if (u_style == 1) {
        vec3 c = mix(vec3(0.045, 0.05, 0.065), vec3(0.075, 0.08, 0.10), 1.0 - length(uv - 0.5));
        return c * (0.97 + 0.03 * step(0.5, fract(px.y / 3.0)));
    }
    if (u_style == 2) {
        vec3 c = vec3(0.16, 0.23, 0.19) * (0.86 + 0.22 * sk_fbm(pos * 3.0));
        return c + vec3(0.05) * smoothstep(0.62, 0.9, sk_fbm(pos * 1.7 + 4.0)) + 0.012 * grain;
    }
    if (u_style == 3) {
        vec3 c = vec3(0.10, 0.29, 0.53) * (0.94 + 0.08 * fibre);
        vec2 g1 = abs(fract(px / (28.0 * s)) - 0.5) * 28.0 * s;
        vec2 g2 = abs(fract(px / (140.0 * s)) - 0.5) * 140.0 * s;
        float minor = 1.0 - smoothstep(0.0, 1.0, min(g1.x, g1.y));
        float major = 1.0 - smoothstep(0.0, 1.4, min(g2.x, g2.y));
        return c + vec3(0.55, 0.7, 0.9) * (minor * 0.07 + major * 0.14);
    }
    if (u_style == 4) {
        return vec3(0.955, 0.94, 0.895) * (0.96 + 0.04 * fibre)
            - 0.03 * smoothstep(0.7, 0.95, sk_noise(pos * 60.0));
    }
    if (u_style == 5) {
        return vec3(0.85, 0.83, 0.79) * (0.93 + 0.05 * grain + 0.04 * fibre);
    }
    if (u_style == 6) {
        vec3 c = vec3(0.985, 0.98, 0.965) * (0.985 + 0.015 * grain);
        float rule = 1.0 - smoothstep(0.0, 1.3 * s, abs(fract(px.y / (36.0 * s)) - 0.5) * 36.0 * s);
        float margin = 1.0 - smoothstep(0.0, 1.5 * s, abs(uv.x - 0.085) * res.x);
        c = mix(c, vec3(0.55, 0.72, 0.88), rule * 0.45);
        return mix(c, vec3(0.88, 0.35, 0.38), margin * 0.6);
    }
    return vec3(0.945, 0.93, 0.90) * (0.965 + 0.035 * grain);
}
vec3 sk_ink_colour(vec3 src) {
    if (u_style == 1) {
        float m = max(max(src.r, src.g), src.b);
        vec3 tint = m > 0.001 ? src / m : vec3(1.0);
        return mix(vec3(0.35, 0.85, 1.0), tint, 0.55);
    }
    if (u_style == 2) return vec3(0.93, 0.94, 0.90);
    if (u_style == 3) return vec3(0.90, 0.95, 1.0);
    if (u_style == 4) return vec3(0.05, 0.05, 0.06);
    if (u_style == 5) return vec3(0.12, 0.11, 0.11);
    if (u_style == 6) return vec3(0.13, 0.22, 0.58);
    return vec3(0.22, 0.21, 0.24);
}
