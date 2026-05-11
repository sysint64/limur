// mix in sRGB space (browser/Photoshop default)
const COLOR_SPACE_SRGB: u32 = 0u;

// mix in linear light (physically correct, perceptually non-uniform)
const COLOR_SPACE_LINEAR: u32 = 3u;

const COLOR_SPACE_OK_LAB: u32 = 1u;
const COLOR_SPACE_OK_LCH: u32 = 2u;

const TWO_PI: f32 = 6.28318530717958647692;
const PI: f32 = 3.14159265358979323846;

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

    switch data.shape_type {
        case 0u: {
            return rect(data, p, half_size);
        }
        case 1u: {
            return oval(data, p, half_size);
        }
        case 2u: {
            return rect_outer_shadow(data, p, half_size);
        }
        case 3u: {
            return rect_inner_shadow(data, p, half_size);
        }
        case 4u: {
            return oval_outer_shadow(data, p, half_size);
        }
        case 5u: {
            return oval_inner_shadow(data, p, half_size);
        }
        default: {
            return vec4<f32>(0, 0, 0, 0);
        }
    }
}

fn oval(data: VectorData, p: vec2<f32>, half_size: vec2<f32>) -> vec4<f32> {
    let dist = sdf_oval(p, half_size);
    let alpha = oval_fill_mask(dist, p);

    return fill(data, p, half_size, alpha, 0.0);
}

fn oval_outer_shadow(data: VectorData, p: vec2<f32>, half_size: vec2<f32>) -> vec4<f32> {
    let blur_radius = data.box_shadow.z;
    let spread_radius = data.box_shadow.w;
    let offset = data.box_shadow.xy;
    let hs = half_size + spread_radius;

    if blur_radius == 0.0 {
        let dist = sdf_oval(p - offset, hs);
        let alpha = oval_fill_mask(dist, p - offset);

        return vec4<f32>(data.fill_color.rgb, data.fill_color.a * alpha);
    }

    let samples = select(4, 8, blur_radius < 10.0);
    let shadow = oval_box_shadow(
        p,
        half_size,
        blur_radius,
        offset,
        spread_radius,
        data.fill_color,
        samples,
    );

    return shadow;
}

fn oval_inner_shadow(data: VectorData, p: vec2<f32>, half_size: vec2<f32>) -> vec4<f32> {
    let blur_radius = data.box_shadow.z;
    let spread_radius = data.box_shadow.w;
    let offset = data.box_shadow.xy;

    // Clip to the oval boundary
    let dist = sdf_oval(p, half_size);
    let clip_alpha = oval_fill_mask(dist, p);

    if clip_alpha <= 0.0 {
        return vec4<f32>(0.0);
    }

    let inner_hs = max(half_size - spread_radius, vec2(0.0));

    if blur_radius == 0.0 {
        let inner_dist = sdf_oval(p - offset, inner_hs);
        let inner_fill = oval_fill_mask(inner_dist, p - offset);
        let shadow_alpha = (1.0 - inner_fill) * clip_alpha;

        return vec4<f32>(data.fill_color.rgb, data.fill_color.a * shadow_alpha);
    }

    let samples = select(4, 8, blur_radius < 10.0);
    let value = oval_box_shadow(
        p,
        inner_hs,
        blur_radius,
        offset,
        0.0,
        data.fill_color,
        samples,
    );

    let shadow_alpha = (1.0 - value.a / data.fill_color.a) * clip_alpha;

    return vec4<f32>(data.fill_color.rgb, data.fill_color.a * shadow_alpha);
}

fn rect(data: VectorData, p: vec2<f32>, half_size: vec2<f32>) -> vec4<f32> {
    let dist = sdf_rounded_rect(p, half_size, data.border_radii);
    let alpha = fill_mask(dist, p, half_size, data.border_radii);

    let border_left = data.border_widths.x;
    let border_top = data.border_widths.y;
    let border_right = data.border_widths.z;
    let border_bottom = data.border_widths.w;

    // Inner rect center shifts by the asymmetry in opposing border widths.
    let inner_center = vec2(
        (border_left - border_right) * 0.5,
        (border_bottom - border_top) * 0.5,
    );
    let inner_half = max(
        half_size - vec2(
            (border_left + border_right) * 0.5,
            (border_top + border_bottom) * 0.5,
        ),
        vec2(0.0),
    );

    let inner_radii = max(
        data.border_radii - vec4(
            min(border_top, border_left),
            min(border_top, border_right),
            min(border_bottom, border_right),
            min(border_bottom, border_left),
        ),
        vec4(0.0)
    );

    let inner_dist = sdf_rounded_rect(p - inner_center, inner_half, inner_radii);

    return fill(data, p, half_size, alpha, inner_dist);
}

