struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position:  vec4<f32>,
    @location(0)  coord:               vec2<f32>,
    @location(1)  fill_color:          vec4<f32>,
    @location(2)  border_color_top:    vec4<f32>,
    @location(3)  border_color_right:  vec4<f32>,
    @location(4)  border_color_bottom: vec4<f32>,
    @location(5)  border_color_left:   vec4<f32>,
    @location(6)  border_widths:       vec4<f32>,
    @location(7)  border_radii:        vec4<f32>,
    @location(8)  size_and_grad:       vec4<f32>,
    @location(9)  gradient_p0:         vec4<f32>,
    @location(10) gradient_s0:         vec4<f32>,
    @location(11) gradient_s1:         vec4<f32>,
    @location(12) gradient_s2:         vec4<f32>,
    @location(13) gradient_s3:         vec4<f32>,
    @location(14) gradient_s4:         vec4<f32>,
    @location(15) gradient_s5:         vec4<f32>,
    @location(16) gradient_s6:         vec4<f32>,
    @location(17) gradient_s7:         vec4<f32>,
};

struct InstanceInput {
    @location(5)  mvp_0:               vec4<f32>,
    @location(6)  mvp_1:               vec4<f32>,
    @location(7)  mvp_2:               vec4<f32>,
    @location(8)  mvp_3:               vec4<f32>,
    @location(9)  fill_color:          vec4<f32>,
    @location(10) border_color_top:    vec4<f32>,
    @location(11) border_color_right:  vec4<f32>,
    @location(12) border_color_bottom: vec4<f32>,
    @location(13) border_color_left:   vec4<f32>,
    @location(14) border_widths:       vec4<f32>,  // top, right, bottom, left
    @location(15) border_radii:        vec4<f32>,  // top_left, top_right, bottom_right, bottom_left
    @location(16) size_and_grad:       vec4<f32>,  // [width, height, gradient_type, shape_type]
    @location(17) gradient_p0:         vec4<f32>,  // linear:[sx,sy,ex,ey]  radial:[cx,cy,r,0]
    @location(18) gradient_s0:         vec4<f32>,
    @location(19) gradient_s1:         vec4<f32>,
    @location(20) gradient_s2:         vec4<f32>,
    @location(21) gradient_s3:         vec4<f32>,
    @location(22) gradient_s4:         vec4<f32>,
    @location(23) gradient_s5:         vec4<f32>,
    @location(24) gradient_s6:         vec4<f32>,
    @location(25) gradient_s7:         vec4<f32>,
};

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        return c / 12.92;
    } else {
        return pow((c + 0.055) / 1.055, 2.4);
    }
}

// Signed distance field for a rounded rect with per-corner radii.
// p.y > 0 = UI top (y is flipped in the MVP matrix).
// radii: [top_left, top_right, bottom_right, bottom_left]
fn sdf_rounded_rect(p: vec2<f32>, half_size: vec2<f32>, radii: vec4<f32>) -> f32 {
    var r: f32;
    if p.x >= 0.0 {
        r = select(radii.z, radii.y, p.y >= 0.0);
    } else {
        r = select(radii.w, radii.x, p.y >= 0.0);
    }
    let q = abs(p) - half_size + r;
    return length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

// Approximate SDF for an ellipse. Negative inside, positive outside.
fn sdf_ellipse(p: vec2<f32>, r: vec2<f32>) -> f32 {
    let k = length(p / r);
    return (k - 1.0) * min(r.x, r.y) / max(k, 0.0001);
}

// Lerp between 8 pre-baked gradient samples.
fn sample_baked(
    t: f32,
    s0: vec4<f32>, s1: vec4<f32>, s2: vec4<f32>, s3: vec4<f32>,
    s4: vec4<f32>, s5: vec4<f32>, s6: vec4<f32>, s7: vec4<f32>,
) -> vec4<f32> {
    let i_f = clamp(t, 0.0, 1.0) * 7.0;
    let i   = i32(i_f);
    let f   = i_f - f32(i);
    switch i {
        case 0:          { return mix(s0, s1, f); }
        case 1:          { return mix(s1, s2, f); }
        case 2:          { return mix(s2, s3, f); }
        case 3:          { return mix(s3, s4, f); }
        case 4:          { return mix(s4, s5, f); }
        case 5:          { return mix(s5, s6, f); }
        case 6, default: { return mix(s6, s7, f); }
    }
}

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
    let mvp = mat4x4<f32>(instance.mvp_0, instance.mvp_1, instance.mvp_2, instance.mvp_3);
    var out: VertexOutput;
    out.coord               = model.position.xy;
    out.fill_color          = instance.fill_color;
    out.border_color_top    = instance.border_color_top;
    out.border_color_right  = instance.border_color_right;
    out.border_color_bottom = instance.border_color_bottom;
    out.border_color_left   = instance.border_color_left;
    out.border_widths       = instance.border_widths;
    out.border_radii        = instance.border_radii;
    out.size_and_grad       = instance.size_and_grad;
    out.gradient_p0         = instance.gradient_p0;
    out.gradient_s0         = instance.gradient_s0;
    out.gradient_s1         = instance.gradient_s1;
    out.gradient_s2         = instance.gradient_s2;
    out.gradient_s3         = instance.gradient_s3;
    out.gradient_s4         = instance.gradient_s4;
    out.gradient_s5         = instance.gradient_s5;
    out.gradient_s6         = instance.gradient_s6;
    out.gradient_s7         = instance.gradient_s7;
    out.clip_position       = mvp * vec4<f32>(model.position, 1.0);
    return out;
}

