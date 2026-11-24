use std::path::PathBuf;

use egui::{pos2, vec2, Align2, FontId, Rect, RichText, ScrollArea, Sense, Stroke, Ui, UiBuilder};

use crate::app::App;
use crate::i18n::{tr, Txt};
use crate::model::{Game, Platform, Tier};
use crate::prefix;
use crate::theme;
use crate::ui::{widgets, Action};

pub fn show(app: &mut App, ui: &mut Ui, actions: &mut Vec<Action>, id: &str) {
    let lang = app.cfg.lang;
    let Some(mut game) = app.library.get(id).cloned() else {
        actions.push(Action::Back);
        return;
    };
    let area = ui.available_rect_before_wrap();

    ui.scope_builder(
        UiBuilder::new().max_rect(Rect::from_min_size(area.min, vec2(40.0, 40.0))),
        |ui| {
            if widgets::icon_button(ui, widgets::Icon::ChevronLeft, vec2(36.0, 36.0), false).clicked() {
                actions.push(Action::Back);
            }
        },
    );

    // Logo del juego a la izquierda, en el centro vertical.
    let logo_width = 320.0f32.min(area.width() * 0.36);
    let logo_bottom = match app.textures.get(game.art.logo.as_deref()) {
        Some(texture) => {
            let height = (logo_width * texture.size.y / texture.size.x.max(1.0)).min(190.0);
            let rect = Rect::from_min_size(
                pos2(area.min.x + 40.0, area.center().y - height - 40.0),
                vec2(logo_width, height),
            );
            // Sombra suave: el logo se lee sobre arte claro sin añadir un panel.
            ui.painter().image(
                texture.handle.id(),
                rect.translate(vec2(2.0, 3.0)),
                Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::from_black_alpha(150),
            );
            widgets::texture_rounded(ui.painter(), rect, 4.0, texture.handle.id(), texture.size);
            rect.max.y
        }
        None => {
            let pos = pos2(area.min.x + 40.0, area.center().y - 80.0);
            ui.painter().text(
                pos,
                Align2::LEFT_CENTER,
                &game.title,
                FontId::proportional(40.0),
                theme::TEXT,
            );
            pos.y + 26.0
        }
    };

    let accent = app.accent_for(&game);
    let play_rect = Rect::from_min_size(
        pos2(area.min.x + 40.0, logo_bottom + 22.0),
        vec2(152.0, 46.0),
    );
    let play = ui.interact(play_rect, egui::Id::new("play"), Sense::click());
    let (fill, text_color) = if !game.compatible() {
        (theme::SURFACE_HIGH, theme::MUTED)
    } else if play.hovered() {
        (lighten(accent, 0.18), theme::on_color(accent))
    } else {
        (accent, theme::on_color(accent))
    };
    ui.painter().add(egui::Shape::convex_polygon(
        widgets::rounded_points(play_rect, 23.0),
        fill,
        Stroke::NONE,
    ));
    ui.painter().text(
        play_rect.center(),
        Align2::CENTER_CENTER,
        tr(lang, Txt::Play),
        FontId::proportional(15.0),
        text_color,
    );
    if play.clicked() && game.compatible() {
        actions.push(Action::Play(game.id.clone()));
    }

    let options_rect = Rect::from_min_size(
        pos2(play_rect.max.x + 12.0, play_rect.min.y),
        vec2(46.0, 46.0),
    );
    let options = ui.interact(options_rect, egui::Id::new("game_options"), Sense::click());
    let (bg, border) = if options.hovered() {
        (
            egui::Color32::from_white_alpha(30),
            egui::Color32::from_white_alpha(50),
        )
    } else {
        (
            egui::Color32::from_black_alpha(130),
            egui::Color32::from_white_alpha(22),
        )
    };
    ui.painter().add(egui::Shape::convex_polygon(
        widgets::rounded_points(options_rect, 23.0),
        bg,
        Stroke::new(1.0_f32, border),
    ));
    widgets::icon(
        ui.painter(),
        Rect::from_center_size(options_rect.center(), vec2(20.0, 20.0)),
        widgets::Icon::Gear,
        theme::TEXT,
        1.6,
    );
    if options.clicked() {
        app.game_dialog = true;
    }

    paint_info_card(ui, app, &game, area);
    game_dialog(app, ui, &mut game, actions);

    let mut changed = false;
    if let Some(stored) = app.library.get_mut(id) {
        changed |= stored.title != game.title;
        changed |= stored.executable != game.executable;
        changed |= stored.install_dir != game.install_dir;
        changed |= stored.platform != game.platform;
        changed |= stored.platform_locked != game.platform_locked;
        changed |= stored.blocked != game.blocked;
        changed |= stored.proton != game.proton;
        changed |= stored.prefix != game.prefix;
        stored.title = game.title.clone();
        stored.executable = game.executable.clone();
        stored.install_dir = game.install_dir.clone();
        stored.platform = game.platform;
        stored.platform_locked = game.platform_locked;
        stored.blocked = game.blocked;
        stored.proton = game.proton.clone();
        stored.prefix = game.prefix.clone();
    }
    if changed {
        app.library.save();
    }
}