fn fill(data: VectorData, p: vec2<f32>, half_size: vec2<f32>, alpha: f32, inner_dist: f32) -> vec4<f32> {
    let gradient_type = data.gradient_info.x;
    let fill_color = vec4<f32>(data.fill_color.rgb, data.fill_color.a * alpha);

    let inner_aa = 0.5 * fwidth(inner_dist);
    let border_fill_factor = 1.0 - smoothstep(-inner_aa, inner_aa, inner_dist);
    let border = add_border(data, p, half_size, inner_dist);

    if gradient_type == 0u {
        let result_color = mix_premultiplied(border, fill_color, border_fill_factor);

        return vec4<f32>(result_color.rgb, result_color.a * alpha);
    }

    let gradient_start_index = u32(data.gradient_info.y);
    let gradient_stop_count = u32(data.gradient_info.z);

    let size = data.boundary.zw;
    let uv = p / size + 0.5;

    var gradient_t: f32 = 0.0;

    switch gradient_type {
        // Linear gradient
        case 1u: {
            let start = data.gradient_params.xy;
            let end = data.gradient_params.zw;
            let direction = end - start;
            let range = dot(direction, direction);

            gradient_t = select(
                0.0,
                clamp(dot(uv - start, direction) / range, 0.0, 1.0),
                range > 0.0001
            );
        }
        // Radial gradient
        case 2u: {
            let center = data.gradient_params.xy;
            let radius = data.gradient_params.z;

            // make UV square
            let aspect = size.x / size.y;
            let d = (uv - center) * vec2(aspect, 1.0);
            // gradient_t = clamp(length(d) / max(radius * aspect, 0.0001), 0.0, 1.0);

            gradient_t = clamp(
                length(d) / max(radius * aspect, 0.0001),
                0.0,
                1.0
            );
        }
        // Sweep gradient
        case 3u: {
            let center = data.gradient_params.xy;
            let start_angle = data.gradient_params.z;
            let end_angle = data.gradient_params.w;

            let aspect = size.x / size.y;
            // multiply by size to measure angles in pixel space, not stretched UV space
            let d = (uv - center) * vec2(aspect, 1.0) * size;
            var angle = atan2(d.y, d.x);

            if angle < start_angle {
                angle += TWO_PI;
            }

            let range = end_angle - start_angle;

            gradient_t = select(
                0.0,
                clamp((angle - start_angle) / range, 0.0, 1.0),
                range > 0.0001
            );
        }
        default:  {
            return fill_color;
        }
    }

    let gradient_color = sample_gradient(
        gradient_start_index,
        gradient_stop_count,
        gradient_t,
        COLOR_SPACE_OK_LCH
    );

    let result_color = mix_premultiplied(border, gradient_color, border_fill_factor);

    // mix_stops always outputs linear RGB; compositor stores linear (Rgba16Float).
    return vec4<f32>(result_color.rgb, result_color.a * alpha);
}

fn add_border(data: VectorData, p: vec2<f32>, half_size: vec2<f32>, inner_dist: f32) -> vec4<f32> {
    // Normalize each edge distance by its border width so that the diagonal
    // boundary between two adjacent sides runs from the outer corner to the
    // inner corner (CSS-style). Sides with zero width get infinite normalized
    // distance and are never selected.
    let bw = data.border_widths; // [left, top, right, bottom] -> [x, y, z, w]

    let d_top    = half_size.y - p.y;
    let d_right  = half_size.x - p.x;
    let d_bottom = half_size.y + p.y;
    let d_left   = half_size.x + p.x;

    let inf = 1e9;
    let n_top    = select(inf, d_top    / bw.y, bw.y > 0.0);
    let n_right  = select(inf, d_right  / bw.z, bw.z > 0.0);
    let n_bottom = select(inf, d_bottom / bw.w, bw.w > 0.0);
    let n_left   = select(inf, d_left   / bw.x, bw.x > 0.0);

    // Track the winner (c0/n0) and runner-up (c1/n1) by normalized distance.
    var c0 = data.border_color_top;
    var n0 = n_top;
    var c1 = data.border_color_top;
    var n1 = inf;

    if n_right < n0 {
        c1 = c0; n1 = n0; c0 = data.border_color_right; n0 = n_right;
    } else if n_right < n1 {
        c1 = data.border_color_right; n1 = n_right;
    }
    if n_bottom < n0 {
        c1 = c0; n1 = n0; c0 = data.border_color_bottom; n0 = n_bottom;
    } else if n_bottom < n1 {
        c1 = data.border_color_bottom; n1 = n_bottom;
    }
    if n_left < n0 {
        c1 = c0; n1 = n0; c0 = data.border_color_left; n0 = n_left;
    } else if n_left < n1 {
        c1 = data.border_color_left; n1 = n_left;
    }

    // Anti-alias the boundary between the two nearest sides.
    // f = 0 at the boundary (n0 == n1), negative in winner territory.
    // blend goes from 0 (far from boundary, pure c0) to 0.5 (at boundary, 50/50).
    let f = n0 - n1;
    let fw = fwidth(f);
    let blend = smoothstep(-fw, 0.0, f) * 0.5;

    return mix_premultiplied(c0, c1, blend);
}

