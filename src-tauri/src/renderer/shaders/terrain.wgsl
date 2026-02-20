// ── WorldWeaver terrain.wgsl ──────────────────────────────────────────────────
// Full-screen triangle rendering of the R32Float heightmap texture.
// Inputs  : bind group 0 (heightmap, sampler, color_ramp, flow, uniforms)
// Technique: no vertex buffer — gl_VertexIndex 0..2 → clip-space triangle
//
// NOTE: heightmap (b0) and flow (b3) are R32Float which is NOT filterable on
// most adapters without TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES.
// We therefore use textureLoad (integer coords) for those two, and
// textureSample (with s_linear) only for the Rgba8Unorm color_ramp.

struct Uniforms {
    translate:        vec2<f32>,
    scale:            f32,
    _pad0:            f32,
    canvas_size:      vec2<f32>,
    world_size:       vec2<f32>,
    sea_level:        f32,
    max_elevation:    f32,
    contour_interval: f32,
    has_flow:         f32,
    hide_underwater:  f32,
    sun_azimuth:      f32,
    _pad1:            vec2<f32>,
};

@group(0) @binding(0) var t_heightmap:  texture_2d<f32>;
@group(0) @binding(1) var s_nonfilter:  sampler;        // NonFiltering (unused but keeps slot)
@group(0) @binding(2) var t_color_ramp: texture_2d<f32>;
@group(0) @binding(3) var t_flow:       texture_2d<f32>;
@group(0) @binding(4) var<uniform> u:   Uniforms;

// ── Vertex ────────────────────────────────────────────────────────────────────

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) screen_pos: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    // Full-screen triangle that covers clip space [-1,+1] × [-1,+1]
    let x = f32((vi << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(vi & 2u) * 2.0 - 1.0;
    var out: VsOut;
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
    // Convert clip [-1,+1] to screen pixels [0, canvas_size]
    out.screen_pos = vec2<f32>((x + 1.0) * 0.5 * u.canvas_size.x,
                               (1.0 - (y + 1.0) * 0.5) * u.canvas_size.y);
    return out;
}

// ── Fragment helpers ──────────────────────────────────────────────────────────

// Convert a UV [0,1] coordinate to integer texel coords for textureLoad.
fn uv_to_texel(uv: vec2<f32>, dims: vec2<u32>) -> vec2<i32> {
    let x = clamp(i32(uv.x * f32(dims.x)), 0, i32(dims.x) - 1);
    let y = clamp(i32(uv.y * f32(dims.y)), 0, i32(dims.y) - 1);
    return vec2<i32>(x, y);
}

fn sample_height(uv: vec2<f32>) -> f32 {
    let dims = textureDimensions(t_heightmap);
    return textureLoad(t_heightmap, uv_to_texel(uv, dims), 0).r;
}

fn sample_flow(uv: vec2<f32>) -> f32 {
    let dims = textureDimensions(t_flow);
    return textureLoad(t_flow, uv_to_texel(uv, dims), 0).r;
}

// Horn's method 3×3 Sobel for hillshading
fn hillshade(uv: vec2<f32>, px: vec2<f32>) -> f32 {
    let e  = px;
    let hN  = sample_height(uv + vec2<f32>( 0.0, -e.y));
    let hS  = sample_height(uv + vec2<f32>( 0.0,  e.y));
    let hE  = sample_height(uv + vec2<f32>( e.x,  0.0));
    let hW  = sample_height(uv + vec2<f32>(-e.x,  0.0));
    let hNE = sample_height(uv + vec2<f32>( e.x, -e.y));
    let hNW = sample_height(uv + vec2<f32>(-e.x, -e.y));
    let hSE = sample_height(uv + vec2<f32>( e.x,  e.y));
    let hSW = sample_height(uv + vec2<f32>(-e.x,  e.y));

    // Sobel gradient (Horn 1981)
    let dzdx = (hNE + 2.0*hE + hSE - hNW - 2.0*hW - hSW) / 8.0;
    let dzdy = (hSW + 2.0*hS + hSE - hNW - 2.0*hN - hNE) / 8.0;

    // Sun direction from azimuth (u.sun_azimuth degrees, 45° altitude)
    let az_rad  = u.sun_azimuth * 3.14159265 / 180.0;
    let sun_x = cos(az_rad);
    let sun_y = sin(az_rad);
    let sun_z = 1.0; // altitude factor

    // Normal dot light
    let normal = normalize(vec3<f32>(-dzdx * 8.0, -dzdy * 8.0, 1.0));
    let light  = normalize(vec3<f32>(sun_x, sun_y, sun_z));
    return clamp(dot(normal, light), 0.0, 1.0);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // Screen px → world coordinates
    let world = (in.screen_pos - u.translate) / u.scale;

    // Discard pixels outside the world rectangle
    if world.x < 0.0 || world.x >= u.world_size.x ||
       world.y < 0.0 || world.y >= u.world_size.y {
        return vec4<f32>(0.05, 0.08, 0.15, 1.0); // dark ocean background
    }

    let uv = world / u.world_size;

    // Sample height via textureLoad (R32Float — not filterable)
    let h = sample_height(uv);

    // Colour from hypsometric ramp (normalized height → ramp texel index)
    // Use textureLoad so we don't need a filtering sampler for the 256-wide ramp.
    let ramp_u = clamp(h, 0.0, 1.0);
    let ramp_i = i32(ramp_u * 255.0);
    var color = textureLoad(t_color_ramp, vec2<i32>(ramp_i, 0), 0).rgb;

    // Hillshade
    let px_size = 1.0 / u.world_size; // one texel step in UV space
    let shade = hillshade(uv, px_size);
    // Blend hillshade: multiply for dark side, partial on lit side
    let shaded = color * mix(0.45, 1.0, shade);

    // River overlay from flow accumulation (textureLoad — R32Float)
    var final_color = shaded;
    if u.has_flow > 0.5 && h >= u.sea_level {
        let flow = sample_flow(uv);
        if flow > 0.15 {
            let river_t = smoothstep(0.15, 0.8, flow);
            let river_col = vec3<f32>(0.18, 0.42, 0.70);
            final_color = mix(shaded, river_col, river_t * 0.75);
        }
    }

    // Contour lines (every contour_interval metres)
    if u.max_elevation > 0.0 && h >= u.sea_level {
        let elev = h * u.max_elevation;
        let contour_frac = fract(elev / u.contour_interval);
        // Thin line at boundary [0, 0.02] or [0.98, 1.0] in fraction space
        let line_w = 0.025;
        if contour_frac < line_w || contour_frac > (1.0 - line_w) {
            final_color = mix(final_color, vec3<f32>(0.0, 0.0, 0.0), 0.25);
        }
    }

    // Underwater tint
    if h < u.sea_level {
        let depth = (u.sea_level - h) / u.sea_level;
        let water = vec3<f32>(0.04, 0.15, 0.35);
        final_color = mix(final_color, water, clamp(depth * 2.5, 0.0, 0.85));
    }

    return vec4<f32>(final_color, 1.0);
}
