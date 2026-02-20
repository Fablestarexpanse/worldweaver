// ── terrain/erosion.rs ────────────────────────────────────────────────────────
// CPU hydraulic erosion simulation (runs once at generation time).
// For interactive brush erosion we use the GPU compute shader instead.

use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;

/// Run `passes` iterations of droplet-based erosion on `heights`.
pub fn erode(heights: &mut Vec<f32>, w: usize, h: usize, passes: u32) {
    if passes == 0 { return; }

    let mut rng = SmallRng::seed_from_u64(0xDEAD_BEEF);
    let n_drops = w * h / 4; // ~25% pixel coverage per pass

    for _ in 0..passes {
        for _ in 0..n_drops {
            drop_erode(heights, w, h, &mut rng);
        }
    }
}

fn drop_erode(heights: &mut Vec<f32>, w: usize, h: usize, rng: &mut SmallRng) {
    let mut x = rng.gen_range(1..w-1) as f32;
    let mut y = rng.gen_range(1..h-1) as f32;
    let mut sediment = 0.0f32;
    let mut speed    = 0.0f32;
    let mut water    = 1.0f32;

    const INERTIA:    f32 = 0.05;
    const CAPACITY:   f32 = 4.0;
    const EROSION:    f32 = 0.3;
    const DEPOSITION: f32 = 0.3;
    const EVAPORATION:f32 = 0.01;
    const GRAVITY:    f32 = 1.0;
    const MAX_STEPS:  usize = 64;

    let mut dir_x = 0.0f32;
    let mut dir_y = 0.0f32;

    for _ in 0..MAX_STEPS {
        let ix = x as usize;
        let iy = y as usize;
        if ix == 0 || ix >= w-1 || iy == 0 || iy >= h-1 { break; }

        // Bilinear sample of gradient
        let fx = x.fract();
        let fy = y.fract();
        let h00 = heights[iy * w + ix];
        let h10 = heights[iy * w + ix + 1];
        let h01 = heights[(iy+1) * w + ix];
        let h11 = heights[(iy+1) * w + ix + 1];

        // Gradient
        let gx = (h10 - h00) * (1.0 - fy) + (h11 - h01) * fy;
        let gy = (h01 - h00) * (1.0 - fx) + (h11 - h10) * fx;
        let h_cur = h00 * (1.0-fx)*(1.0-fy) + h10 * fx*(1.0-fy)
                  + h01 * (1.0-fx)*fy        + h11 * fx*fy;

        dir_x = dir_x * INERTIA - gx * (1.0 - INERTIA);
        dir_y = dir_y * INERTIA - gy * (1.0 - INERTIA);
        let len = (dir_x*dir_x + dir_y*dir_y).sqrt().max(1e-6);
        dir_x /= len;
        dir_y /= len;

        let new_x = x + dir_x;
        let new_y = y + dir_y;
        let nx = new_x as usize;
        let ny = new_y as usize;
        if nx == 0 || nx >= w-1 || ny == 0 || ny >= h-1 { break; }

        let h_new = heights[ny * w + nx];
        let h_diff = h_new - h_cur;

        let capacity = (-h_diff).max(0.01) * speed * water * CAPACITY;

        if sediment > capacity {
            // Deposit excess sediment
            let deposit = (sediment - capacity) * DEPOSITION;
            sediment -= deposit;
            heights[iy * w + ix] += deposit;
        } else {
            // Erode
            let erode_amt = ((capacity - sediment) * EROSION).min(-h_diff).max(0.0);
            sediment += erode_amt;
            heights[iy * w + ix] -= erode_amt;
        }

        speed = (speed * speed + h_diff.abs() * GRAVITY).sqrt();
        water *= 1.0 - EVAPORATION;
        x = new_x;
        y = new_y;
    }
}
