// Backdrop filter: samples the composite texture (everything behind this layer)
// and applies a tint over the rect region.
//
// Uses the same plane geometry + instancing pattern as the vector renderer.
// Slot 0: TexturedVertex (locations 0-1)
// Slot 1: BackdropFilterInstance (locations 2-6)

@group(0) @binding(0) var composite_tex: texture_2d<f32>;
@group(0) @binding(1) var composite_sampler: sampler;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct Instance {
    @location(2) mvp_col0: vec4<f32>,
    @location(3) mvp_col1: vec4<f32>,
    @location(4) mvp_col2: vec4<f32>,
    @location(5) mvp_col3: vec4<f32>,
    @location(6) tint: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) composite_uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
}

@vertex
fn vs_main(v: Vertex, inst: Instance) -> VertexOutput {
    let mvp = mat4x4(inst.mvp_col0, inst.mvp_col1, inst.mvp_col2, inst.mvp_col3);
    let clip = mvp * vec4(v.position, 1.0);

    // Derive UV into the composite texture from NDC clip position.
    // Clip Y is up (+1 = top); texture Y is down (0 = top) → flip.
    let uv = vec2((clip.x + 1.0) * 0.5, (1.0 - clip.y) * 0.5);

    var out: VertexOutput;
    out.clip_position = clip;
    out.composite_uv  = uv;
    out.tint          = inst.tint;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSample(composite_tex, composite_sampler, in.composite_uv);
    // Premultiplied alpha-over: tint on top of composite sample.
    return in.tint + base * (1.0 - in.tint.a);
}
