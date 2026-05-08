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
    box_shadow_style: u32,
    _pad0: u32,
    _pad1: u32,
    fill_color: vec4<f32>,
    border_color_left: vec4<f32>,
    border_color_top: vec4<f32>,
    border_color_right: vec4<f32>,
    border_color_bottom: vec4<f32>,
    border_widths: vec4<f32>,
    border_radii: vec4<f32>,
    box_shadow: vec4<f32>,
    box_shadow_color: vec4<f32>,
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
            return draw_rect(data, p, half_size);
        }
        default: {
            return vec4<f32>(0, 0, 0, 0);
        }
    }
}

fn draw_rect(data: VectorData, p: vec2<f32>, half_size: vec2<f32>) -> vec4<f32> {
    let dist = sdf_rounded_rect(p, half_size, data.border_radii);
    let alpha = fill_mask(dist, p);

    return vec4<f32>(data.fill_color.rgb * alpha, data.fill_color.a * alpha);
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

fn fill_mask(d: f32, p: vec2<f32>) -> f32 {
    let fw = length(fwidth(p));

    return smoothstep(fw * 0.5, -fw * 0.5, d);
}
