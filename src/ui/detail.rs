use std::path::PathBuf;

use egui::{vec2, Rect, RichText, ScrollArea, Stroke, Ui, UiBuilder};

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
    let hero = Rect::from_min_size(area.min, vec2(area.width(), 210.0));
    paint_hero(app, ui, hero, &game);

    // Cabecera sobre el arte
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(hero.shrink2(vec2(26.0, 20.0)))
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            if widgets::icon_button(ui, widgets::Icon::ChevronLeft, vec2(34.0, 34.0), false).clicked() {
                actions.push(Action::Back);
            }
        },
    );

    let title_rect = Rect::from_min_max(
        egui::pos2(hero.min.x + 44.0, hero.max.y - 84.0),
        egui::pos2(hero.max.x - 44.0, hero.max.y - 20.0),
    );
    ui.scope_builder(UiBuilder::new().max_rect(title_rect), |ui| {
        ui.label(RichText::new(&game.title).size(34.0).strong());
        ui.label(
            RichText::new(format!(
                "{} · {} · {}",
                game.source.label(),
                game.platform.label(lang),
                if game.compatible() {
                    tr(lang, Txt::Compatible)
                } else {
                    tr(lang, Txt::NotCompatible)
                }
            ))
            .size(12.0)
            .color(theme::MUTED),
        );
    });

    let body = Rect::from_min_max(egui::pos2(area.min.x, hero.max.y + 16.0), area.max);
    ui.scope_builder(UiBuilder::new().max_rect(body), |ui| {
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            action_row(app, ui, &game, actions);
            ui.add_space(6.0);
            info_card(app, ui, &mut game, actions);
            ui.add_space(6.0);
            compat_card(app, ui, &mut game, actions);
            ui.add_space(6.0);
            art_card(app, ui, &mut game, actions);
            ui.add_space(12.0);
        });
    });

    let mut changed = false;
    if let Some(stored) = app.library.get_mut(id) {
        changed |= stored.title != game.title;
        changed |= stored.platform != game.platform;
        changed |= stored.platform_locked != game.platform_locked;
        changed |= stored.blocked != game.blocked;
        changed |= stored.proton != game.proton;
        changed |= stored.prefix != game.prefix;
        stored.title = game.title.clone();
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

fn paint_hero(app: &mut App, ui: &Ui, rect: Rect, game: &Game) {
    let painter = ui.painter();
    let texture = app
        .textures
        .get(game.art.hero.as_deref())
        .map(|t| (t.handle.id(), t.size));
    match texture {
        Some((texture_id, size)) => widgets::texture_rounded(painter, rect, 16.0, texture_id, size),
        None => {
            painter.add(egui::Shape::convex_polygon(
                widgets::rounded_points(rect, 16.0),
                theme::SURFACE_HIGH,
                Stroke::NONE,
            ));
        }
    }
    widgets::gradient(
        painter,
        Rect::from_min_max(egui::pos2(rect.min.x, rect.center().y), rect.max),
        egui::Color32::TRANSPARENT,
        egui::Color32::from_black_alpha(230),
        true,
    );

    if let Some(texture) = app.textures.get(game.art.logo.as_deref()) {
        let width = 260.0f32.min(rect.width() * 0.34);
        let size = vec2(width, width * texture.size.y / texture.size.x.max(1.0));
        widgets::texture_rounded(
            painter,
            Rect::from_min_size(egui::pos2(rect.max.x - width - 34.0, rect.min.y + 24.0), size),
            4.0,
            texture.handle.id(),
            texture.size,
        );
    }
}

fn action_row(app: &mut App, ui: &mut Ui, game: &Game, actions: &mut Vec<Action>) {
    let lang = app.cfg.lang;
    ui.horizontal(|ui| {
        if game.compatible() {
            if widgets::pill(ui, tr(lang, Txt::Play), true, 138.0).clicked() {
                actions.push(Action::Play(game.id.clone()));
            }
        }
        if widgets::pill(ui, tr(lang, Txt::OpenFolder), false, 132.0).clicked() {
            actions.push(Action::OpenFolder(game.id.clone()));
        }
        if widgets::icon_button(ui, widgets::Icon::Star, vec2(38.0, 38.0), game.favorite).clicked() {
            actions.push(Action::Favorite(game.id.clone()));
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if app.confirm_delete {
                if widgets::pill(ui, tr(lang, Txt::ConfirmDelete), false, 210.0).clicked() {
                    actions.push(Action::Remove(game.id.clone()));
                    app.confirm_delete = false;
                }
            } else if widgets::icon_button(ui, widgets::Icon::Trash, vec2(38.0, 38.0), false).clicked() {
                app.confirm_delete = true;
            }
        });
    });
}

