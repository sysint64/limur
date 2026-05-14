// Backdrop filter: samples the composite texture (everything behind this layer)
// and applies a tint over the rect region.
//
// No vertex buffer — corner is derived analytically from vertex_index.
// Slot 0: BackdropFilterInstance (locations 0-1)

@group(0) @binding(0) var composite_tex: texture_2d<f32>;
@group(0) @binding(1) var composite_sampler: sampler;
struct Globals {
    screen_size: vec2<f32>,
}

@group(0) @binding(2) var<uniform> globals: Globals;

struct Instance {
    @location(0) boundary: vec4<f32>, // [left, top, width, height] in top-left pixel space
    @location(1) tint: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) composite_uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, inst: Instance) -> VertexOutput {
    // Derive corner factor from vertex_index: (0,0) TL, (1,0) TR, (0,1) BL, (1,1) BR.
    let corner = vec2<f32>(f32(vertex_index & 1u), f32((vertex_index >> 1u) & 1u));
    let px = inst.boundary.xy + corner * inst.boundary.zw;
    let clip = vec4<f32>(
        px.x / globals.screen_size.x * 2.0 - 1.0,
        1.0 - px.y / globals.screen_size.y * 2.0,
        0.0,
        1.0,
    );

    // Derive UV into the composite texture from NDC clip position.
    // Clip Y is up (+1 = top); texture Y is down (0 = top) → flip.
    let uv = vec2<f32>((clip.x + 1.0) * 0.5, (1.0 - clip.y) * 0.5);

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
