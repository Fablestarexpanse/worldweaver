// ── undo_gpu.rs ───────────────────────────────────────────────────────────────
// Before a brush stroke begins, snapshot the pixels under the brush radius
// from the GPU heightmap texture into a CPU-side zstd-compressed UndoRegion.
//
// Restore: decompress, write back to GPU via upload_heightmap sub-region.

use std::sync::Arc;
use parking_lot::Mutex;
use crate::state::{AppState, UndoRegion};
use super::context::WgpuContext;

/// Capture a square region centred on (screen_x, screen_y) before painting.
/// The captured area is brush_radius * 2 pixels wide in world space.
pub fn snapshot_brush_region(
    _ctx:     &mut WgpuContext,
    state:    &Arc<Mutex<AppState>>,
    screen_x: f32,
    screen_y: f32,
) {
    let st = state.lock();

    let terrain = match &st.terrain {
        Some(t) => t,
        None => return,
    };

    let radius  = st.brush.radius as u32;
    let (wx, wy) = st.viewport.screen_to_world(screen_x, screen_y);
    let wx = wx as i32;
    let wy = wy as i32;
    let ww = terrain.config.world_width as i32;
    let wh = terrain.config.world_height as i32;

    // Clamp region to world bounds
    let x0 = (wx - radius as i32).max(0) as u32;
    let y0 = (wy - radius as i32).max(0) as u32;
    let x1 = (wx + radius as i32).min(ww - 1) as u32;
    let y1 = (wy + radius as i32).min(wh - 1) as u32;
    let rw = x1.saturating_sub(x0) + 1;
    let rh = y1.saturating_sub(y0) + 1;

    if rw == 0 || rh == 0 { return; }

    // Extract CPU-side heights for this region from the flat array
    let mut region_data = vec![0f32; (rw * rh) as usize];
    for row in 0..rh {
        for col in 0..rw {
            let si = ((y0 + row) * terrain.config.world_width + (x0 + col)) as usize;
            let di = (row * rw + col) as usize;
            region_data[di] = terrain.heights[si];
        }
    }

    // zstd-compress
    let raw: &[u8] = bytemuck::cast_slice(&region_data);
    let compressed = match zstd::encode_all(raw, 3) {
        Ok(c) => c,
        Err(e) => { log::warn!("undo snapshot compress failed: {e}"); return; }
    };

    let region = UndoRegion { x: x0, y: y0, width: rw, height: rh, compressed };

    drop(st); // release before lock in push
    state.lock().undo_stack.push(region);
}

/// Restore the most recent undo region to both CPU state and GPU texture.
pub fn apply_undo(ctx: &mut WgpuContext, state: &Arc<Mutex<AppState>>) {
    let mut st = state.lock();

    let region = match st.undo_stack.pop() {
        Some(r) => r,
        None => return,
    };

    let terrain = match &mut st.terrain {
        Some(t) => t,
        None => return,
    };

    // Decompress
    let raw = match zstd::decode_all(region.compressed.as_slice()) {
        Ok(r) => r,
        Err(e) => { log::warn!("undo decompress failed: {e}"); return; }
    };
    let heights: &[f32] = bytemuck::cast_slice(&raw);

    // Restore CPU-side heights
    let ww = terrain.config.world_width;
    for row in 0..region.height {
        for col in 0..region.width {
            let di = (row * region.width + col) as usize;
            let si = ((region.y + row) * ww + (region.x + col)) as usize;
            if di < heights.len() && si < terrain.heights.len() {
                terrain.heights[si] = heights[di];
            }
        }
    }

    // Write sub-region back to GPU texture
    let bytes: &[u8] = bytemuck::cast_slice(heights);
    ctx.queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &ctx.heightmap_texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x: region.x, y: region.y, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        &bytes[..(region.width * region.height * 4) as usize],
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(region.width * 4),
            rows_per_image: Some(region.height),
        },
        wgpu::Extent3d {
            width:  region.width,
            height: region.height,
            depth_or_array_layers: 1,
        },
    );
}
