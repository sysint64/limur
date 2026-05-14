// Separable Gaussian blur shader — instancing style.
// Instance data comes through per-instance vertex attributes (no uniform buffer),
// matching the pattern in rect.wgsl.
//
// Bind group 0: source texture + sampler.
// Instance buffer: rect, tex_params, tint (locations 0–2).

@group(0) @binding(0) var src_texture: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;

// ── Per-instance vertex attributes ────────────────────────────────────────────

struct InstanceInput {
    @location(0) rect:       vec4<f32>,  // x, y, w, h in pixels (UI-space, y down)
    @location(1) tex_params: vec4<f32>,  // [tex_width, tex_height, blur_radius, 0]
    @location(2) tint:       vec4<f32>,  // RGBA premultiplied tint applied after V-pass
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv:         vec2<f32>,   // normalized [0, 1] texture coordinate
    @location(1) tex_params: vec4<f32>,   // forwarded from instance
    @location(2) tint:       vec4<f32>,   // forwarded from instance
    @location(3) rect:       vec4<f32>,   // x, y, w, h in pixels — used for edge clamping
};

// ── Vertex shader ─────────────────────────────────────────────────────────────
// Generates a quad from vertex_index (no vertex buffer).
// Topology: TriangleStrip, 4 vertices (TL, TR, BL, BR).

@vertex
fn vs_main(@builtin(vertex_index) idx: u32, instance: InstanceInput) -> VertexOutput {
    // Local [0,1] × [0,1] quad corners in TL, TR, BL, BR order.
    var lp = array<vec2<f32>, 4>(
        vec2(0.0, 0.0),
        vec2(1.0, 0.0),
        vec2(0.0, 1.0),
        vec2(1.0, 1.0),
    );
    let p = lp[idx];

    let tex_w = instance.tex_params.x;
    let tex_h = instance.tex_params.y;

    let pixel_x = instance.rect.x + p.x * instance.rect.z;
    let pixel_y = instance.rect.y + p.y * instance.rect.w;

    // NDC (y is up in clip space; pixel y is down)
    let ndc_x =  (pixel_x / tex_w) * 2.0 - 1.0;
    let ndc_y = -(pixel_y / tex_h) * 2.0 + 1.0;

    // UV (y is down in texture space, matches pixel_y)
    let uv_x = pixel_x / tex_w;
    let uv_y = pixel_y / tex_h;

    var out: VertexOutput;
    out.clip_position = vec4(ndc_x, ndc_y, 0.0, 1.0);
    out.uv         = vec2(uv_x, uv_y);
    out.tex_params = instance.tex_params;
    out.tint       = instance.tint;
    out.rect       = instance.rect;
    return out;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn gaussian_weight(offset: f32, sigma: f32) -> f32 {
    return exp(-(offset * offset) / (2.0 * sigma * sigma));
}

// Premultiplied alpha-over: src on top of dst.
fn blend_over(src: vec4<f32>, dst: vec4<f32>) -> vec4<f32> {
    return src + dst * (1.0 - src.a);
}

// ── Fragment: horizontal pass ─────────────────────────────────────────────────
// Reads from composite (full frame), writes to ping.
// Clamps X sampling to the rect's UV X range so border pixels repeat rather
// than bleeding in transparent content from outside the rect.

@fragment
fn fs_horizontal(in: VertexOutput) -> @location(0) vec4<f32> {
    let radius = i32(in.tex_params.z);
    if radius <= 0 {
        return textureSample(src_texture, src_sampler, in.uv);
    }

    let tex_w   = in.tex_params.x;
    let texel_x = 1.0 / tex_w;
    let sigma   = f32(radius) * 0.5;

    var color        = vec4(0.0);
    var total_weight = 0.0;

    for (var dx = -radius; dx <= radius; dx += 1) {
        let w = gaussian_weight(f32(dx), sigma);
        let s = vec2(in.uv.x + f32(dx) * texel_x, in.uv.y);
        color        += textureSample(src_texture, src_sampler, s) * w;
        total_weight += w;
    }

    return color / total_weight;
}

// ── Fragment: vertical pass ───────────────────────────────────────────────────
// Reads from ping (H-blur result), writes to composite.
// Ping was cleared to transparent before the H-pass; outside the rect it is
// (0,0,0,0).  Clamping Y to the rect's UV Y range prevents sampling those
// transparent pixels, which would otherwise cause black edges.
// Also applies the tint via premultiplied alpha-over.

@fragment
fn fs_vertical(in: VertexOutput) -> @location(0) vec4<f32> {
    let radius = i32(in.tex_params.z);
    if radius <= 0 {
        let base = textureSample(src_texture, src_sampler, in.uv);
        return blend_over(in.tint, base);
    }

    let tex_h   = in.tex_params.y;
    let texel_y = 1.0 / tex_h;
    let sigma   = f32(radius) * 0.5;

    var color        = vec4(0.0);
    var total_weight = 0.0;

    for (var dy = -radius; dy <= radius; dy += 1) {
        let w = gaussian_weight(f32(dy), sigma);
        let s = vec2(in.uv.x, in.uv.y + f32(dy) * texel_y);
        color        += textureSample(src_texture, src_sampler, s) * w;
        total_weight += w;
    }

    let blurred = color / total_weight;
    return blend_over(in.tint, blurred);
}
