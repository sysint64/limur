// Liquid glass — two-pass separable effect.
//
// H-pass:        composite → ping  (horizontal Gaussian blur, expanded Y quad)
// V-refract pass: ping → composite  (vertical blur + refraction + superellipse clip)
//
// Bind group 0: source texture + sampler (no globals — tex_w/tex_h come from instance).
//
// Instance attributes (locations 0–4):
//   0: rect        [x, y, w, h] in pixels (y-down)
//   1: tex_params  [tex_w, tex_h, blur_radius, 0]
//   2: tint        premultiplied RGBA
//   3: params      [power_factor, f_power, noise, glow_weight]
//   4: refraction  [a, b, c, d]

@group(0) @binding(0) var src_texture: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;

struct InstanceInput {
    @location(0) rect:       vec4<f32>,
    @location(1) tex_params: vec4<f32>,
    @location(2) tint:       vec4<f32>,
    @location(3) params:     vec4<f32>,
    @location(4) refraction: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv:         vec2<f32>,   // UV into source texture
    @location(1) local_uv:   vec2<f32>,   // [0,1] within the quad
    @location(2) rect:       vec4<f32>,
    @location(3) tex_params: vec4<f32>,
    @location(4) tint:       vec4<f32>,
    @location(5) params:     vec4<f32>,
    @location(6) refraction: vec4<f32>,
}

// ── Vertex ────────────────────────────────────────────────────────────────────

@vertex
fn vs_main(@builtin(vertex_index) idx: u32, inst: InstanceInput) -> VertexOutput {
    var lp = array<vec2<f32>, 4>(
        vec2(0.0, 0.0),
        vec2(1.0, 0.0),
        vec2(0.0, 1.0),
        vec2(1.0, 1.0),
    );
    let corner = lp[idx];

    let tex_w = inst.tex_params.x;
    let tex_h = inst.tex_params.y;

    let px = inst.rect.xy + corner * inst.rect.zw;
    let uv = vec2<f32>(px.x / tex_w, px.y / tex_h);
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);

    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc, 0.0, 1.0);
    out.uv            = uv;
    out.local_uv      = corner;
    out.rect          = inst.rect;
    out.tex_params    = inst.tex_params;
    out.tint          = inst.tint;
    out.params        = inst.params;
    out.refraction    = inst.refraction;
    return out;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn gaussian_weight(offset: f32, sigma: f32) -> f32 {
    return exp(-(offset * offset) / (2.0 * sigma * sigma));
}

fn sdSuperellipse(p: vec2<f32>, n: f32, r: f32) -> f32 {
    let pa = abs(p);
    let numerator   = pow(pa.x, n) + pow(pa.y, n) - pow(r, n);
    let denominator = n * sqrt(pow(pa.x, 2.0 * n - 2.0) + pow(pa.y, 2.0 * n - 2.0)) + 1e-5;
    return numerator / denominator;
}

fn refract_f(x: f32, a: f32, b: f32, c: f32, d: f32) -> f32 {
    let M_E = 2.718281828459045;
    return 1.0 - b * pow(c * M_E, -d * x - a);
}

fn rand(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// ── H-pass ────────────────────────────────────────────────────────────────────
// Reads composite, writes horizontal-blurred result to ping.
// Quad is expanded vertically by blur_radius so the V-pass edges have valid data.

@fragment
fn fs_horizontal(in: VertexOutput) -> @location(0) vec4<f32> {
    let radius = i32(in.tex_params.z);
    if radius <= 0 {
        return textureSample(src_texture, src_sampler, in.uv);
    }

    let texel_x = 1.0 / in.tex_params.x;
    let sigma   = f32(radius) * 0.5;

    var color        = vec4<f32>(0.0);
    var total_weight = 0.0;

    for (var dx = -radius; dx <= radius; dx += 1) {
        let w = gaussian_weight(f32(dx), sigma);
        let s = vec2<f32>(in.uv.x + f32(dx) * texel_x, in.uv.y);
        color        += textureSample(src_texture, src_sampler, s) * w;
        total_weight += w;
    }

    return color / total_weight;
}

// ── V-refract pass ────────────────────────────────────────────────────────────
// Reads ping (H-blurred). For pixels inside the superellipse: applies refraction
// distortion and vertical blur, then writes to composite.
// Pixels outside the superellipse are discarded — composite keeps original content.
// LoadOp::Load on composite ensures this.

@fragment
fn fs_vertical_refract(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_w  = in.tex_params.x;
    let tex_h  = in.tex_params.y;
    let radius = i32(in.tex_params.z);
    let sigma  = f32(radius) * 0.5;

    let power_factor = in.params.x;
    let f_power      = in.params.y;
    let noise_amt    = in.params.z;
    let glow_weight  = in.params.w;

    let a = in.refraction.x;
    let b = in.refraction.y;
    let c = in.refraction.z;
    let d = in.refraction.w;

    // Local coord in [-1, 1]
    let p = (in.local_uv - vec2<f32>(0.5)) * 2.0;

    // Superellipse clip — discard means composite retains original pixel here.
    let dist_signed = sdSuperellipse(p, power_factor, 1.0);
    if dist_signed > 0.0 {
        discard;
    }
    let dist = -dist_signed;

    // Refracted local position
    let rf       = pow(refract_f(dist, a, b, c, d), f_power);
    let sample_p = p * rf;

    // Map refracted position to ping UV
    let center_uv    = (in.rect.xy + in.rect.zw * 0.5) / vec2<f32>(tex_w, tex_h);
    let half_size    = in.rect.zw * 0.5 / vec2<f32>(tex_w, tex_h);
    let refracted_uv = center_uv + sample_p * half_size;

    // Vertical Gaussian blur centered on refracted UV
    var color = vec4<f32>(0.0);
    if radius <= 0 {
        color = textureSample(src_texture, src_sampler, refracted_uv);
    } else {
        let texel_y = 1.0 / tex_h;
        var total_weight = 0.0;

        for (var dy = -radius; dy <= radius; dy += 1) {
            let w = gaussian_weight(f32(dy), sigma);
            let s = vec2<f32>(refracted_uv.x, refracted_uv.y + f32(dy) * texel_y);
            color        += textureSample(src_texture, src_sampler, s) * w;
            total_weight += w;
        }
        color /= total_weight;
    }

    // Edge glow
    let glow      = sin(atan2(p.y, p.x) - 0.5);
    let glow_fade = smoothstep(0.5, -0.5, dist);
    let mul       = glow * glow_weight * glow_fade + 1.0;
    color         = color * vec4<f32>(mul, mul, mul, 1.0);

    // Noise
    let n = (rand(in.clip_position.xy * 1e-3) - 0.5) * noise_amt;
    color += vec4<f32>(n, n, n, 0.0);

    // Tint (premultiplied alpha-over)
    return in.tint + color * (1.0 - in.tint.a);
}
