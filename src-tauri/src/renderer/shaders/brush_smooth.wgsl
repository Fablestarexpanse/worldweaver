// ── brush_smooth.wgsl ─────────────────────────────────────────────────────────
// Compute shader: Gaussian-blur the heightmap within the brush radius.

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

fn load(x: i32, y: i32, dim: vec2<u32>) -> f32 {
    let cx = clamp(x, 0, i32(dim.x) - 1);
    let cy = clamp(y, 0, i32(dim.y) - 1);
    return textureLoad(hmap, vec2<i32>(cx, cy)).r;
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

    let ix = i32(gid.x);
    let iy = i32(gid.y);

    // 3×3 Gaussian kernel weights
    let k00 = 1.0; let k01 = 2.0; let k02 = 1.0;
    let k10 = 2.0; let k11 = 4.0; let k12 = 2.0;
    let k20 = 1.0; let k21 = 2.0; let k22 = 1.0;
    let total = 16.0;

    let blurred = (
        k00 * load(ix-1, iy-1, dim) + k01 * load(ix, iy-1, dim) + k02 * load(ix+1, iy-1, dim) +
        k10 * load(ix-1, iy,   dim) + k11 * load(ix, iy,   dim) + k12 * load(ix+1, iy,   dim) +
        k20 * load(ix-1, iy+1, dim) + k21 * load(ix, iy+1, dim) + k22 * load(ix+1, iy+1, dim)
    ) / total;

    let old_h = load(ix, iy, dim);
    let new_h = mix(old_h, blurred, falloff * bp.strength);
    textureStore(hmap, vec2<i32>(gid.xy), vec4<f32>(clamp(new_h, 0.0, 1.0), 0.0, 0.0, 1.0));
}