fn sample_gradient(start: u32, count: u32, t: f32, color_space: u32) -> vec4<f32> {
  for (var i = 0u; i < count - 1u; i += 1) {
    let a = gradient_stops[start + i];
    let b = gradient_stops[start + i + 1u];

    if t <= b.offset {
        let length = b.offset - a.offset;
        let f = select(0.0, (t - a.offset) / length, length > 0.0001);

        return mix_stops(a.color, b.color, f, color_space);
    }
  }

  return gradient_stops[start + count - 1u].color;
}

fn linear_to_srgb_channel(c: f32) -> f32 {
    if c <= 0.0031308 { return c * 12.92; }
    return 1.055 * pow(c, 1.0 / 2.4) - 0.055;
}

fn srgb_to_linear_channel(c: f32) -> f32 {
    if c <= 0.04045 { return c / 12.92; }
    return pow((c + 0.055) / 1.055, 2.4);
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    return vec3(srgb_to_linear_channel(c.r), srgb_to_linear_channel(c.g), srgb_to_linear_channel(c.b));
}

fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    return vec3(linear_to_srgb_channel(c.r), linear_to_srgb_channel(c.g), linear_to_srgb_channel(c.b));
}

fn mix_stops(a: vec4<f32>, b: vec4<f32>, f: f32, color_space: u32) -> vec4<f32> {
    var mixed: vec3<f32>;

    switch color_space {
        case COLOR_SPACE_LINEAR: {
            // mix in linear light - physically correct, perceptually non-uniform
            mixed = mix(a.rgb, b.rgb, f);
        }
        case COLOR_SPACE_SRGB, default: {
            // mix in sRGB space - matches CSS/browser default
            mixed = to_linear(mix(to_srgb(a.rgb), to_srgb(b.rgb), f));
        }
        case COLOR_SPACE_OK_LAB: {
            let a_lab = linear_to_oklab(a.rgb);
            let b_lab = linear_to_oklab(b.rgb);

            mixed = oklab_to_linear(mix(a_lab, b_lab, f));
        }
        case COLOR_SPACE_OK_LCH: {
            let a_lab = linear_to_oklab(a.rgb);
            let b_lab = linear_to_oklab(b.rgb);
            let a_lch = vec3(a_lab.x, length(a_lab.yz), atan2(a_lab.z, a_lab.y));
            let b_lch = vec3(b_lab.x, length(b_lab.yz), atan2(b_lab.z, b_lab.y));

            var dh = b_lch.z - a_lch.z;
            if dh >  PI { dh -= TWO_PI; }
            if dh < -PI { dh += TWO_PI; }

            let h   = a_lch.z + f * dh;
            let lch = vec3(mix(a_lch.x, b_lch.x, f), mix(a_lch.y, b_lch.y, f), h);
            mixed = oklab_to_linear(vec3(lch.x, lch.y * cos(lch.z), lch.y * sin(lch.z)));
        }
    }

    return vec4(mixed, mix(a.a, b.a, f));
}

// Approximate SDF for an ellipse
fn sdf_oval(p: vec2<f32>, half_size: vec2<f32>) -> f32 {
    let q = p / half_size;
    let d = length(q) - 1.0;

    return d * min(half_size.x, half_size.y);
}

fn oval_fill_mask(d: f32, p: vec2<f32>) -> f32 {
    let fw = length(fwidth(p));

    return smoothstep(fw * 0.5, -fw * 0.5, d);
}

fn oval_box_shadow(
    p: vec2<f32>,
    half_size: vec2<f32>,
    blur: f32,
    offset: vec2<f32>,
    spread: f32,
    color: vec4<f32>,
    samples: i32,
) -> vec4<f32> {
    let point = p - offset;
    let hs = half_size + spread;

    let low = point.y - hs.y;
    let high = point.y + hs.y;
    let start = clamp(-3.0 * blur, low, high);
    let end = clamp(3.0 * blur, low, high);

    let step = (end - start) / f32(samples);
    var y = start + step * 0.5;
    var value = 0.0;

    for (var i = 0; i < samples; i++) {
        value += oval_shadow_x(point.x, point.y - y, blur, hs)
               * gaussian(y, blur) * step;
        y += step;
    }

    return vec4(color.rgb, color.a * value);
}

