// ── terrain/hydrology.rs ──────────────────────────────────────────────────────
// Steepest-descent flow accumulation (D8 algorithm).
// Returns a normalised [0, 1] flow array (same size as heights).

pub fn accumulate(heights: &[f32], w: usize, h: usize) -> Vec<f32> {
    let n = w * h;
    let mut acc = vec![1u32; n]; // each cell starts with 1 drop of rain

    // Build drainage direction array
    let mut flow_dir = vec![0usize; n]; // index of the downhill neighbour
    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            let cur = heights[idx];
            let mut best_idx = idx;
            let mut best_h   = cur;

            // 8-connectivity
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || nx >= w as i32 || ny < 0 || ny >= h as i32 { continue; }
                    let ni = ny as usize * w + nx as usize;
                    if heights[ni] < best_h {
                        best_h = heights[ni];
                        best_idx = ni;
                    }
                }
            }
            flow_dir[idx] = best_idx;
        }
    }

    // Topological sort by elevation (process highest first)
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_unstable_by(|&a, &b| {
        heights[b].partial_cmp(&heights[a]).unwrap_or(std::cmp::Ordering::Equal)
    });

    for &idx in &order {
        let dst = flow_dir[idx];
        if dst != idx {
            let a = acc[idx];
            acc[dst] += a;
        }
    }

    // Normalise to [0, 1] with sqrt to compress the dynamic range
    let max_acc = *acc.iter().max().unwrap_or(&1) as f32;
    acc.iter()
       .map(|&a| ((a as f32).sqrt() / max_acc.sqrt()).clamp(0.0, 1.0))
       .collect()
}
