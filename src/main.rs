#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod accent;
mod api;
mod app;
mod backdrop;
mod config;
mod gamepad;
mod i18n;
mod launch;
mod library;
mod model;
mod prefix;
mod prefs;
mod scan;
mod theme;
mod ui;
mod vdf;

fn main() -> eframe::Result<()> {
    let cfg = config::Config::load();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Auera")
        .with_inner_size(cfg.window_size)
        .with_min_inner_size([980.0, 620.0])
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(true);
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "Auera",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}

fn window_icon() -> Option<egui::IconData> {
    let image = image::load_from_memory(include_bytes!("../assets/icon-normal.png"))
        .ok()?
        .to_rgba8();
    Some(egui::IconData {
        rgba: image.as_raw().clone(),
        width: image.width(),
        height: image.height(),
    })
}