fn oval_shadow_x(
    x: f32,
    y: f32,
    sigma: f32,
    half_size: vec2<f32>,
) -> f32 {
    // Ellipse half-width at height y: a * sqrt(1 - (y/b)^2)
    let t = y / half_size.y;
    let t2 = t * t;

    if t2 >= 1.0 {
        return 0.0;
    }

    let extent = half_size.x * sqrt(1.0 - t2);

    // Analytical gaussian integral from -extent to +extent
    let inv_sigma = sqrt(0.5) / sigma;
    let integral = 0.5 + 0.5 * erf_approx(vec2(
        (x - extent) * inv_sigma,
        (x + extent) * inv_sigma,
    ));

    return integral.y - integral.x;
}

fn rect_outer_shadow(data: VectorData, p: vec2<f32>, half_size: vec2<f32>) -> vec4<f32> {
    let blur_radius = data.box_shadow.z;
    let spread_radius = data.box_shadow.w;
    let offset = round(data.box_shadow.xy);
    let outer_radii = max(data.border_radii + vec4(spread_radius), vec4(0.0));

    if blur_radius == 0. {
        let dist = sdf_rounded_rect(p - offset, half_size + spread_radius, outer_radii);
        let fw = length(fwidth(p));
        let alpha = smoothstep(fw * 0.5, -fw * 0.5, dist);

        return vec4<f32>(data.fill_color.rgb, data.fill_color.a * alpha);
    }

    let samples = select(4, 8, blur_radius < 10.0);
    let shadow = box_shadow(
        p,
        half_size - vec2<f32>(0.5),
        outer_radii,
        blur_radius,
        offset,
        spread_radius,
        data.fill_color,
        samples,
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

        return vec4<f32>(data.fill_color.rgb, data.fill_color.a * shadow_alpha);
    }

    let samples = select(4, 8, blur_radius < 10.0);
    let value = box_shadow(
        p,
        max(inner_hs, vec2(0.0)),
        inner_radii,
        blur_radius,
        offset,
        0.0,
        data.fill_color,
        samples,
    );

    // Invert: where box_shadow is bright (inside the hole), we want no shadow.
    // Where it's dark (near/outside the hole edge), we want shadow.
    let shadow_alpha = (1.0 - value.a / data.fill_color.a) * clip_alpha;

    return vec4<f32>(data.fill_color.rgb, data.fill_color.a * shadow_alpha);
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
    return exp(-(x * x) / (2.0 * sigma * sigma)) / (sqrt(2.0 * PI) * sigma);
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

// Source: https://bottosson.github.io/posts/oklab/
fn linear_to_oklab(c: vec3<f32>) -> vec3<f32> {
    let l = 0.4122214708 * c.r + 0.5363325363 * c.g + 0.0514459929 * c.b;
    let m = 0.2119034982 * c.r + 0.6806995451 * c.g + 0.1073969566 * c.b;
    let s = 0.0883024619 * c.r + 0.2817188376 * c.g + 0.6299787005 * c.b;

    let l_ = sign(l) * pow(abs(l), 1.0 / 3.0);
    let m_ = sign(m) * pow(abs(m), 1.0 / 3.0);
    let s_ = sign(s) * pow(abs(s), 1.0 / 3.0);

    return vec3(
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    );
}

/// Source: https://bottosson.github.io/posts/oklab/
fn oklab_to_linear(lab: vec3<f32>) -> vec3<f32> {
    let l_ = lab.x + 0.3963377774 * lab.y + 0.2158037573 * lab.z;
    let m_ = lab.x - 0.1055613458 * lab.y - 0.0638541728 * lab.z;
    let s_ = lab.x - 0.0894841775 * lab.y - 1.2914855480 * lab.z;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    return clamp(vec3(
         4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    ), vec3(0.0), vec3(1.0));
}

fn mix_premultiplied(a: vec4<f32>, b: vec4<f32>, t: f32) -> vec4<f32> {
    let a_pm = vec4<f32>(a.rgb * a.a, a.a);
    let b_pm = vec4<f32>(b.rgb * b.a, b.a);
    let m = mix(a_pm, b_pm, t);

    return select(
        vec4<f32>(0.0),
        vec4<f32>(m.rgb / m.a, m.a),
        m.a > 0.001
    );
}
