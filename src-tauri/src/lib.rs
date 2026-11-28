// El núcleo (escaneo, APIs, modelo, prefijos, lanzamiento) vive junto al
// backend de Tauri; la interfaz principal es Tauri.
pub mod accent;
pub mod api;
pub mod backdrop;
pub mod config;
pub mod gamepad;
pub mod i18n;
pub mod launch;
pub mod library;
pub mod model;
pub mod prefix;
pub mod prefs;
pub mod scan;
pub mod vdf;

mod commands;
mod media;
mod service;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("auera", |_ctx, request| media::respond(&request))
        .setup(|app| {
            let service = service::Service::new(app.handle().clone());
            app.manage(service);
            service::start_background(app.handle().clone());

            if let Some(window) = app.get_webview_window("main") {
                let cfg = config::Config::load();
                let size = tauri::LogicalSize::new(
                    cfg.window_size[0] as f64,
                    cfg.window_size[1] as f64,
                );
                let _ = window.set_size(size);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::scan,
            commands::fetch_art,
            commands::fetch_tiers,
            commands::search_art,
            commands::pick_art,
            commands::compute_size,
            commands::create_prefix,
            commands::detect_prefix,
            commands::pick_prefix,
            commands::pick_executable,
            commands::play,
            commands::accent,
            commands::backdrop,
            commands::refresh_backdrop,
            commands::warm_media,
            commands::set_config,
            commands::set_prefs,
            commands::update_game,
            commands::remove_game,
            commands::toggle_favorite,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