/// Tarjeta inferior derecha con el resumen del juego.
fn paint_info_card(ui: &Ui, app: &App, game: &Game, area: Rect) {
    let lang = app.cfg.lang;
    let size = vec2(256.0, 92.0);
    let rect = Rect::from_min_size(pos2(area.max.x - size.x, area.max.y - size.y), size);
    let painter = ui.painter();
    painter.add(egui::Shape::convex_polygon(
        widgets::rounded_points(rect, 16.0),
        egui::Color32::from_black_alpha(155),
        Stroke::new(1.0_f32, egui::Color32::from_white_alpha(18)),
    ));

    let cells = [
        (tr(lang, Txt::Source), game.source.label().to_string()),
        (tr(lang, Txt::Size), game.size_label()),
        (tr(lang, Txt::Playtime), game.playtime_label()),
        (
            tr(lang, Txt::Platform),
            game.platform.label(lang).to_string(),
        ),
    ];
    let column = (rect.width() - 34.0) / 2.0;
    for (index, (label, value)) in cells.iter().enumerate() {
        let x = rect.min.x + 16.0 + (index % 2) as f32 * (column + 4.0);
        let y = rect.min.y + 16.0 + (index / 2) as f32 * 38.0;
        painter.text(
            pos2(x, y),
            Align2::LEFT_TOP,
            label,
            FontId::proportional(10.5),
            theme::MUTED,
        );
        painter.text(
            pos2(x, y + 15.0),
            Align2::LEFT_TOP,
            value,
            FontId::proportional(12.5),
            theme::TEXT,
        );
    }
}

fn game_dialog(app: &mut App, ui: &mut Ui, game: &mut Game, actions: &mut Vec<Action>) {
    if !app.game_dialog {
        return;
    }
    let lang = app.cfg.lang;
    let mut close = false;

    egui::Window::new(tr(lang, Txt::GameOptions))
        .id(egui::Id::new("game_dialog"))
        .collapsible(false)
        .resizable(false)
        .default_width(650.0)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(ui.ctx(), |ui| {
            ui.set_width(610.0);
            ScrollArea::vertical()
                .max_height(430.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    info_card(app, ui, game, actions);
                    ui.add_space(6.0);
                    compat_card(app, ui, game, actions);
                    ui.add_space(6.0);
                    art_card(app, ui, game, actions);
                });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                delete_button(app, ui, game, actions);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::pill(ui, tr(lang, Txt::Close), true, 112.0).clicked() {
                        close = true;
                    }
                });
            });
        });

    if close {
        app.game_dialog = false;
    }
}

