int sk_style;
float sk_split;
float sk_hash(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
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
void sk_decode() {
    int code = u_pid_base - 1;
    if (code < 0) {
        sk_style = 0;
        sk_split = 0.0;
        return;
    }
    sk_style = code / 10000;
    sk_split = clamp(float(code - sk_style * 10000) / 1000.0, 0.0, 0.95);
}
float sk_luma(vec2 q) {
    return dot(texture(u_tex_new, q).rgb, vec3(0.299, 0.587, 0.114));
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
float sk_width() {
    if (sk_style == 4 || sk_style == 5) return 1.8;
    if (sk_style == 2) return 1.4;
    return 1.0;
}
vec3 sk_ground(vec2 uv, vec2 pos, vec2 px, vec2 res, float s) {
    float grain = sk_noise(px / 2.2);
    float fibre = sk_fbm(pos * 9.0);
    if (sk_style == 1) {
        vec3 c = mix(vec3(0.045, 0.05, 0.065), vec3(0.075, 0.08, 0.10), 1.0 - length(uv - 0.5));
        return c * (0.97 + 0.03 * step(0.5, fract(px.y / 3.0)));
    }
    if (sk_style == 2) {
        vec3 c = vec3(0.16, 0.23, 0.19) * (0.86 + 0.22 * sk_fbm(pos * 3.0));
        return c + vec3(0.05) * smoothstep(0.62, 0.9, sk_fbm(pos * 1.7 + 4.0)) + 0.012 * grain;
    }
    if (sk_style == 3) {
        vec3 c = vec3(0.10, 0.29, 0.53) * (0.94 + 0.08 * fibre);
        vec2 g1 = abs(fract(px / (28.0 * s)) - 0.5) * 28.0 * s;
        vec2 g2 = abs(fract(px / (140.0 * s)) - 0.5) * 140.0 * s;
        float minor = 1.0 - smoothstep(0.0, 1.0, min(g1.x, g1.y));
        float major = 1.0 - smoothstep(0.0, 1.4, min(g2.x, g2.y));
        return c + vec3(0.55, 0.7, 0.9) * (minor * 0.07 + major * 0.14) ;
    }
    if (sk_style == 4) {
        return vec3(0.955, 0.94, 0.895) * (0.96 + 0.04 * fibre) - 0.03 * smoothstep(0.7, 0.95, sk_noise(pos * 60.0));
    }
    if (sk_style == 5) {
        return vec3(0.85, 0.83, 0.79) * (0.93 + 0.05 * grain + 0.04 * fibre);
    }
    if (sk_style == 6) {
        vec3 c = vec3(0.985, 0.98, 0.965) * (0.985 + 0.015 * grain);
        float rule = 1.0 - smoothstep(0.0, 1.3 * s, abs(fract(px.y / (36.0 * s)) - 0.5) * 36.0 * s);
        float margin = 1.0 - smoothstep(0.0, 1.5 * s, abs(uv.x - 0.085) * res.x);
        c = mix(c, vec3(0.55, 0.72, 0.88), rule * 0.45);
        return mix(c, vec3(0.88, 0.35, 0.38), margin * 0.6);
    }
    return vec3(0.945, 0.93, 0.90) * (0.965 + 0.035 * grain);
}
vec3 sk_ink_colour(vec3 src) {
    if (sk_style == 1) {
        float m = max(max(src.r, src.g), src.b);
        vec3 tint = m > 0.001 ? src / m : vec3(1.0);
        return mix(vec3(0.35, 0.85, 1.0), tint, 0.55);
    }
    if (sk_style == 2) return vec3(0.93, 0.94, 0.90);
    if (sk_style == 3) return vec3(0.90, 0.95, 1.0);
    if (sk_style == 4) return vec3(0.05, 0.05, 0.06);
    if (sk_style == 5) return vec3(0.12, 0.11, 0.11);
    if (sk_style == 6) return vec3(0.13, 0.22, 0.58);
    return vec3(0.22, 0.21, 0.24);
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
vec3 sk_frame(vec2 uv, float q) {
    vec2 res = max(u_res, vec2(1.0));
    vec2 asp = vec2(res.x / res.y, 1.0);
    vec2 pos = uv * asp;
    vec2 px = uv * res;
    vec2 focus = vec2(0.5, 0.45) * asp;
    float scale = max(1.0, res.y / 1080.0) * sk_width();
    vec2 s1 = scale / res;
    vec2 g = sk_sobel(uv, s1) * 0.75 + sk_sobel(uv, s1 * 2.0) * 0.25;
    float mag = length(g);
    vec3 src = texture(u_tex_new, uv).rgb;
    float lum = dot(src, vec3(0.299, 0.587, 0.114));
    float dark = 1.0 - lum;
    float m = mag * mix(0.75, 1.25, dark);
    float outline = smoothstep(0.42, 0.9, m);
    float detail = smoothstep(0.14, 0.42, m) * (1.0 - outline);
    vec2 tangent = mag > 0.0001 ? vec2(-g.y, g.x) / mag : vec2(1.0, 0.0);
    float tOut = sk_draw_time(pos, focus, tangent, 0.07, 0.55, 1.0);
    float tDet = sk_draw_time(pos, focus, tangent, 0.32, 0.50, 5.0);
    float tSh = sk_draw_time(pos, focus, vec2(0.7071, 0.7071), 0.55, 0.40, 11.0);
    float rOut = smoothstep(tOut, tOut + 0.012, q);
    float rDet = smoothstep(tDet, tDet + 0.012, q);
    float rSh = smoothstep(tSh, tSh + 0.05, q);
    float tipOut = rOut * (1.0 - smoothstep(0.0, 0.05, q - tOut));
    float tipDet = rDet * (1.0 - smoothstep(0.0, 0.05, q - tDet));
    float pressure = 0.72 + 0.28 * sk_noise(vec2(dot(pos, tangent) * 40.0, dot(pos, vec2(-tangent.y, tangent.x)) * 5.0));
    float lines = outline * rOut + detail * rDet * 0.75;
    float tip = max(outline * tipOut, detail * tipDet * 0.75);
    vec2 r12 = 9.0 * scale / res;
    float around = 0.125 * (sk_luma(uv + vec2(r12.x, 0.0)) + sk_luma(uv - vec2(r12.x, 0.0))
        + sk_luma(uv + vec2(0.0, r12.y)) + sk_luma(uv - vec2(0.0, r12.y))
        + sk_luma(uv + r12 * 0.7071) + sk_luma(uv - r12 * 0.7071)
        + sk_luma(uv + vec2(r12.x, -r12.y) * 0.7071) + sk_luma(uv + vec2(-r12.x, r12.y) * 0.7071));
    float subject = 1.0 - smoothstep(0.35, 0.85, length(pos - focus));
    float tone = clamp(smoothstep(0.03, 0.2, around - lum) + smoothstep(0.55, 0.95, dark) * 0.35 * subject, 0.0, 1.0);
    float tone2 = smoothstep(0.12, 0.3, around - lum);
    float hs = 6.0 * max(1.0, res.y / 1080.0);
    float wob = (sk_noise(px / 45.0) - 0.5) * 2.5;
    float h1 = smoothstep(0.55, 0.95, sin((dot(px, vec2(0.7071, 0.7071)) + wob * hs) / hs * 6.2831) * 0.5 + 0.5);
    float h2 = smoothstep(0.55, 0.95, sin((dot(px, vec2(-0.7071, 0.7071)) + wob * hs) / hs * 6.2831) * 0.5 + 0.5);
    float hatch = (h1 * tone + h2 * tone2 * 0.7) * (0.6 + 0.4 * sk_noise(px / 25.0));
    vec3 ground = sk_ground(uv, pos, px, res, max(1.0, res.y / 1080.0));
    vec3 ink = sk_ink_colour(src);
    vec3 col = ground;
    if (sk_style == 1) {
        float glow = smoothstep(0.08, 0.5, length(sk_sobel(uv, s1 * 3.0))) * max(rOut, rDet);
        col += ink * glow * 0.22;
        col = mix(col, src * 0.35 + ground, rSh * smoothstep(0.1, 0.9, lum) * 0.35);
        col += ink * lines * pressure * 0.95;
        col += vec3(1.0) * tip * 0.8;
    } else if (sk_style == 2) {
        float chalk = smoothstep(0.2, 0.62, sk_noise(px / 1.6) * 0.6 + sk_noise(px / 5.0) * 0.4);
        col = mix(col, ink, hatch * rSh * 0.22 * chalk);
        col = mix(col, ink, clamp(lines * pressure * chalk * 1.15, 0.0, 1.0));
        col = mix(col, vec3(1.0), tip * 0.6);
    } else if (sk_style == 3) {
        col = mix(col, ink, clamp(lines * 1.05, 0.0, 1.0));
        col += vec3(0.6, 0.85, 1.0) * tip * 0.55;
    } else if (sk_style == 4) {
        float wash = max(tone, smoothstep(0.55, 0.95, dark) * 0.5) * rSh * (0.55 + 0.45 * sk_fbm(pos * 5.0));
        col = mix(col, vec3(0.42, 0.42, 0.44), wash * 0.45);
        col = mix(col, ink, clamp(lines * (0.85 + 0.3 * pressure), 0.0, 1.0));
        col = mix(col, ink, tip * 0.4);
    } else if (sk_style == 5) {
        float grit = smoothstep(0.15, 0.55, sk_noise(px / 1.4));
        float smudge = tone * rSh * (0.6 + 0.4 * sk_fbm(pos * 4.0));
        col = mix(col, ink, smudge * 0.55);
        col = mix(col, ink, clamp(lines * pressure * (0.6 + 0.5 * grit), 0.0, 1.0));
        col = mix(col, ink * 0.5, tip * 0.5);
    } else if (sk_style == 6) {
        col = mix(col, ink, hatch * rSh * 0.45);
        col = mix(col, ink, clamp(lines * pressure * 0.95, 0.0, 1.0));
        col = mix(col, ink * 0.6, tip * 0.6);
    } else {
        col = mix(col, ink, hatch * rSh * 0.32);
        col = mix(col, ink, clamp(lines * pressure * 0.88, 0.0, 1.0));
        col = mix(col, ink * 0.35, tip * 0.7);
    }
    if (q < 0.16) {
        vec3 old = texture(u_tex_old, uv).rgb;
        float cover = smoothstep(0.0, 0.1, q + (sk_fbm(pos * 3.0 + 9.1) - 0.5) * 0.06);
        col = mix(old, col, cover);
    }
    return col;
}
vec4 sk_final(vec2 uv) {
    return vec4(sk_frame(uv, 1.0), 1.0);
}
