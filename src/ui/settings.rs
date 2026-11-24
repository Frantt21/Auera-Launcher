use egui::{RichText, ScrollArea, Ui, UiBuilder};

use crate::app::App;
use crate::config::Lang;
use crate::i18n::{tr, Txt};
use crate::theme;
use crate::ui::{widgets, Action};

pub fn show(app: &mut App, ui: &mut Ui, actions: &mut Vec<Action>) {
    let lang = app.cfg.lang;
    let area = ui.available_rect_before_wrap();
    ui.scope_builder(UiBuilder::new().max_rect(area), |ui| {
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.label(RichText::new(tr(lang, Txt::Settings)).size(30.0).strong());
            ui.add_space(6.0);

            general(app, ui, lang);
            ui.add_space(6.0);
            launchers(app, ui, lang, actions);
            ui.add_space(6.0);
            watched_folders(app, ui, lang);
            ui.add_space(6.0);
            integrations(app, ui, lang, actions);
            ui.add_space(6.0);
            system(app, ui, lang);
            ui.add_space(6.0);
            log(app, ui, lang);
            ui.add_space(12.0);
        });
    });
}

fn general(app: &mut App, ui: &mut Ui, lang: Lang) {
    widgets::card(ui, |ui| {
        widgets::section_title(ui, tr(lang, Txt::General));
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr(lang, Txt::Language)).size(12.5));
            if widgets::chip(ui, "Español", app.cfg.lang == Lang::Es).clicked() {
                app.cfg.lang = Lang::Es;
                app.cfg.save();
            }
            if widgets::chip(ui, "English", app.cfg.lang == Lang::En).clicked() {
                app.cfg.lang = Lang::En;
                app.cfg.save();
            }
        });

        let mut gamepad = app.cfg.gamepad_enabled;
        if ui.checkbox(&mut gamepad, tr(lang, Txt::Gamepad)).changed() {
            app.cfg.gamepad_enabled = gamepad;
            app.cfg.save();
        }
        let status = match app.gamepad_name() {
            Some(name) => format!("{} · {}", tr(lang, Txt::GamepadDetected), name),
            None => tr(lang, Txt::GamepadMissing).to_string(),
        };
        ui.label(
            RichText::new(status)
                .size(11.5)
                .color(if app.gamepad_name().is_some() { theme::OK } else { theme::MUTED }),
        );
    });
}

fn launchers(app: &mut App, ui: &mut Ui, lang: Lang, actions: &mut Vec<Action>) {
    widgets::card(ui, |ui| {
        ui.horizontal(|ui| {
            widgets::section_title(ui, tr(lang, Txt::Source));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = if app.scanning {
                    tr(lang, Txt::Scanning)
                } else {
                    tr(lang, Txt::Scan)
                };
                if widgets::pill(ui, label, false, 118.0).clicked() && !app.scanning {
                    actions.push(Action::Scan);
                }
            });
        });

        let toggles = [
            (&mut app.cfg.scan_steam, Txt::ScanSteam),
            (&mut app.cfg.scan_epic, Txt::ScanEpic),
            (&mut app.cfg.scan_heroic, Txt::ScanHeroic),
            (&mut app.cfg.scan_hydra, Txt::ScanHydra),
            (&mut app.cfg.scan_desktop, Txt::ScanDesktop),
        ];
        let mut changed = false;
        for (value, label) in toggles {
            changed |= ui.checkbox(value, tr(lang, label)).changed();
        }
        if changed {
            app.cfg.save();
        }
    });
}

fn watched_folders(app: &mut App, ui: &mut Ui, lang: Lang) {
    widgets::card(ui, |ui| {
        widgets::section_title(ui, tr(lang, Txt::WatchedFolders));
        let folders = app.cfg.watched_folders.clone();
        for folder in &folders {
            ui.horizontal(|ui| {
                ui.label(RichText::new(folder.display().to_string()).size(12.0));
                if ui
                    .add(egui::Button::new(RichText::new(tr(lang, Txt::Remove)).size(11.0)).frame(false))
                    .clicked()
                {
                    app.cfg.watched_folders.retain(|f| f != folder);
                    app.cfg.save();
                }
            });
        }
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.folder_input)
                    .hint_text("/home/usuario/Juegos")
                    .desired_width(320.0),
            );
            if widgets::pill(ui, tr(lang, Txt::AddFolder), false, 132.0).clicked() {
                let path = std::path::PathBuf::from(app.folder_input.trim());
                if path.is_dir() && !app.cfg.watched_folders.contains(&path) {
                    app.cfg.watched_folders.push(path);
                    app.folder_input.clear();
                    app.cfg.save();
                }
            }
        });
    });
}

fn integrations(app: &mut App, ui: &mut Ui, lang: Lang, actions: &mut Vec<Action>) {
    widgets::card(ui, |ui| {
        widgets::section_title(ui, tr(lang, Txt::Integrations));
        let mut dirty = false;
        dirty |= ui
            .checkbox(&mut app.cfg.steamgrid_enabled, tr(lang, Txt::SteamGrid))
            .changed();
        dirty |= ui
            .add(
                egui::TextEdit::singleline(&mut app.cfg.steamgrid_key)
                    .hint_text(tr(lang, Txt::SteamGridKey))
                    .desired_width(360.0)
                    .password(true),
            )
            .changed();
        dirty |= ui
            .checkbox(&mut app.cfg.protondb_enabled, tr(lang, Txt::ProtonDb))
            .changed();
        if widgets::pill(ui, tr(lang, Txt::RefreshArt), false, 150.0).clicked() {
            actions.push(Action::FetchArt);
        }
        if dirty {
            app.cfg.save();
        }
    });
}

fn system(app: &mut App, ui: &mut Ui, lang: Lang) {
    widgets::card(ui, |ui| {
        widgets::section_title(ui, tr(lang, Txt::Proton));
        if app.protons.is_empty() {
            ui.label(RichText::new("—").size(12.0).color(theme::MUTED));
        }
        for proton in &app.protons {
            ui.label(RichText::new(&proton.name).size(12.0));
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new(format!("{}: {}", tr(lang, Txt::Platform), if cfg!(windows) { "Windows" } else { "Linux" }))
                .size(11.5)
                .color(theme::MUTED),
        );
        ui.label(
            RichText::new(crate::config::data_dir().display().to_string())
                .size(11.0)
                .color(theme::MUTED),
        );
    });
}

fn log(app: &mut App, ui: &mut Ui, lang: Lang) {
    widgets::card(ui, |ui| {
        widgets::section_title(ui, tr(lang, Txt::Log));
        let frame = egui::Frame::new()
            .fill(theme::SURFACE_HIGH)
            .corner_radius(egui::CornerRadius::same(8))
            .inner_margin(egui::Margin::same(10));
        frame.show(ui, |ui| {
            ScrollArea::vertical()
                .max_height(160.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in app.log.iter().rev().take(60).collect::<Vec<_>>().into_iter().rev() {
                        ui.label(RichText::new(line).monospace().size(11.0).color(theme::MUTED));
                    }
                });
        });
    });
}
