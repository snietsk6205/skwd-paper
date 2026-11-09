// Custom build (M3): composes two transitions rendered offscreen.
// luma_a = the first transition's frame (in "mask" mode: its black and white
// mask), chroma_a = the second transition's frame, texB = the old wallpaper.
// u_style = mode (1 blend, 2 split, 3 mask, 4 sweep, 5 light, 6 dark),
// u_pid_base = split shape (0 left/right, 1 top/bottom, 2 diagonal,
// 3 circle, 4 checker), u_progress = linear progress.
float combo_split(vec2 uv, int shape) {
    vec2 q = vec2(uv.x * 1.7777, uv.y);
    if (shape == 1) {
        return smoothstep(-0.004, 0.004, uv.y - 0.5);
    }
    if (shape == 2) {
        return smoothstep(-0.004, 0.004, (q.x - 0.8888) * 0.49 + (uv.y - 0.5) * 0.87);
    }
    if (shape == 3) {
        vec2 c = fx_origin(vec2(0.5));
        return smoothstep(-0.004, 0.004, length((uv - c) * vec2(1.7777, 1.0)) - 0.42);
    }
    if (shape == 4) {
        vec2 cell = floor(q * 3.0);
        return mod(cell.x + cell.y, 2.0);
    }
    return smoothstep(-0.004, 0.004, uv.x - 0.5);
}
float combo_seam(vec2 uv, int shape) {
    float s = combo_split(uv, shape);
    return s * (1.0 - s) * 4.0;
}
void main() {
    float t = clamp(u_progress, 0.0, 1.0);
    vec3 x = texture(luma_a, v_uv).rgb;
    vec3 y = texture(chroma_a, v_uv).rgb;
    vec3 col = x;
    if (u_style == 1) {
        col = mix(x, y, smoothstep(0.15, 0.85, t));
    } else if (u_style == 2) {
        col = mix(x, y, combo_split(v_uv, u_pid_base));
        col += vec3(0.9) * combo_seam(v_uv, u_pid_base) * sin(t * 3.14159) * 0.35;
    } else if (u_style == 3) {
        vec3 old = texB(v_uv).rgb;
        float m = clamp(dot(x, vec3(0.299, 0.587, 0.114)), 0.0, 1.0);
        col = mix(old, y, m);
    } else if (u_style == 4) {
        float edge = t * 1.3 - 0.15;
        float d = v_uv.x * 0.85 + v_uv.y * 0.15 - edge;
        col = mix(x, y, smoothstep(0.08, -0.08, d));
    } else if (u_style == 5) {
        col = max(x, y);
    } else if (u_style == 6) {
        col = min(x, y);
    }
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}
