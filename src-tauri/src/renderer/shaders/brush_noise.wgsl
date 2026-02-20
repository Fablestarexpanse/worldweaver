// ── brush_noise.wgsl ──────────────────────────────────────────────────────────
// Compute shader: add procedural noise perturbation within brush.
// Uses a simple hash-based value noise (no external textures needed).

struct BrushParams {
    center:        vec2<f32>,
    radius:        f32,
    strength:      f32,
    flatten_target: f32,
    noise_scale:   f32,
    _pad:          vec2<f32>,
};

@group(0) @binding(0) var hmap: texture_storage_2d<r32float, read_write>;
@group(0) @binding(1) var<uniform> bp: BrushParams;

// Hash-based pseudo-random noise [−1, 1]
fn hash2(p: vec2<f32>) -> f32 {
    var q = vec2<f32>(dot(p, vec2<f32>(127.1, 311.7)),
                      dot(p, vec2<f32>(269.5, 183.3)));
    q = -1.0 + 2.0 * fract(sin(q) * 43758.5453123);
    return q.x;
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f); // smoothstep
    let a = hash2(i + vec2<f32>(0.0, 0.0));
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let dd = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, dd, u.x), u.y);
}

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(hmap);
    if gid.x >= dim.x || gid.y >= dim.y { return; }

    let coord = vec2<f32>(f32(gid.x), f32(gid.y));
    let d = distance(coord, bp.center);
    if d > bp.radius { return; }

    let t = 1.0 - d / bp.radius;
    let falloff = t * t * (3.0 - 2.0 * t);

    // Multi-octave noise
    let p = coord * bp.noise_scale;
    let n = value_noise(p) * 0.5
          + value_noise(p * 2.0) * 0.25
          + value_noise(p * 4.0) * 0.125;

    let old_h = textureLoad(hmap, vec2<i32>(gid.xy)).r;
    let delta = n * bp.strength * falloff * 0.006;
    let new_h = clamp(old_h + delta, 0.0, 1.0);
    textureStore(hmap, vec2<i32>(gid.xy), vec4<f32>(new_h, 0.0, 0.0, 1.0));
}
