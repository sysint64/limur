// Full-screen triangle blit: copies src_texture to the current render target.
// No vertex buffer — positions are generated from vertex_index.
// Blend mode is configured on the pipeline, not in the shader.
// encode_srgb = 1: apply linear→sRGB encoding before output (for non-sRGB surfaces).

struct Globals {
    encode_srgb: u32,
};

@group(0) @binding(0) var src_texture: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;
@group(0) @binding(2) var<uniform> globals: Globals;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    // Oversized triangle that covers the full clip quad.
    var pos = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0),
        vec2( 3.0, -1.0),
        vec2(-1.0,  3.0),
    );
    var uv = array<vec2<f32>, 3>(
        vec2(0.0, 1.0),
        vec2(2.0, 1.0),
        vec2(0.0, -1.0),
    );
    var out: VertexOutput;
    out.clip_position = vec4(pos[idx], 0.0, 1.0);
    out.uv = uv[idx];
    return out;
}

fn linear_to_srgb_channel(c: f32) -> f32 {
    if c <= 0.0031308 { return c * 12.92; }
    return 1.055 * pow(c, 1.0 / 2.4) - 0.055;
}

fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    return vec3(
        linear_to_srgb_channel(c.r),
        linear_to_srgb_channel(c.g),
        linear_to_srgb_channel(c.b),
    );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(src_texture, src_sampler, in.uv);
    if globals.encode_srgb != 0u {
        return vec4(to_srgb(color.rgb), color.a);
    }
    return color;
}
