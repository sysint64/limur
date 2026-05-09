struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) coord: vec2<f32>,
    @location(1) @interpolate(flat) instance_id: u32,
};

struct InstanceInput {
    @location(2) mvp_matrix_0: vec4<f32>,
    @location(3) mvp_matrix_1: vec4<f32>,
    @location(4) mvp_matrix_2: vec4<f32>,
    @location(5) mvp_matrix_3: vec4<f32>,
};

struct VectorData {
    boundary: vec4<f32>,
    shape_type: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    fill_color: vec4<f32>,
    border_color_left: vec4<f32>,
    border_color_top: vec4<f32>,
    border_color_right: vec4<f32>,
    border_color_bottom: vec4<f32>,
    border_widths: vec4<f32>,
    border_radii: vec4<f32>,
    box_shadow: vec4<f32>,
    gradient_info: vec4<u32>,
    gradient_params: vec4<f32>,
};

struct GradientStop {
    color: vec4<f32>,
    offset: f32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var<storage, read> shape_data: array<VectorData>;
@group(0) @binding(1) var<storage, read> gradient_stops: array<GradientStop>;

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
    @builtin(instance_index) instance_idx: u32,
) -> VertexOutput {
    let mvp_matrix = mat4x4<f32>(
        instance.mvp_matrix_0,
        instance.mvp_matrix_1,
        instance.mvp_matrix_2,
        instance.mvp_matrix_3,
    );

    var out: VertexOutput;

    out.clip_position = mvp_matrix * vec4<f32>(model.position, 1.0);
    out.coord = model.position.xy;
    out.instance_id = instance_idx;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let data = shape_data[in.instance_id];
    let size = data.boundary.zw;
    let screen_position = data.boundary.xy;
    let half_size = size * 0.5;
    let p = in.clip_position.xy - screen_position - half_size;

    switch (data.shape_type) {
        case 0u: {
            return rect(data, p, half_size);
        }
        case 2u: {
            return rect_outer_shadow(data, p, half_size);
        }
        case 3u: {
            return rect_inner_shadow(data, p, half_size);
        }
        default: {
            return vec4<f32>(0, 0, 0, 0);
        }
    }
}

fn rect(data: VectorData, p: vec2<f32>, half_size: vec2<f32>) -> vec4<f32> {
    let dist = sdf_rounded_rect(p, half_size, data.border_radii);
    let alpha = fill_mask(dist, p, half_size, data.border_radii);

    return vec4<f32>(data.fill_color.rgb * alpha, data.fill_color.a * alpha);
}

fn rect_outer_shadow(data: VectorData, p: vec2<f32>, half_size: vec2<f32>) -> vec4<f32> {
    let blur_radius = data.box_shadow.z;
    let spread_radius = data.box_shadow.w;
    let offset = data.box_shadow.xy;
    let outer_radii = max(data.border_radii + vec4(spread_radius), vec4(0.0));

    if blur_radius == 0. {
        let dist = sdf_rounded_rect(p - offset, half_size + spread_radius, outer_radii);
        let alpha = fill_mask(dist, p, half_size + spread_radius, outer_radii);

        return vec4<f32>(data.fill_color.rgb * alpha, data.fill_color.a * alpha);
    }

    let shadow = box_shadow(
        p,
        half_size - vec2<f32>(0.5),
        outer_radii,
        blur_radius,
        offset,
        spread_radius,
        data.fill_color,
        4,
    );

    return shadow;
}

fn rect_inner_shadow(data: VectorData, p: vec2<f32>, half_size: vec2<f32>) -> vec4<f32> {
    let blur_radius = data.box_shadow.z;
    let spread_radius = data.box_shadow.w;
    let offset = data.box_shadow.xy;

    // Clip to the rect boundary so shadow doesn't bleed outside
    let dist = sdf_rounded_rect(p, half_size, data.border_radii);
    let clip_alpha = fill_mask(dist, p, half_size, data.border_radii);

    if clip_alpha <= 0.0 {
        return vec4<f32>(0.0);
    }

    // For inner shadow, we conceptually have a "hole" that is the rect
    // shrunk by spread, offset by the shadow offset. The shadow is the
    // blurred edge of that hole, visible only inside the original rect.
    //
    // shadow_alpha = 1 - box_shadow_value_of_shrunk_rect
    // This gives 0 in the center (fully lit) and 1 near the edges (shadowed).

    let inner_hs = half_size - spread_radius;

    // Adjust radii for the inset - keep corners concentric
    let inner_radii = max(data.border_radii - vec4(spread_radius), vec4(0.0));

    if blur_radius == 0.0 {
        // Hard inner shadow: just check if we're outside the shrunk rect
        let inner_dist = sdf_rounded_rect(p - offset, max(inner_hs, vec2(0.0)), inner_radii);
        let inner_fill = fill_mask(inner_dist, p - offset, max(inner_hs, vec2(0.0)), inner_radii);
        let shadow_alpha = (1.0 - inner_fill) * clip_alpha;

        return vec4<f32>(data.fill_color.rgb * shadow_alpha, data.fill_color.a * shadow_alpha);
    }

    let value = box_shadow(
        p,
        max(inner_hs, vec2(0.0)),
        inner_radii,
        blur_radius,
        offset,
        0.0,
        data.fill_color,
        4,
    );

    // Invert: where box_shadow is bright (inside the hole), we want no shadow.
    // Where it's dark (near/outside the hole edge), we want shadow.
    let shadow_alpha = (1.0 - value.a / data.fill_color.a) * clip_alpha;

    return vec4<f32>(data.fill_color.rgb * shadow_alpha, data.fill_color.a * shadow_alpha);
}

