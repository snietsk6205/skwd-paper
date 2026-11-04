#version 330 core
in vec2 v_uv;
out vec4 frag;
uniform sampler2D u_tex_old;
uniform sampler2D u_tex_new;
uniform float u_progress;
uniform vec2 u_res;
float rv_hash(vec2 p) {
    return fract(sin(dot(p, vec2(269.5, 183.3))) * 43758.5453);
}
float rv_noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(rv_hash(i), rv_hash(i + vec2(1.0, 0.0)), f.x),
               mix(rv_hash(i + vec2(0.0, 1.0)), rv_hash(i + vec2(1.0, 1.0)), f.x), f.y);
}
float rv_fbm(vec2 p) {
    float v = 0.0;
    float amp = 0.5;
    for (int i = 0; i < 4; i++) {
        v += amp * rv_noise(p);
        p *= 2.0;
        amp *= 0.5;
    }
    return v;
}
void main() {
    float p = clamp(u_progress, 0.0, 1.0);
    vec4 b = texture(u_tex_new, v_uv);
    if (p >= 1.0) {
        frag = b;
        return;
    }
    vec4 a = texture(u_tex_old, v_uv);
    vec2 res = max(u_res, vec2(1.0));
    vec2 asp = vec2(res.x / res.y, 1.0);
    vec2 pos = v_uv * asp;
    float d = length((v_uv - vec2(0.5, 0.45)) * asp) + (rv_fbm(pos * 3.0 + vec2(1.3, 7.7)) - 0.5) * 0.35;
    float e = p * p * (3.0 - 2.0 * p);
    float radius = e * 1.45;
    float reveal = smoothstep(radius, radius - 0.07, d) * step(0.0001, p);
    float rim = exp(-abs(d - radius) * 40.0) * step(0.0001, p) * (1.0 - p);
    vec3 col = mix(a.rgb, b.rgb, reveal);
    col = mix(col, col * 0.8, rim * 0.55);
    frag = vec4(col, 1.0);
}
