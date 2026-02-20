// ── terrain/noise_gen.rs ──────────────────────────────────────────────────────
// Multi-octave fractional Brownian motion heightmap generation.
// Uses the `noise` crate v0.9 Fbm<Perlin>.

use noise::{NoiseFn, Perlin, Fbm, MultiFractal};
use super::config::TerrainConfig;

pub fn generate(cfg: &TerrainConfig) -> Vec<f32> {
    let w = cfg.world_width as usize;
    let h = cfg.world_height as usize;
    let n = w * h;

    // Build fractional Brownian motion source
    // noise 0.9: Fbm::new(seed) then set_* builder methods
    let fbm = Fbm::<Perlin>::new(cfg.seed as u32)
        .set_octaves(cfg.octaves as usize)
        .set_frequency(cfg.frequency)
        .set_persistence(cfg.persistence)
        .set_lacunarity(cfg.lacunarity);

    // Generate sequentially (Fbm is not Send in noise 0.9, so no rayon here)
    let mut heights = Vec::with_capacity(n);
    for i in 0..n {
        let xi = (i % w) as f64 / w as f64;
        let yi = (i / w) as f64 / h as f64;
        // Sample at 3D point with z=0 to get 2D slice
        let v = fbm.get([xi, yi, 0.0]) as f32;
        // Fbm returns roughly [-1, +1] — normalise to [0, 1]
        let normalised = (v + 1.0) * 0.5;
        heights.push(normalised.clamp(0.0, 1.0));
    }

    // Apply island mask: fade edges toward ocean
    apply_island_mask(&mut heights, w, h, cfg.sea_level);

    heights
}

/// Radial gradient that pushes map edges below sea level → island / continent shape.
fn apply_island_mask(heights: &mut [f32], w: usize, h: usize, sea_level: f32) {
    for y in 0..h {
        for x in 0..w {
            let nx = x as f32 / w as f32 - 0.5;
            let ny = y as f32 / h as f32 - 0.5;
            let dist = (nx * nx + ny * ny).sqrt() * 2.0; // [0..~1.4]
            let mask = (1.0 - dist).max(0.0).powf(1.5);  // smooth falloff
            let idx = y * w + x;
            heights[idx] = heights[idx] * mask + sea_level * 0.8 * (1.0 - mask);
            heights[idx] = heights[idx].clamp(0.0, 1.0);
        }
    }
}
