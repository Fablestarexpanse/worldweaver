// ── commands/terrain.rs ───────────────────────────────────────────────────────
// Tauri IPC commands for terrain generation, persistence, and settings.

use std::sync::Arc;
use parking_lot::Mutex;
use tauri::State;
use serde::Serialize;

use crate::state::AppState;
use crate::terrain::{self, TerrainConfig};
use crate::terrain::volcanoes::{self, VolcanoConfig};

type SharedState = Arc<Mutex<AppState>>;

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct GenerateResult {
    pub world_width:  u32,
    pub world_height: u32,
    pub sea_level:    f32,
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// Generate a new terrain from the given config and store it in AppState.
/// The render thread will see `dirty = true` on the next frame and upload
/// the heightmap to the GPU automatically.
#[tauri::command]
pub async fn generate_terrain(
    state: State<'_, SharedState>,
    config: TerrainConfig,
) -> Result<GenerateResult, String> {
    let cfg = config.clone();
    let result = tokio::task::spawn_blocking(move || terrain::generate(cfg))
        .await
        .map_err(|e| format!("task join: {e}"))?
        .map_err(|e| format!("generate: {e}"))?;

    let w = result.config.world_width;
    let h = result.config.world_height;
    let sl = result.config.sea_level;

    {
        let mut st = state.lock();
        // Fit viewport to new world size
        st.viewport.fit_world(w as f32, h as f32);
        st.terrain = Some(result);
    }

    Ok(GenerateResult { world_width: w, world_height: h, sea_level: sl })
}

/// Return the current terrain config (for UI display / editing).
#[tauri::command]
pub fn get_terrain_config(
    state: State<'_, SharedState>,
) -> Option<TerrainConfig> {
    state.lock().terrain.as_ref().map(|t| t.config.clone())
}

/// Save the current world to disk.
#[tauri::command]
pub async fn save_world(
    state: State<'_, SharedState>,
    path: String,
) -> Result<(), String> {
    let data_snapshot = {
        let st = state.lock();
        st.terrain.as_ref().map(|t| t.clone())
    };

    let data = data_snapshot.ok_or("no terrain to save")?;
    tokio::task::spawn_blocking(move || crate::terrain::persistence::save(&path, &data))
        .await
        .map_err(|e| format!("task: {e}"))?
        .map_err(|e| format!("save: {e}"))
}

/// Load a world from disk.
#[tauri::command]
pub async fn load_world(
    state: State<'_, SharedState>,
    path: String,
) -> Result<GenerateResult, String> {
    let data = tokio::task::spawn_blocking(move || crate::terrain::persistence::load(&path))
        .await
        .map_err(|e| format!("task: {e}"))?
        .map_err(|e| format!("load: {e}"))?;

    let w  = data.config.world_width;
    let h  = data.config.world_height;
    let sl = data.config.sea_level;

    {
        let mut st = state.lock();
        st.viewport.fit_world(w as f32, h as f32);
        st.terrain = Some(data);
    }

    Ok(GenerateResult { world_width: w, world_height: h, sea_level: sl })
}

/// Stamp volcanic cones onto the current terrain.
#[tauri::command]
pub async fn generate_volcanoes(
    state: State<'_, SharedState>,
    config: VolcanoConfig,
) -> Result<(), String> {
    let mut st = state.lock();
    let terrain = st.terrain.as_mut().ok_or("no terrain")?;
    let seed = terrain.config.seed;
    let w = terrain.config.world_width as usize;
    let h = terrain.config.world_height as usize;
    volcanoes::apply(&mut terrain.heights, w, h, &config, seed);
    terrain.dirty = true;
    Ok(())
}