// Based on vger-rs: https://github.com/audulus/vger-rs/tree/main
fn box_shadow(
    p: vec2<f32>,
    half_size: vec2<f32>,
    radii: vec4<f32>,
    blur: f32,
    offset: vec2<f32>,
    spread: f32,
    color: vec4<f32>,
    // (4 = fast, 8 = smooth)
    samples: i32,
) -> vec4<f32> {
    let point = p - offset;
    let hs = half_size + spread;

    // Clamp vertical integration range to +-3 sigma
    let low = point.y - hs.y;
    let high = point.y + hs.y;
    let start = clamp(-3.0 * blur, low, high);
    let end = clamp(3.0 * blur, low, high);

    let step = (end - start) / f32(samples);
    var y = start + step * 0.5;
    var value = 0.0;

    for (var i = 0; i < samples; i++) {
        value += rounded_box_shadow_x(point.x, point.y - y, blur, hs, radii)
               * gaussian(y, blur) * step;
        y += step;
    }

    return vec4(color.rgb, color.a * value);
}

fn gaussian(x: f32, sigma: f32) -> f32 {
    let pi = 3.141592653589793;

    return exp(-(x * x) / (2.0 * sigma * sigma)) / (sqrt(2.0 * pi) * sigma);
}

fn rounded_box_shadow_x(
    x: f32,
    y: f32,
    sigma: f32,
    half_size: vec2<f32>,
    radii: vec4<f32>,
) -> f32 {
    // Pick the correct radius for this side (top vs bottom)
    let r_left = select(radii.w, radii.x, y >= 0.0);
    let r_right = select(radii.z, radii.y, y >= 0.0);

    // Left edge extent at height y
    let delta_left = min(half_size.y - r_left - abs(y), 0.0);
    let curved_left = half_size.x - r_left + sqrt(max(0.0, r_left * r_left - delta_left * delta_left));

    // Right edge extent at height y
    let delta_right = min(half_size.y - r_right - abs(y), 0.0);
    let curved_right = half_size.x - r_right + sqrt(max(0.0, r_right * r_right - delta_right * delta_right));

    // Analytical gaussian integral from -curved_left to +curved_right
    let inv_sigma = sqrt(0.5) / sigma;
    let integral = 0.5 + 0.5 * erf_approx(vec2(
        (x - curved_right) * inv_sigma,
        (x + curved_left) * inv_sigma,
    ));

    return integral.y - integral.x;
}

fn erf_approx(x: vec2<f32>) -> vec2<f32> {
    let s = sign(x);
    let a = abs(x);
    var y = 1.0 + (0.278393 + (0.230389 + 0.078108 * (a * a)) * a) * a;
    y *= y;

    return s - s / (y * y);
}

// Signed distance field for a rounded rect with per-corner radii.
// p.y > 0 = UI top (y is flipped in the MVP matrix).
// radii: [top_left, top_right, bottom_right, bottom_left]
fn sdf_rounded_rect(p: vec2<f32>, half_size: vec2<f32>, radii: vec4<f32>) -> f32 {
    var cr = radii;

    let any_nonzero = any(radii > vec4(0.0));
    if any_nonzero {
        let size = half_size * 2.0;
        let s = min(1.0, min(
            min(size.x / (radii.x + radii.y), size.x / (radii.z + radii.w)),
            min(size.y / (radii.x + radii.w), size.y / (radii.y + radii.z))
        ));
        cr = radii * s;
    }

    let rx = select(
        select(cr.w, cr.x, p.y >= 0.0),
        select(cr.z, cr.y, p.y >= 0.0),
        p.x >= 0.0
    );

    let q = abs(p) - half_size + rx;

    return length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - rx;
}

fn fill_mask(d: f32, p: vec2<f32>, half_size: vec2<f32>, radii: vec4<f32>) -> f32 {
    let has_radius = any(radii > vec4(0.0));

    if !has_radius {
        return select(0.0, 1.0, d <= 0.0);
    }

    let fw = length(fwidth(p));

    // Pick the radius for the quadrant p is in
    let r = select(
        select(radii.z, radii.y, p.y >= 0.0),
        select(radii.w, radii.x, p.y >= 0.0),
        p.x < 0.0
    );

    // How far into the corner zone on each axis
    let cx = abs(p.x) - (half_size.x - r);
    let cy = abs(p.y) - (half_size.y - r);

    // Smoothly ramp from hard edge to AA over ~1 pixel
    let corner_blend = smoothstep(0.0, fw, min(cx, cy));

    let hard = select(0.0, 1.0, d <= 0.0);
    let soft = smoothstep(fw * 0.5, -fw * 0.5, d);

    return mix(hard, soft, corner_blend);
}
