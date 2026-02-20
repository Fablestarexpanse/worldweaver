// ── terrain/volcanoes.rs ──────────────────────────────────────────────────────
// Places volcanic cones on the heightmap.

use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolcanoConfig {
    pub count:  u32,
    pub radius: f32,
    pub height: f32,
}

impl Default for VolcanoConfig {
    fn default() -> Self {
        Self { count: 3, radius: 80.0, height: 0.95 }
    }
}

/// Stamp volcanic cones onto heights in place.
pub fn apply(heights: &mut Vec<f32>, w: usize, h: usize, cfg: &VolcanoConfig, seed: u64) {
    let mut rng = SmallRng::seed_from_u64(seed ^ 0xF00D_CAFE);

    for _ in 0..cfg.count {
        let cx = rng.gen_range(cfg.radius as usize..(w - cfg.radius as usize)) as f32;
        let cy = rng.gen_range(cfg.radius as usize..(h - cfg.radius as usize)) as f32;

        let r = cfg.radius;
        let top = cfg.height;
        // Caldera radius ~15% of cone
        let caldera_r = r * 0.15;

        let xi0 = ((cx - r).max(0.0) as usize).min(w-1);
        let xi1 = ((cx + r).min(w as f32 - 1.0) as usize).min(w-1);
        let yi0 = ((cy - r).max(0.0) as usize).min(h-1);
        let yi1 = ((cy + r).min(h as f32 - 1.0) as usize).min(h-1);

        for y in yi0..=yi1 {
            for x in xi0..=xi1 {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let d = (dx*dx + dy*dy).sqrt();
                if d > r { continue; }

                let t = 1.0 - d / r;
                let cone_h = top * t * t; // squared falloff

                // Caldera depression at peak
                let caldera_depth = if d < caldera_r {
                    let ct = 1.0 - d / caldera_r;
                    ct * ct * 0.25
                } else { 0.0 };

                let target = cone_h - caldera_depth;
                let idx = y * w + x;
                heights[idx] = heights[idx].max(target).clamp(0.0, 1.0);
            }
        }
    }
}
