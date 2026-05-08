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
    // let size = data.boundary.zw;
    // let half_size = size * 0.5;
    // let p = (in.coord - 0.5) * size;

    return data.fill_color;
}
