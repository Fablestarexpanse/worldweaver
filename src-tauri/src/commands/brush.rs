// ── commands/brush.rs ─────────────────────────────────────────────────────────
// Tauri IPC commands for changing brush tool, parameters, and undo.

use std::sync::Arc;
use parking_lot::Mutex;
use tauri::State;
use serde::Deserialize;

use crate::state::{AppState, BrushTool};

type SharedState = Arc<Mutex<AppState>>;

/// Switch the active brush tool (or pass null to deactivate).
#[tauri::command]
pub fn set_active_tool(
    state: State<'_, SharedState>,
    tool: Option<BrushTool>,
) {
    state.lock().brush.active_tool = tool;
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrushParams {
    pub radius:         Option<f32>,
    pub strength:       Option<f32>,
    pub flatten_target: Option<f32>,
    pub noise_scale:    Option<f32>,
}

/// Update one or more brush parameters from the UI panel.
#[tauri::command]
pub fn set_brush_params(
    state: State<'_, SharedState>,
    params: BrushParams,
) {
    let mut st = state.lock();
    if let Some(r)  = params.radius         { st.brush.radius         = r; }
    if let Some(s)  = params.strength       { st.brush.strength       = s; }
    if let Some(ft) = params.flatten_target { st.brush.flatten_target = ft; }
    if let Some(ns) = params.noise_scale    { st.brush.noise_scale    = ns; }
}

/// Undo the most recent brush stroke.
/// The actual GPU texture restore is done in the render thread via undo_gpu.
/// Here we just signal the intent — the render thread polls undo state.
///
/// NOTE: Because GPU texture readback requires the render thread context,
/// the actual undo is performed by posting a flag and letting the render loop handle it.
#[tauri::command]
pub fn undo_stroke(
    state: State<'_, SharedState>,
) -> Result<(), String> {
    // The undo stack popping is handled in the render thread (undo_gpu::apply_undo).
    // We set a flag here; the render loop checks it.
    state.lock().undo_stack.pending_undo = true;
    Ok(())
}