fn info_card(app: &mut App, ui: &mut Ui, game: &mut Game, actions: &mut Vec<Action>) {
    let lang = app.cfg.lang;
    widgets::card(ui, |ui| {
        widgets::section_title(ui, tr(lang, Txt::Game));
        ui.add(
            egui::TextEdit::singleline(&mut game.title)
                .desired_width(ui.available_width())
                .font(FontId::proportional(16.0)),
        );
        ui.add_space(6.0);

        egui::Grid::new("game_paths")
            .num_columns(2)
            .spacing([14.0, 8.0])
            .show(ui, |ui| {
                field_label(ui, tr(lang, Txt::Executable));
                if let Some(path) = path_field(ui, game.executable.as_deref(), "…/juego.exe") {
                    game.executable = (!path.as_os_str().is_empty()).then_some(path);
                }
                ui.end_row();
                field_label(ui, tr(lang, Txt::InstalledIn));
                if let Some(path) = path_field(ui, Some(game.install_dir.as_path()), "…/carpeta del juego") {
                    if !path.as_os_str().is_empty() {
                        game.install_dir = path;
                    }
                }
                ui.end_row();
            });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let windows = game.platform == Platform::Windows;
            if widgets::chip(ui, tr(lang, Txt::Windows), windows).clicked() {
                game.platform = Platform::Windows;
            }
            if widgets::chip(ui, tr(lang, Txt::Linux), !windows).clicked() {
                game.platform = Platform::Linux;
            }
            if widgets::chip(ui, tr(lang, Txt::LockPlatform), game.platform_locked).clicked() {
                game.platform_locked = !game.platform_locked;
            }
            if widgets::chip(ui, tr(lang, Txt::BlockOnSystem), game.blocked).clicked() {
                game.blocked = !game.blocked;
            }
            if widgets::pill(ui, tr(lang, Txt::ComputeSize), false, 158.0).clicked() {
                actions.push(Action::ComputeSize(game.id.clone()));
            }
        });
    });
}

fn compat_card(app: &mut App, ui: &mut Ui, game: &mut Game, actions: &mut Vec<Action>) {
    let lang = app.cfg.lang;
    widgets::card(ui, |ui| {
        ui.horizontal(|ui| {
            widgets::section_title(ui, tr(lang, Txt::Compatibility));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let color = theme::tier_color(game.tier);
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        tr(lang, Txt::Tier),
                        if game.tier == Tier::Unknown {
                            tr(lang, Txt::NoTier)
                        } else {
                            game.tier.label()
                        }
                    ))
                    .size(12.0)
                    .color(color),
                );
            });
        });

        if cfg!(unix) {
            ui.horizontal(|ui| {
                field_label(ui, tr(lang, Txt::Prefix));

                // Ruta editable: se aplica al escribir y queda guardada en la biblioteca.
                let mut text = game
                    .prefix
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default();
                let response = ui.add(
                    egui::TextEdit::singleline(&mut text)
                        .id(egui::Id::new("prefix_path"))
                        .hint_text("/ruta/al/pfx")
                        .desired_width(300.0),
                );
                if response.changed() {
                    let trimmed = text.trim();
                    game.prefix = (!trimmed.is_empty()).then(|| PathBuf::from(trimmed));
                }
                if response.lost_focus() {
                    if let Some(path) = &game.prefix {
                        let expanded = prefix::expand(&path.to_string_lossy());
                        if expanded != *path {
                            game.prefix = Some(expanded);
                        }
                    }
                }

                let (dot, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                let color = match &game.prefix {
                    Some(path) if path.is_dir() => theme::OK,
                    Some(_) => theme::DANGER,
                    None => theme::MUTED,
                };
                ui.painter().circle_filled(dot.center(), 4.0, color);

                if widgets::chip(ui, tr(lang, Txt::DetectPrefix), false).clicked() {
                    actions.push(Action::DetectPrefix(game.id.clone()));
                }
                if widgets::chip(ui, tr(lang, Txt::CreatePrefix), false).clicked() {
                    actions.push(Action::CreatePrefix(game.id.clone()));
                }
                if game.prefix.is_some()
                    && widgets::chip(ui, tr(lang, Txt::ClearPrefix), false).clicked()
                {
                    game.prefix = None;
                }
            });

            ui.horizontal(|ui| {
                field_label(ui, tr(lang, Txt::Proton));
                let current = game.proton.clone().unwrap_or_else(|| "Auto".to_string());
                egui::ComboBox::from_id_salt("proton_select")
                    .selected_text(RichText::new(current).size(12.0))
                    .show_ui(ui, |ui| {
                        for proton in &app.protons {
                            let selected = game.proton.as_deref() == Some(proton.name.as_str());
                            if ui.selectable_label(selected, &proton.name).clicked() {
                                game.proton = Some(proton.name.clone());
                            }
                        }
                        if ui.selectable_label(game.proton.is_none(), "Auto").clicked() {
                            game.proton = None;
                        }
                    });
                if app.protons.is_empty() {
                    ui.label(RichText::new("—").size(11.5).color(theme::DANGER));
                }
            });
        }
    });
}

