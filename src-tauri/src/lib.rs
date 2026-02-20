mod commands;
mod renderer;
mod state;
mod terrain;

use std::sync::Arc;
use parking_lot::Mutex;
use state::AppState;

pub fn run() {
    env_logger::init();

    let app_state = Arc::new(Mutex::new(AppState::default()));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(app_state.clone())
        .setup(move |app| {
            // The UI panel window is declared in tauri.conf.json.
            // Spawn the wgpu render window on a dedicated thread so it owns
            // the surface lifecycle without competing with the WebView.
            let app_handle = app.handle().clone();
            let state_clone = app_state.clone();

            std::thread::spawn(move || {
                renderer::run_render_window(app_handle, state_clone);
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::terrain::generate_terrain,
            commands::terrain::get_terrain_config,
            commands::terrain::save_world,
            commands::terrain::load_world,
            commands::terrain::generate_volcanoes,
            commands::brush::set_active_tool,
            commands::brush::set_brush_params,
            commands::brush::undo_stroke,
            commands::viewport::set_viewport_transform,
            commands::viewport::reset_view,
        ])
        .run(tauri::generate_context!())
        .expect("error while running WorldWeaver");
}
