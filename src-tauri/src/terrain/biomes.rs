// ── terrain/biomes.rs ─────────────────────────────────────────────────────────
// Whittaker-diagram biome classification by elevation and latitude proxy.

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Biome {
    DeepOcean   = 0,
    ShallowSea  = 1,
    Beach       = 2,
    Grassland   = 3,
    Shrubland   = 4,
    Savanna     = 5,
    TropicalRainforest = 6,
    TemperateForest   = 7,
    BorealForest      = 8,
    Tundra      = 9,
    Snow        = 10,
    Desert      = 11,
    Mountain    = 12,
    HighMountain = 13,
}

/// Classify each pixel into a biome.
/// Returns a `Vec<u8>` of `Biome` discriminants.
pub fn classify(heights: &[f32], w: usize, h: usize, sea_level: f32) -> Vec<u8> {
    heights.iter().enumerate().map(|(i, &hv)| {
        let y_frac = (i / w) as f32 / h as f32; // 0 = top (north), 1 = bottom (south)
        // Latitude proxy: 0 at equator (y=0.5), 1 at poles
        let lat = (y_frac - 0.5).abs() * 2.0;

        if hv < sea_level - 0.1 {
            return Biome::DeepOcean as u8;
        }
        if hv < sea_level {
            return Biome::ShallowSea as u8;
        }
        if hv < sea_level + 0.02 {
            return Biome::Beach as u8;
        }

        let elev_above = (hv - sea_level) / (1.0 - sea_level);

        if elev_above > 0.85 {
            return Biome::HighMountain as u8;
        }
        if elev_above > 0.65 {
            return if lat > 0.6 { Biome::Snow as u8 } else { Biome::Mountain as u8 };
        }

        // Low-to-mid elevation: biome by latitude
        if lat > 0.75 {
            return if elev_above > 0.3 { Biome::Snow as u8 } else { Biome::Tundra as u8 };
        }
        if lat > 0.55 {
            return Biome::BorealForest as u8;
        }
        if lat > 0.35 {
            return if elev_above > 0.4 { Biome::Mountain as u8 } else { Biome::TemperateForest as u8 };
        }
        if lat > 0.15 {
            return Biome::Shrubland as u8;
        }
        // Equatorial
        if elev_above < 0.25 { Biome::TropicalRainforest as u8 }
        else if elev_above < 0.5 { Biome::Savanna as u8 }
        else { Biome::Mountain as u8 }
    }).collect()
}