fn art_card(app: &mut App, ui: &mut Ui, game: &mut Game, actions: &mut Vec<Action>) {
    let lang = app.cfg.lang;
    widgets::card(ui, |ui| {
        widgets::section_title(ui, tr(lang, Txt::SteamGrid));
        ui.horizontal(|ui| {
            let cover = Rect::from_min_size(ui.cursor().min, vec2(72.0, 104.0));
            let texture = app
                .textures
                .get(game.art.cover.as_deref())
                .map(|t| (t.handle.id(), t.size));
            match texture {
                Some((texture_id, size)) => {
                    widgets::texture_rounded(ui.painter(), cover, 10.0, texture_id, size);
                }
                None => {
                    ui.painter().add(egui::Shape::convex_polygon(
                        widgets::rounded_points(cover, 10.0),
                        theme::SURFACE_HIGH,
                        Stroke::NONE,
                    ));
                }
            }
            ui.allocate_space(vec2(72.0, 104.0));

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let hint = tr(lang, Txt::SgdbNameHint);
                    ui.add(
                        egui::TextEdit::singleline(&mut app.art_query)
                            .hint_text(hint)
                            .desired_width(170.0),
                    );
                    if widgets::pill(ui, tr(lang, Txt::SearchArt), false, 104.0).clicked() {
                        let query = if app.art_query.trim().is_empty() {
                            game.title.clone()
                        } else {
                            app.art_query.clone()
                        };
                        actions.push(Action::SearchArt {
                            id: game.id.clone(),
                            query,
                        });
                    }
                    if widgets::pill(ui, tr(lang, Txt::RefreshArt), false, 138.0).clicked() {
                        actions.push(Action::RefreshArt(game.id.clone()));
                    }
                });

                if app.candidates_for.as_deref() == Some(game.id.as_str()) {
                    ui.add_space(4.0);
                    let results = app.candidates.clone();
                    for (sgdb_id, name) in results {
                        if ui
                            .add(egui::Button::new(RichText::new(name).size(12.0)).frame(false))
                            .clicked()
                        {
                            actions.push(Action::PickArt {
                                id: game.id.clone(),
                                sgdb_id,
                            });
                        }
                    }
                }
            });
        });
    });
}

fn delete_button(app: &mut App, ui: &mut Ui, game: &Game, actions: &mut Vec<Action>) {
    let lang = app.cfg.lang;
    if app.confirm_delete {
        if widgets::pill(ui, tr(lang, Txt::ConfirmDelete), false, 210.0).clicked() {
            actions.push(Action::Remove(game.id.clone()));
            app.confirm_delete = false;
        }
    } else if widgets::icon_button(ui, widgets::Icon::Trash, vec2(38.0, 38.0), false).clicked() {
        app.confirm_delete = true;
    }
}

/// Campo de ruta; devuelve la ruta nueva cuando se edita o se sale del campo.
fn path_field(ui: &mut Ui, current: Option<&std::path::Path>, hint: &str) -> Option<PathBuf> {
    let mut text = current
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    let width = (ui.available_width() - 120.0).max(240.0);
    let response = ui.add(
        egui::TextEdit::singleline(&mut text)
            .hint_text(hint)
            .desired_width(width),
    );
    if response.changed() {
        return Some(PathBuf::from(text.trim()));
    }
    if response.lost_focus() && !text.trim().is_empty() {
        return Some(prefix::expand(text.trim()));
    }
    None
}

fn field_label(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(11.5).color(theme::MUTED));
}

fn lighten(color: egui::Color32, amount: f32) -> egui::Color32 {
    let mix = |channel: u8| (channel as f32 + (255.0 - channel as f32) * amount) as u8;
    egui::Color32::from_rgb(mix(color.r()), mix(color.g()), mix(color.b()))
}