fn info_card(app: &mut App, ui: &mut Ui, game: &mut Game, actions: &mut Vec<Action>) {
    let lang = app.cfg.lang;
    widgets::card(ui, |ui| {
        widgets::section_title(ui, tr(lang, Txt::Game));
        let title_width = (ui.available_width() - 200.0).max(200.0);
        ui.add(
            egui::TextEdit::singleline(&mut game.title)
                .desired_width(title_width)
                .font(egui::FontId::proportional(16.0)),
        );

        ui.add_space(6.0);
        egui::Grid::new("game_fields")
            .num_columns(4)
            .spacing([18.0, 8.0])
            .show(ui, |ui| {
                field(ui, tr(lang, Txt::Source), game.source.label());
                field(ui, tr(lang, Txt::Playtime), &game.playtime_label());
                ui.end_row();
                field(ui, tr(lang, Txt::Size), &game.size_label());
                field(
                    ui,
                    tr(lang, Txt::Platform),
                    game.platform.label(lang),
                );
                ui.end_row();
            });

        ui.horizontal(|ui| {
            if widgets::chip(ui, tr(lang, Txt::Windows), game.platform == Platform::Windows).clicked() {
                game.platform = Platform::Windows;
                game.platform_locked = true;
            }
            if widgets::chip(ui, tr(lang, Txt::Linux), game.platform == Platform::Linux).clicked() {
                game.platform = Platform::Linux;
                game.platform_locked = true;
            }
            ui.checkbox(&mut game.platform_locked, tr(lang, Txt::LockPlatform));
            ui.checkbox(&mut game.blocked, tr(lang, Txt::BlockOnSystem));
            if widgets::pill(ui, tr(lang, Txt::ComputeSize), false, 140.0).clicked() {
                actions.push(Action::ComputeSize(game.id.clone()));
            }
        });

        ui.add_space(6.0);
        egui::Grid::new("game_paths")
            .num_columns(2)
            .spacing([18.0, 8.0])
            .show(ui, |ui| {
                let executable = game
                    .executable
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "—".into());
                field(ui, tr(lang, Txt::Executable), &executable);
                ui.end_row();
                field(ui, tr(lang, Txt::InstalledIn), &game.install_dir.display().to_string());
                ui.end_row();
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
                ui.label(
                    RichText::new(tr(lang, Txt::Prefix))
                        .size(11.5)
                        .color(theme::MUTED),
                );

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
                        .desired_width(430.0),
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

                let (dot, _) = ui.allocate_exact_size(vec2(10.0, 10.0), egui::Sense::hover());
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
                ui.label(
                    RichText::new(tr(lang, Txt::Proton))
                        .size(11.5)
                        .color(theme::MUTED),
                );
                let current = game
                    .proton
                    .clone()
                    .unwrap_or_else(|| "Auto".to_string());
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
                    ui.label(
                        RichText::new("—")
                            .size(11.5)
                            .color(theme::DANGER),
                    );
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
            let cover = Rect::from_min_size(ui.cursor().min, vec2(96.0, 136.0));
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
            ui.allocate_space(vec2(96.0, 136.0));

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let hint = tr(lang, Txt::SgdbNameHint);
                    ui.add(
                        egui::TextEdit::singleline(&mut app.art_query)
                            .hint_text(hint)
                            .desired_width(240.0),
                    );
                    if widgets::pill(ui, tr(lang, Txt::SearchArt), false, 120.0).clicked() {
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
                    if widgets::pill(ui, tr(lang, Txt::RefreshArt), false, 148.0).clicked() {
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

fn field(ui: &mut Ui, label: &str, value: &str) {
    ui.label(RichText::new(label).size(11.5).color(theme::MUTED));
    ui.label(RichText::new(value).size(12.5));
}
