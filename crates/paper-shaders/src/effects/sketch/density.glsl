// Low-resolution line-density map for the "busiest area" start point.
// sk_tex0 = analysis A (outline, detail, ...); this pass renders into a small
// target whose size is in u_res, so every output texel averages one cell.
void main() {
    vec2 cell = 1.0 / sk_res();
    float total = 0.0;
    for (int j = 0; j < 4; j++) {
        for (int i = 0; i < 4; i++) {
            vec2 p = v_uv + (vec2(float(i), float(j)) - 1.5) * 0.25 * cell;
            vec4 a = texture(sk_tex0, clamp(p, vec2(0.0), vec2(1.0)));
            total += a.r + 0.6 * a.g;
        }
    }
    frag = vec4(clamp(total / 16.0, 0.0, 1.0), 0.0, 0.0, 1.0);
}
