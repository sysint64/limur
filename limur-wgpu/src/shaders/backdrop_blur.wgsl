// Backdrop filter with Gaussian blur.
//
// Samples the composite texture (everything behind this layer) with a
// separable 2D Gaussian kernel, then composites a tint on top.
//
// No vertex buffer — corner is derived analytically from vertex_index.
// Slot 0 instance attributes (locations 0-2):
//   0: boundary   [left, top, width, height] in pixel space (y-down)
//   1: tint        premultiplied RGBA
//   2: blur_params [blur_radius, 0, 0, 0]

@group(0) @binding(0) var composite_tex: texture_2d<f32>;
@group(0) @binding(1) var composite_sampler: sampler;

struct Globals {
    screen_size: vec2<f32>,
}
@group(0) @binding(2) var<uniform> globals: Globals;

struct Instance {
    @location(0) boundary:    vec4<f32>,  // [left, top, width, height]
    @location(1) tint:        vec4<f32>,  // premultiplied RGBA
    @location(2) blur_params: vec4<f32>,  // x = blur_radius in pixels
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) composite_uv:  vec2<f32>,
    @location(1) tint:          vec4<f32>,
    @location(2) blur_params:   vec4<f32>,
    @location(3) boundary_uv:   vec4<f32>, // [u_left, v_top, u_right, v_bottom]
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, inst: Instance) -> VertexOutput {
    let corner = vec2<f32>(f32(vertex_index & 1u), f32((vertex_index >> 1u) & 1u));
    let px = inst.boundary.xy + corner * inst.boundary.zw;
    let clip = vec4<f32>(
        px.x / globals.screen_size.x * 2.0 - 1.0,
        1.0 - px.y / globals.screen_size.y * 2.0,
        0.0,
        1.0,
    );
    let uv = vec2<f32>((clip.x + 1.0) * 0.5, (1.0 - clip.y) * 0.5);

    var out: VertexOutput;
    out.clip_position = clip;
    out.composite_uv  = uv;
    out.tint          = inst.tint;
    out.blur_params   = inst.blur_params;
    out.boundary_uv   = vec4<f32>(
        inst.boundary.x / globals.screen_size.x,
        inst.boundary.y / globals.screen_size.y,
        (inst.boundary.x + inst.boundary.z) / globals.screen_size.x,
        (inst.boundary.y + inst.boundary.w) / globals.screen_size.y,
    );
    return out;
}

fn gaussian_weight(offset: f32, sigma: f32) -> f32 {
    return exp(-(offset * offset) / (2.0 * sigma * sigma));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let radius = i32(in.blur_params.x);

    // Fast path: no blur, just tint.
    if radius <= 0 {
        let base = textureSampleLevel(composite_tex, composite_sampler, in.composite_uv, 0.0);
        return in.tint + base * (1.0 - in.tint.a);
    }

    let dims    = vec2<f32>(textureDimensions(composite_tex));
    let texel   = 1.0 / dims;
    let sigma   = f32(radius) * 0.5;
    let uv_min  = in.boundary_uv.xy;
    let uv_max  = in.boundary_uv.zw;

    var color        = vec4<f32>(0.0);
    var total_weight = 0.0;

    for (var dy = -radius; dy <= radius; dy++) {
        let wy = gaussian_weight(f32(dy), sigma);
        for (var dx = -radius; dx <= radius; dx++) {
            let w  = wy * gaussian_weight(f32(dx), sigma);
            let s  = clamp(
                in.composite_uv + vec2<f32>(f32(dx), f32(dy)) * texel,
                uv_min, uv_max,
            );
            color        += textureSampleLevel(composite_tex, composite_sampler, s, 0.0) * w;
            total_weight += w;
        }
    }

    let blurred = color / total_weight;
    return in.tint + blurred * (1.0 - in.tint.a);
}
