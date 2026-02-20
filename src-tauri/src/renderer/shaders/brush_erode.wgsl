// ── brush_erode.wgsl ──────────────────────────────────────────────────────────
// Compute shader: simplified hydraulic erosion pass within brush area.
// Each invocation moves height toward a local minimum, simulating water runoff.

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

    let h   = load(ix,   iy,   dim);
    let hN  = load(ix,   iy-1, dim);
    let hS  = load(ix,   iy+1, dim);
    let hE  = load(ix+1, iy,   dim);
    let hW  = load(ix-1, iy,   dim);

    // Steepest downhill neighbour
    var h_min = h;
    h_min = min(h_min, hN);
    h_min = min(h_min, hS);
    h_min = min(h_min, hE);
    h_min = min(h_min, hW);

    // Erode toward steepest neighbour
    let erode_amount = max(0.0, h - h_min) * falloff * bp.strength * 0.08;
    let new_h = clamp(h - erode_amount, 0.0, 1.0);
    textureStore(hmap, vec2<i32>(gid.xy), vec4<f32>(new_h, 0.0, 0.0, 1.0));
}
