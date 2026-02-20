// ── terrain/mod.rs ────────────────────────────────────────────────────────────
pub mod config;
pub mod noise_gen;
pub mod erosion;
pub mod hydrology;
pub mod biomes;
pub mod volcanoes;
pub mod persistence;

pub use config::TerrainConfig;

use anyhow::Result;
use crate::state::TerrainData;

/// Full terrain generation pipeline.
/// Called from the `generate_terrain` Tauri command.
pub fn generate(cfg: TerrainConfig) -> Result<TerrainData> {
    let w = cfg.world_width as usize;
    let h = cfg.world_height as usize;

    log::info!("Generating terrain {}×{} seed={}", w, h, cfg.seed);

    // 1. Multi-octave noise base
    let mut heights = noise_gen::generate(&cfg);

    // 2. Hydraulic erosion passes
    erosion::erode(&mut heights, w, h, cfg.erosion_passes);

    // 3. Biome classification
    let biomes = biomes::classify(&heights, w, h, cfg.sea_level);

    // 4. Flow accumulation (hydrology)
    let flow = hydrology::accumulate(&heights, w, h);

    log::info!("Terrain generation complete");

    Ok(TerrainData {
        config: cfg,
        heights,
        flow,
        biomes,
        dirty: true,
    })
}
