// ── brush_flatten.wgsl ────────────────────────────────────────────────────────
// Compute shader: push terrain toward flatten_target height within brush.

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

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(hmap);
    if gid.x >= dim.x || gid.y >= dim.y { return; }

    let coord = vec2<f32>(f32(gid.x), f32(gid.y));
    let d = distance(coord, bp.center);
    if d > bp.radius { return; }

    let t = 1.0 - d / bp.radius;
    let falloff = t * t * (3.0 - 2.0 * t);

    let old_h = textureLoad(hmap, vec2<i32>(gid.xy)).r;
    let new_h = mix(old_h, bp.flatten_target, falloff * bp.strength * 0.15);
    textureStore(hmap, vec2<i32>(gid.xy), vec4<f32>(clamp(new_h, 0.0, 1.0), 0.0, 0.0, 1.0));
}