// ── Clip AA entry point ────────────────────────────────────────────────────────
// Used with blend=(Zero, SrcAlpha) on a non-MSAA pass targeting layer_view.
// Outputs outer_alpha so the destination is multiplied by 1 inside and 0 outside
// the clip shape, giving smooth anti-aliased clip boundaries.
@fragment
fn fs_clip_aa(in: VertexOutput) -> @location(0) vec4<f32> {
    let size       = in.size_and_grad.xy;
    let shape_type = i32(in.size_and_grad.w);
    let half_size  = size * 0.5;
    let p          = (in.coord - 0.5) * size;

    var outer_dist: f32;
    if shape_type == 1 {
        outer_dist = sdf_ellipse(p, half_size);
    } else {
        outer_dist = sdf_rounded_rect(p, half_size, in.border_radii);
    }
    let outer_aa    = 0.5 * fwidth(outer_dist);
    let outer_alpha = 1.0 - smoothstep(-outer_aa, outer_aa, outer_dist);
    if outer_alpha <= 0.0 { discard; }
    // rgb=0 (src_factor=Zero ignores it); alpha=outer_alpha multiplies destination.
    return vec4(0.0, 0.0, 0.0, outer_alpha);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let size          = in.size_and_grad.xy;
    let gradient_type = i32(in.size_and_grad.z);
    let shape_type    = i32(in.size_and_grad.w);

    let half_size = size * 0.5;
    // coord in [0, 1] from PlaneResources; p is in pixel space centered at origin.
    let p  = (in.coord - 0.5) * size;

    // uv in [0, 1] for gradient evaluation (matches limur's normalized convention).
    // coord.y = 1.0 is UI top (after the y-flip in the MVP), so uv.y must be inverted.
    let uv = vec2(in.coord.x, 1.0 - in.coord.y);

    // ── Outer SDF ──────────────────────────────────────────────────────────────
    var outer_dist: f32;
    if shape_type == 1 {
        outer_dist = sdf_ellipse(p, half_size);
    } else {
        outer_dist = sdf_rounded_rect(p, half_size, in.border_radii);
    }
    let outer_aa    = 0.5 * fwidth(outer_dist);
    let outer_alpha = 1.0 - smoothstep(-outer_aa, outer_aa, outer_dist);
    if outer_alpha <= 0.0 { discard; }

    // ── Inner (fill) SDF ───────────────────────────────────────────────────────
    // border_widths: top(x), right(y), bottom(z), left(w).  p.y > 0 = UI top.
    // Inner rect center shifts by the asymmetry in opposing border widths.
    let inner_center = vec2(
        (in.border_widths.w - in.border_widths.y) * 0.5,   // (left - right) / 2
        (in.border_widths.z - in.border_widths.x) * 0.5,   // (bottom - top) / 2
    );
    let inner_half = max(
        half_size - vec2(
            (in.border_widths.y + in.border_widths.w) * 0.5,
            (in.border_widths.x + in.border_widths.z) * 0.5,
        ),
        vec2(0.0),
    );

    var inner_dist: f32;
    if shape_type == 1 {
        // Ellipse: uniform border — just shrink the radii.
        let inner_r = max(half_size - vec2(in.border_widths.x), vec2(0.0));
        inner_dist = sdf_ellipse(p, max(inner_r, vec2(0.001)));
    } else {
        // Per-corner radius shrinks by the adjacent border widths.
        let inner_radii = max(in.border_radii - vec4(
            min(in.border_widths.x, in.border_widths.w),   // top_left:     min(top, left)
            min(in.border_widths.x, in.border_widths.y),   // top_right:    min(top, right)
            min(in.border_widths.z, in.border_widths.y),   // bottom_right: min(bottom, right)
            min(in.border_widths.z, in.border_widths.w),   // bottom_left:  min(bottom, left)
        ), vec4(0.0));
        inner_dist = sdf_rounded_rect(p - inner_center, inner_half, inner_radii);
    }

    let inner_aa    = 0.5 * fwidth(inner_dist);
    let fill_factor = 1.0 - smoothstep(-inner_aa, inner_aa, inner_dist);

    // ── Fill color (sRGB) ──────────────────────────────────────────────────────
    var fill: vec4<f32>;
    if gradient_type == 1 {
        // Linear gradient: gradient_p0 = [start.x, start.y, end.x, end.y] (normalized UV).
        let g_start = in.gradient_p0.xy;
        let g_end   = in.gradient_p0.zw;
        let dir     = g_end - g_start;
        let len2    = dot(dir, dir);
        var t: f32;
        if len2 < 0.0001 { t = 0.0; } else { t = clamp(dot(uv - g_start, dir) / len2, 0.0, 1.0); }
        fill = sample_baked(t,
            in.gradient_s0, in.gradient_s1, in.gradient_s2, in.gradient_s3,
            in.gradient_s4, in.gradient_s5, in.gradient_s6, in.gradient_s7);
    } else if gradient_type == 2 {
        // Radial gradient: gradient_p0 = [center.x, center.y, radius, 0] (normalized UV).
        let g_center = in.gradient_p0.xy;
        let g_radius = in.gradient_p0.z;
        let t = clamp(length(uv - g_center) / max(g_radius, 0.0001), 0.0, 1.0);
        fill = sample_baked(t,
            in.gradient_s0, in.gradient_s1, in.gradient_s2, in.gradient_s3,
            in.gradient_s4, in.gradient_s5, in.gradient_s6, in.gradient_s7);
    } else {
        // Solid color (or transparent for Fill::None).
        fill = in.fill_color;
    }

    // ── Border color: nearest edge wins (sRGB) ─────────────────────────────────
    let d_top    = half_size.y - p.y;
    let d_right  = half_size.x - p.x;
    let d_bottom = half_size.y + p.y;
    let d_left   = half_size.x + p.x;

    var border = in.border_color_top;
    let best_tr = select(in.border_color_right, border,   d_top    <= d_right);
    let best_d  = min(d_top, d_right);
    let best_tb = select(in.border_color_bottom, best_tr, best_d   <= d_bottom);
    let best_d2 = min(best_d, d_bottom);
    border      = select(in.border_color_left, best_tb,   best_d2  <= d_left);

    // ── Blend in linear space ──────────────────────────────────────────────────
    // Convert both colors to linear before blending so the AA transition at the
    // fill/border junction is perceptually correct and doesn't create a bright fringe.
    let border_l = vec4(srgb_to_linear(border.r), srgb_to_linear(border.g), srgb_to_linear(border.b), border.a);
    let fill_l   = vec4(srgb_to_linear(fill.r),   srgb_to_linear(fill.g),   srgb_to_linear(fill.b),   fill.a);
    let color    = mix(border_l, fill_l, fill_factor);

    return vec4(color.rgb, color.a * outer_alpha);
}
