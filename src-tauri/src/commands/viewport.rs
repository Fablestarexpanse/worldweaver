// ── commands/viewport.rs ──────────────────────────────────────────────────────
// Tauri IPC commands for viewport control from the UI panel.

use std::sync::Arc;
use parking_lot::Mutex;
use tauri::State;
use serde::Deserialize;

use crate::state::AppState;

type SharedState = Arc<Mutex<AppState>>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewportTransform {
    pub translate_x: Option<f32>,
    pub translate_y: Option<f32>,
    pub scale:       Option<f32>,
}

/// Called by the UI if it needs to programmatically set the viewport
/// (e.g. to centre on a specific world coordinate).
#[tauri::command]
pub fn set_viewport_transform(
    state: State<'_, SharedState>,
    transform: ViewportTransform,
) {
    let mut st = state.lock();
    if let Some(tx) = transform.translate_x { st.viewport.translate[0] = tx; }
    if let Some(ty) = transform.translate_y { st.viewport.translate[1] = ty; }
    if let Some(sc) = transform.scale       { st.viewport.scale        = sc; }
}

/// Reset the viewport to fit the current world.
#[tauri::command]
pub fn reset_view(state: State<'_, SharedState>) {
    let mut st = state.lock();
    // Extract dimensions first to avoid split-borrow conflict
    let dims = st.terrain.as_ref().map(|t| {
        (t.config.world_width as f32, t.config.world_height as f32)
    });
    if let Some((ww, wh)) = dims {
        st.viewport.fit_world(ww, wh);
    }
}
