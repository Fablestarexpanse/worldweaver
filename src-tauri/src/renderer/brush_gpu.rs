// ── brush_gpu.rs ──────────────────────────────────────────────────────────────
// Helpers for GPU brush dispatch:
//   1. Convert screen-space mouse position → world-space center
//   2. Write BrushParamsGpu into the uniform buffer
//   3. Call WgpuContext::dispatch_compute_brush()

use std::sync::Arc;
use parking_lot::Mutex;
use crate::state::AppState;
use super::context::{WgpuContext, BrushParamsGpu};

/// Called from the render thread event loop on every mouse-down / mouse-drag
/// while the left button is held and a brush tool is active.
pub fn dispatch_brush(
    ctx:   &mut WgpuContext,
    state: &Arc<Mutex<AppState>>,
    screen_x: f32,
    screen_y: f32,
) {
    let st = state.lock();

    let tool = match &st.brush.active_tool {
        Some(t) => t.clone(),
        None => return,
    };

    // Convert screen → world coordinates using the current viewport transform
    let (wx, wy) = st.viewport.screen_to_world(screen_x, screen_y);

    let params = BrushParamsGpu {
        center:         [wx, wy],
        radius:         st.brush.radius,
        // Lower tool passes negative strength so one shader handles both
        strength: match tool {
            crate::state::BrushTool::Lower => -st.brush.strength,
            _ => st.brush.strength,
        },
        flatten_target: st.brush.flatten_target,
        noise_scale:    st.brush.noise_scale,
        _pad: [0.0; 2],
    };

    drop(st); // release lock before GPU work

    // Upload brush params to the uniform buffer
    ctx.queue.write_buffer(
        &ctx.brush_params_buffer,
        0,
        bytemuck::bytes_of(&params),
    );

    ctx.dispatch_compute_brush(&tool);
}
