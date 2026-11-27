// El backend reutiliza los módulos del núcleo de la app egui mediante `#[path]`.
// Así ambos binarios comparten escaneo, APIs, modelo y lanzamiento sin duplicar
// código ni romper la app antigua mientras se porta la interfaz.
#[path = "../../src/accent.rs"]
pub mod accent;
#[path = "../../src/api/mod.rs"]
pub mod api;
#[path = "../../src/backdrop.rs"]
pub mod backdrop;
#[path = "../../src/config.rs"]
pub mod config;
#[path = "../../src/gamepad.rs"]
pub mod gamepad;
#[path = "../../src/i18n.rs"]
pub mod i18n;
#[path = "../../src/launch.rs"]
pub mod launch;
#[path = "../../src/library.rs"]
pub mod library;
#[path = "../../src/model.rs"]
pub mod model;
#[path = "../../src/prefix.rs"]
pub mod prefix;
#[path = "../../src/prefs.rs"]
pub mod prefs;
#[path = "../../src/scan/mod.rs"]
pub mod scan;
#[path = "../../src/vdf.rs"]
pub mod vdf;

mod commands;
mod media;
mod service;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
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
            commands::play,
            commands::accent,
            commands::backdrop,
            commands::set_config,
            commands::set_prefs,
            commands::update_game,
            commands::remove_game,
            commands::toggle_favorite,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
