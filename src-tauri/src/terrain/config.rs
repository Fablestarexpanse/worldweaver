// ── terrain/config.rs ─────────────────────────────────────────────────────────
use serde::{Deserialize, Serialize};

/// All parameters that define a world generation run.
/// Serialised to/from JSON for the Tauri command interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainConfig {
    pub world_width:     u32,
    pub world_height:    u32,
    pub seed:            u64,

    // Noise
    pub octaves:         u32,
    pub frequency:       f64,
    pub persistence:     f64,
    pub lacunarity:      f64,
    pub amplitude:       f64,

    // Sea and elevation
    pub sea_level:       f32,
    pub max_elevation:   f32,

    // Erosion
    pub erosion_passes:  u32,

    // Render hints
    pub contour_interval: f32,
    pub sun_azimuth:     f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            world_width:      1024,
            world_height:     768,
            seed:             42,
            octaves:          8,
            frequency:        2.0,
            persistence:      0.5,
            lacunarity:       2.0,
            amplitude:        1.0,
            sea_level:        0.42,
            max_elevation:    4000.0,
            erosion_passes:   5,
            contour_interval: 100.0,
            sun_azimuth:      315.0,
        }
    }
}
