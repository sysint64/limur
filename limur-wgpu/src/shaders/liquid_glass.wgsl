// Liquid glass backdrop effect.
//
// Pre-pass (handled in Rust): blurs the composite texture.
// This pass: reads the blurred composite with refraction distortion, clips to
// a superellipse (squircle) shape, and adds an edge glow.
//
// Bind group 0:
//   binding 0 — composite texture (blurred)
//   binding 1 — sampler
//   binding 2 — Globals uniform { screen_size: vec2<f32> }
//
// Instance attributes (locations 0–3):
//   0: boundary   [x, y, w, h] in pixels (y-down)
//   1: params     [power_factor, f_power, noise, glow_weight]
//   2: refraction [a, b, c, d]
//   3: tint       premultiplied RGBA

@group(0) @binding(0) var composite_tex: texture_2d<f32>;
@group(0) @binding(1) var composite_sampler: sampler;

struct Globals {
    screen_size: vec2<f32>,
}
@group(0) @binding(2) var<uniform> globals: Globals;

struct Instance {
    @location(0) boundary:   vec4<f32>,  // x, y, w, h pixels
    @location(1) params:     vec4<f32>,  // power_factor, f_power, noise, glow_weight
    @location(2) refraction: vec4<f32>,  // a, b, c, d
    @location(3) tint:       vec4<f32>,  // premultiplied RGBA
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_uv:   vec2<f32>,   // [0,1] within the quad
    @location(1) boundary:   vec4<f32>,
    @location(2) params:     vec4<f32>,
    @location(3) refraction: vec4<f32>,
    @location(4) tint:       vec4<f32>,
}

// ── Vertex ────────────────────────────────────────────────────────────────────

@vertex
fn vs_main(@builtin(vertex_index) idx: u32, inst: Instance) -> VertexOutput {
    // TL, TR, BL, BR corner in [0,1] × [0,1]
    let corner = vec2<f32>(f32(idx & 1u), f32((idx >> 1u) & 1u));

    let px = inst.boundary.xy + corner * inst.boundary.zw;
    let ndc = vec2<f32>(
         px.x / globals.screen_size.x * 2.0 - 1.0,
        -px.y / globals.screen_size.y * 2.0 + 1.0,
    );

    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc, 0.0, 1.0);
    out.local_uv      = corner;
    out.boundary      = inst.boundary;
    out.params        = inst.params;
    out.refraction    = inst.refraction;
    out.tint          = inst.tint;
    return out;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

// Approximate signed distance to a superellipse |x|^n + |y|^n = r^n.
// Negative inside, positive outside.
fn sdSuperellipse(p: vec2<f32>, n: f32, r: f32) -> f32 {
    let pa = abs(p);
    let numerator   = pow(pa.x, n) + pow(pa.y, n) - pow(r, n);
    let denominator = n * sqrt(pow(pa.x, 2.0 * n - 2.0) + pow(pa.y, 2.0 * n - 2.0)) + 1e-6;
    return numerator / denominator;
}

// Refraction falloff: f(x) = 1 - b * (c * e)^(-d*x - a)
fn refract_f(x: f32, a: f32, b: f32, c: f32, d: f32) -> f32 {
    let M_E = 2.718281828459045;
    return 1.0 - b * pow(c * M_E, -d * x - a);
}

fn rand(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// ── Fragment ──────────────────────────────────────────────────────────────────

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let power_factor = in.params.x;
    let f_power      = in.params.y;
    let noise_amt    = in.params.z;
    let glow_weight  = in.params.w;

    let a = in.refraction.x;
    let b = in.refraction.y;
    let c = in.refraction.z;
    let d = in.refraction.w;

    // Local coord in [-1, 1] × [-1, 1]
    let p = (in.local_uv - vec2<f32>(0.5)) * 2.0;

    // Clip to superellipse
    let dist_signed = sdSuperellipse(p, power_factor, 1.0);
    if dist_signed > 0.0 {
        discard;
    }
    let dist = -dist_signed;  // positive distance from edge (0 at edge, increases inward)

    // Refraction: map local position toward center based on edge distance
    let rf = pow(refract_f(dist, a, b, c, d), f_power);
    let sample_p = p * rf;

    // Convert refracted local coord to composite UV
    let half_size  = in.boundary.zw * 0.5 / globals.screen_size;
    let center_uv  = (in.boundary.xy + in.boundary.zw * 0.5) / globals.screen_size;
    let sample_uv  = center_uv + sample_p * half_size;

    // Sample blurred composite
    var color = textureSample(composite_tex, composite_sampler, sample_uv);

    // Noise
    let n = (rand(in.clip_position.xy * 1e-3) - 0.5) * noise_amt;
    color += vec4<f32>(n, n, n, 0.0);

    // Edge glow: sinusoidal angular highlight, fades away from edge
    let glow = sin(atan2(in.local_uv.y * 2.0 - 1.0, in.local_uv.x * 2.0 - 1.0) - 0.5);
    let glow_fade = smoothstep(0.5, -0.5, dist);
    let mul = glow * glow_weight * glow_fade + 1.0;
    color = color * vec4<f32>(mul, mul, mul, 1.0);

    // Tint on top (premultiplied alpha-over)
    return in.tint + color * (1.0 - in.tint.a);
}
