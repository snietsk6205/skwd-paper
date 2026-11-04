// Per-frame sketch: sk_tex0 = analysis A, sk_tex1 = analysis B, sk_tex2 = the
// frozen incoming frame. u_progress is the sketch's own progress q (0..1).
// Alpha is the paper's coverage over the outgoing wallpaper, which the
// pipeline blends underneath, so the outgoing side is never sampled here.
void main() {
    vec2 uv = v_uv;
    vec2 res = sk_res();
    vec2 pos = uv * sk_aspect();
    vec2 px = uv * res;
    float q = clamp(u_progress, 0.0, 1.0);
    vec4 a1 = texture(sk_tex0, uv);
    vec4 a2 = texture(sk_tex1, uv);
    float outline = a1.r;
    float detail = a1.g;
    float pressure = a1.b;
    float glow = a1.a;
    float tOut = a2.r;
    float tDet = a2.g;
    float tSh = a2.b;
    float shade = a2.a * 2.0;
    vec3 src = texture(sk_tex2, uv).rgb;
    float lum = dot(src, vec3(0.299, 0.587, 0.114));
    float rOut = smoothstep(tOut, tOut + 0.012, q);
    float rDet = smoothstep(tDet, tDet + 0.012, q);
    float rSh = smoothstep(tSh, tSh + 0.05, q);
    float tipOut = rOut * (1.0 - smoothstep(0.0, 0.05, q - tOut));
    float tipDet = rDet * (1.0 - smoothstep(0.0, 0.05, q - tDet));
    float lines = outline * rOut + detail * rDet * 0.75;
    float tip = max(outline * tipOut, detail * tipDet * 0.75);
    vec3 ground = sk_ground(uv, pos, px, res, sk_px_scale());
    vec3 ink = sk_ink_colour(src);
    vec3 col = ground;
    if (u_style == 1) {
        col += ink * glow * max(rOut, rDet) * 0.22;
        col = mix(col, src * 0.35 + ground, rSh * shade * 0.35);
        col += ink * lines * pressure * 0.95;
        col += vec3(1.0) * tip * 0.8;
    } else if (u_style == 2) {
        float chalk = smoothstep(0.2, 0.62, sk_noise(px / 1.6) * 0.6 + sk_noise(px / 5.0) * 0.4);
        col = mix(col, ink, shade * rSh * 0.22 * chalk);
        col = mix(col, ink, clamp(lines * pressure * chalk * 1.15, 0.0, 1.0));
        col = mix(col, vec3(1.0), tip * 0.6);
    } else if (u_style == 3) {
        col = mix(col, ink, clamp(lines * 1.05, 0.0, 1.0));
        col += vec3(0.6, 0.85, 1.0) * tip * 0.55;
    } else if (u_style == 4) {
        col = mix(col, vec3(0.42, 0.42, 0.44), shade * rSh * 0.45);
        col = mix(col, ink, clamp(lines * (0.85 + 0.3 * pressure), 0.0, 1.0));
        col = mix(col, ink, tip * 0.4);
    } else if (u_style == 5) {
        float grit = smoothstep(0.15, 0.55, sk_noise(px / 1.4));
        col = mix(col, ink, shade * rSh * 0.55);
        col = mix(col, ink, clamp(lines * pressure * (0.6 + 0.5 * grit), 0.0, 1.0));
        col = mix(col, ink * 0.5, tip * 0.5);
    } else if (u_style == 6) {
        col = mix(col, ink, shade * rSh * 0.45);
        col = mix(col, ink, clamp(lines * pressure * 0.95, 0.0, 1.0));
        col = mix(col, ink * 0.6, tip * 0.6);
    } else {
        col = mix(col, ink, shade * rSh * 0.32);
        col = mix(col, ink, clamp(lines * pressure * 0.88, 0.0, 1.0));
        col = mix(col, ink * 0.35, tip * 0.7);
    }
    float cover = 1.0;
    if (q < 0.16) {
        cover = smoothstep(0.0, 0.1, q + (sk_fbm(pos * 3.0 + 9.1) - 0.5) * 0.06);
    }
    frag = vec4(clamp(col, 0.0, 1.0), cover);
}
