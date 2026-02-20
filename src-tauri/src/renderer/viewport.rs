// ── viewport.rs ───────────────────────────────────────────────────────────────
// Render-thread viewport helpers (pan clamping, etc.).
// The heavy lifting (zoom_at, screen_to_world, fit_world) lives in state.rs.

use crate::state::ViewportState;

/// Apply a pan delta, then clamp so the world stays visible.
pub fn pan(vp: &mut ViewportState, dx: f32, dy: f32) {
    vp.translate[0] += dx;
    vp.translate[1] += dy;
    clamp(vp);
}

/// Ensure the world rectangle doesn't fly completely off screen.
/// At minimum, half the world dimension must remain on screen.
pub fn clamp(vp: &mut ViewportState) {
    let world_w = vp.canvas_size[0]; // worst-case: treat world as full canvas
    let world_h = vp.canvas_size[1];
    let margin_x = (world_w * vp.scale * 0.5).min(vp.canvas_size[0] * 0.5);
    let margin_y = (world_h * vp.scale * 0.5).min(vp.canvas_size[1] * 0.5);

    vp.translate[0] = vp.translate[0]
        .max(-(world_w * vp.scale - margin_x))
        .min(vp.canvas_size[0] - margin_x);

    vp.translate[1] = vp.translate[1]
        .max(-(world_h * vp.scale - margin_y))
        .min(vp.canvas_size[1] - margin_y);
}
