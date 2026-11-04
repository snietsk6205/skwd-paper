// One-time analysis of the frozen incoming frame (sk_tex0).
// u_pid_base == 0 writes (outline, detail, pressure, glow).
// u_pid_base == 1 writes (outline time, detail time, shading time, shade / 2).
// Everything the per-frame compose pass needs is cached here, so the
// expensive edge detection runs once per transition instead of every frame.
float sk_luma(vec2 q) {
    return dot(texture(sk_tex0, q).rgb, vec3(0.299, 0.587, 0.114));
}
vec2 sk_sobel(vec2 q, vec2 s) {
    float tl = sk_luma(q + vec2(-s.x, s.y));
    float tc = sk_luma(q + vec2(0.0, s.y));
    float tr = sk_luma(q + vec2(s.x, s.y));
    float ml = sk_luma(q + vec2(-s.x, 0.0));
    float mr = sk_luma(q + vec2(s.x, 0.0));
    float bl = sk_luma(q + vec2(-s.x, -s.y));
    float bc = sk_luma(q + vec2(0.0, -s.y));
    float br = sk_luma(q + vec2(s.x, -s.y));
    return vec2((tr + 2.0 * mr + br) - (tl + 2.0 * ml + bl),
                (tl + 2.0 * tc + tr) - (bl + 2.0 * bc + br));
}
float sk_draw_time(vec2 pos, vec2 focus, vec2 tangent, float start, float span, float seed) {
    float cs = 0.34;
    vec2 g = floor(pos / cs);
    vec2 best = vec2(0.0);
    vec2 bestId = vec2(0.0);
    float bd = 1e9;
    for (int j = -1; j <= 1; j++) {
        for (int i = -1; i <= 1; i++) {
            vec2 id = g + vec2(float(i), float(j));
            vec2 c = (id + 0.2 + 0.6 * vec2(sk_hash(id + seed), sk_hash(id + seed + 17.3))) * cs;
            float d = length(pos - c);
            if (d < bd) {
                bd = d;
                best = c;
                bestId = id;
            }
        }
    }
    float ang = sk_hash(bestId + seed + 3.1) * 6.2831;
    vec2 dir = vec2(cos(ang), sin(ang));
    vec2 t = dot(tangent, dir) < 0.0 ? -tangent : tangent;
    vec2 flow = normalize(mix(dir, t, 0.7) + vec2(1e-4));
    float along = clamp(dot(pos - best, flow) / (cs * 1.3) + 0.5, 0.0, 1.0);
    float across = dot(pos - best, vec2(-flow.y, flow.x));
    float stroke = sk_hash(vec2(floor(across * 30.0), bestId.x * 7.0 + bestId.y + seed));
    float order = clamp(length(best - focus) / 1.25 + 0.25 * sk_hash(bestId + seed + 9.7), 0.0, 1.0);
    return start + span * (0.55 * order + 0.3 * stroke * 0.5 + 0.3 * along * (0.6 + 0.4 * stroke));
}
void main() {
    vec2 uv = v_uv;
    vec2 res = sk_res();
    vec2 asp = sk_aspect();
    vec2 pos = uv * asp;
    vec2 px = uv * res;
    vec2 focus = sk_focus();
    float scale = sk_px_scale() * sk_width();
    vec2 s1 = scale / res;
    vec2 g = sk_sobel(uv, s1) * 0.75 + sk_sobel(uv, s1 * 2.0) * 0.25;
    float mag = length(g);
    vec3 src = texture(sk_tex0, uv).rgb;
    float lum = dot(src, vec3(0.299, 0.587, 0.114));
    float dark = 1.0 - lum;
    float m = mag * mix(0.75, 1.25, dark);
    float outline = smoothstep(0.42, 0.9, m);
    float detail = smoothstep(0.14, 0.42, m) * (1.0 - outline);
    vec2 tangent = mag > 0.0001 ? vec2(-g.y, g.x) / mag : vec2(1.0, 0.0);
    if (u_pid_base == 0) {
        float pressure = 0.72 + 0.28 * sk_noise(vec2(dot(pos, tangent) * 40.0,
                                                     dot(pos, vec2(-tangent.y, tangent.x)) * 5.0));
        float glow = u_style == 1 ? smoothstep(0.08, 0.5, length(sk_sobel(uv, s1 * 3.0))) : 0.0;
        frag = vec4(outline, detail, pressure, glow);
        return;
    }
    float seed = sk_seed();
    float tOut = sk_draw_time(pos, focus, tangent, 0.07, 0.55, 1.0 + seed);
    float tDet = sk_draw_time(pos, focus, tangent, 0.32, 0.50, 5.0 + seed);
    float tSh = sk_draw_time(pos, focus, vec2(0.7071, 0.7071), 0.55, 0.40, 11.0 + seed);
    vec2 r12 = 9.0 * sk_px_scale() * sk_width() / res;
    float around = 0.125 * (sk_luma(uv + vec2(r12.x, 0.0)) + sk_luma(uv - vec2(r12.x, 0.0))
        + sk_luma(uv + vec2(0.0, r12.y)) + sk_luma(uv - vec2(0.0, r12.y))
        + sk_luma(uv + r12 * 0.7071) + sk_luma(uv - r12 * 0.7071)
        + sk_luma(uv + vec2(r12.x, -r12.y) * 0.7071) + sk_luma(uv + vec2(-r12.x, r12.y) * 0.7071));
    float subject = 1.0 - smoothstep(0.35, 0.85, length(pos - focus));
    float tone = clamp(smoothstep(0.03, 0.2, around - lum)
        + smoothstep(0.55, 0.95, dark) * 0.35 * subject, 0.0, 1.0);
    float tone2 = smoothstep(0.12, 0.3, around - lum);
    float hs = 6.0 * sk_px_scale();
    float wob = (sk_noise(px / 45.0) - 0.5) * 2.5;
    float h1 = smoothstep(0.55, 0.95,
        sin((dot(px, vec2(0.7071, 0.7071)) + wob * hs) / hs * 6.2831) * 0.5 + 0.5);
    float h2 = smoothstep(0.55, 0.95,
        sin((dot(px, vec2(-0.7071, 0.7071)) + wob * hs) / hs * 6.2831) * 0.5 + 0.5);
    float hatch = (h1 * tone + h2 * tone2 * 0.7) * (0.6 + 0.4 * sk_noise(px / 25.0));
    float shade = hatch;
    if (u_style == 1) {
        shade = smoothstep(0.1, 0.9, lum);
    } else if (u_style == 3) {
        shade = 0.0;
    } else if (u_style == 4) {
        shade = max(tone, smoothstep(0.55, 0.95, dark) * 0.5) * (0.55 + 0.45 * sk_fbm(pos * 5.0));
    } else if (u_style == 5) {
        shade = tone * (0.6 + 0.4 * sk_fbm(pos * 4.0));
    }
    frag = vec4(clamp(tOut, 0.0, 1.0), clamp(tDet, 0.0, 1.0), clamp(tSh, 0.0, 1.0),
                clamp(shade * 0.5, 0.0, 1.0));
}
