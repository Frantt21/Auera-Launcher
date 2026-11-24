use egui::{vec2, Align, Align2, FontId, Layout, Rect, RichText, Sense, Stroke, Ui, UiBuilder};

use crate::app::{App, Sort};
use crate::i18n::{tr, Txt};
use crate::model::{Game, Platform, Source, Tier};
use crate::theme;
use crate::ui::{widgets, Action};

const CARD_WIDTH: f32 = 168.0;
const COVER_HEIGHT: f32 = 246.0;
const GAP: f32 = 16.0;

pub fn show(app: &mut App, ui: &mut Ui, actions: &mut Vec<Action>) {
    let lang = app.cfg.lang;
    let area = ui.available_rect_before_wrap();
    let header = Rect::from_min_size(area.min, vec2(area.width(), 118.0));

    ui.scope_builder(UiBuilder::new().max_rect(header), |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr(lang, Txt::Library)).size(30.0).strong());
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("{}", app.library.games.len()))
                    .size(13.0)
                    .color(theme::MUTED),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::pill(ui, tr(lang, Txt::Scan), false, 108.0).clicked() {
                    actions.push(Action::Scan);
                }
                let hint = tr(lang, Txt::Search);
                ui.add(
                    egui::TextEdit::singleline(&mut app.search)
                        .hint_text(hint)
                        .desired_width(230.0),
                );
            });
        });

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            // Filtro por launcher
            let all = app.source_filter.is_none();
            if widgets::chip(ui, tr(lang, Txt::AllSources), all).clicked() {
                app.source_filter = None;
            }
            for source in Source::ALL {
                if app.library.count_by_source(source) == 0 {
                    continue;
                }
                let active = app.source_filter == Some(source);
                if widgets::chip(ui, source.label(), active).clicked() {
                    app.source_filter = Some(source);
                }
            }

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);
            if widgets::chip(ui, tr(lang, Txt::AllPlatforms), app.platform_filter.is_none()).clicked() {
                app.platform_filter = None;
            }
            for platform in [Platform::Windows, Platform::Linux] {
                let active = app.platform_filter == Some(platform);
                if widgets::chip(ui, platform.label(lang), active).clicked() {
                    app.platform_filter = Some(platform);
                }
            }

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);
            if widgets::chip(ui, tr(lang, Txt::FavoritesOnly), app.favorites_only).clicked() {
                app.favorites_only = !app.favorites_only;
            }

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);
            sort_combo(app, ui, lang);
        });
    });

    let grid = Rect::from_min_max(
        egui::pos2(area.min.x, header.max.y + 6.0),
        area.max,
    );
    ui.scope_builder(UiBuilder::new().max_rect(grid), |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let width = ui.available_width();
                let columns = (((width + GAP) / (CARD_WIDTH + GAP)).floor() as usize).max(1);
                app.columns = columns;

                let games: Vec<Game> = app.filtered().into_iter().cloned().collect();
                if games.is_empty() {
                    ui.add_space(40.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new(tr(lang, Txt::NoGames)).size(20.0));
                        ui.label(
                            RichText::new(tr(lang, Txt::NoGamesHint))
                                .size(13.0)
                                .color(theme::MUTED),
                        );
                    });
                    return;
                }

                app.cursor = app.cursor.min(games.len() - 1);
                for (row_index, row) in games.chunks(columns).enumerate() {
                    ui.horizontal(|ui| {
                        for (column_index, game) in row.iter().enumerate() {
                            let index = row_index * columns + column_index;
                            let selected = app.selected.as_deref() == Some(game.id.as_str());
                            let response = card(app, ui, game, selected);
                            if response.clicked() {
                                actions.push(Action::Select(game.id.clone()));
                                actions.push(Action::Open(game.id.clone()));
                            }
                            if response.secondary_clicked() {
                                actions.push(Action::Favorite(game.id.clone()));
                            }
                            if response.hovered() {
                                actions.push(Action::Select(game.id.clone()));
                            }
                            if index == app.cursor && app.view == crate::app::View::Library {
                                let painter = ui.painter();
                                let rect = response.rect;
                                painter.add(egui::Shape::convex_polygon(
                                    widgets::rounded_points(rect, 12.0),
                                    egui::Color32::TRANSPARENT,
                                    Stroke::new(1.5_f32, theme::ACCENT.gamma_multiply(0.5)),
                                ));
                            }
                        }
                    });
                    ui.add_space(GAP);
                }
            });
    });
}

fn sort_combo(app: &mut App, ui: &mut Ui, lang: crate::config::Lang) {
    let label = match app.sort {
        Sort::Name => tr(lang, Txt::Name),
        Sort::Recent => tr(lang, Txt::Recent),
        Sort::Playtime => tr(lang, Txt::Playtime),
        Sort::Size => tr(lang, Txt::Size),
    };
    egui::ComboBox::from_id_salt("sort")
        .selected_text(RichText::new(label).size(12.5))
        .width(150.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut app.sort, Sort::Recent, tr(lang, Txt::Recent));
            ui.selectable_value(&mut app.sort, Sort::Name, tr(lang, Txt::Name));
            ui.selectable_value(&mut app.sort, Sort::Playtime, tr(lang, Txt::Playtime));
            ui.selectable_value(&mut app.sort, Sort::Size, tr(lang, Txt::Size));
        });
}

fn card(app: &mut App, ui: &mut Ui, game: &Game, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(CARD_WIDTH, COVER_HEIGHT + 34.0), Sense::click());
    let cover = Rect::from_min_size(rect.min, vec2(CARD_WIDTH, COVER_HEIGHT));
    let painter = ui.painter();

    let texture = app
        .textures
        .get(game.art.cover.as_deref())
        .map(|t| (t.handle.id(), t.size));
    match texture {
        Some((id, size)) => widgets::texture_rounded(painter, cover, 12.0, id, size),
        None => {
            painter.add(egui::Shape::convex_polygon(
                widgets::rounded_points(cover, 12.0),
                theme::SURFACE_HIGH,
                Stroke::NONE,
            ));
            painter.text(
                cover.center(),
                Align2::CENTER_CENTER,
                game.title.chars().next().unwrap_or('?').to_string(),
                FontId::proportional(54.0),
                theme::MUTED,
            );
        }
    }

    // Halo de selección y hover
    if selected {
        painter.add(egui::Shape::convex_polygon(
            widgets::rounded_points(cover, 12.0),
            egui::Color32::TRANSPARENT,
            Stroke::new(2.0_f32, theme::ACCENT),
        ));
    }
    if response.hovered() {
        widgets::gradient(
            painter,
            Rect::from_min_max(egui::pos2(cover.min.x, cover.center().y), cover.max),
            egui::Color32::TRANSPARENT,
            egui::Color32::from_black_alpha(210),
            true,
        );
        let circle = egui::pos2(cover.center().x, cover.center().y - 10.0);
        painter.circle_filled(circle, 22.0, theme::ACCENT);
        widgets::icon(
            painter,
            Rect::from_center_size(circle, vec2(16.0, 16.0)),
            widgets::Icon::Play,
            theme::ON_ACCENT,
            1.0,
        );
        painter.text(
            egui::pos2(cover.min.x + 12.0, cover.max.y - 12.0),
            Align2::LEFT_BOTTOM,
            game.playtime_label(),
            FontId::proportional(11.0),
            theme::TEXT,
        );
    }

    // Insignias: plataforma y ProtonDB
    let badge = Rect::from_min_size(
        egui::pos2(cover.min.x + 8.0, cover.min.y + 8.0),
        vec2(62.0, 18.0),
    );
    painter.add(egui::Shape::convex_polygon(
        widgets::rounded_points(badge, 9.0),
        egui::Color32::from_black_alpha(170),
        Stroke::new(1.0_f32, egui::Color32::from_white_alpha(26)),
    ));
    painter.text(
        badge.center(),
        Align2::CENTER_CENTER,
        game.platform.label(app.cfg.lang),
        FontId::proportional(9.5),
        if game.compatible() { theme::TEXT } else { theme::DANGER },
    );

    if game.tier != Tier::Unknown {
        let color = theme::tier_color(game.tier);
        painter.circle_filled(egui::pos2(cover.max.x - 15.0, cover.min.y + 16.0), 4.5, color);
    }
    if game.favorite {
        widgets::icon(
            painter,
            Rect::from_min_size(egui::pos2(cover.max.x - 30.0, cover.max.y - 30.0), vec2(16.0, 16.0)),
            widgets::Icon::Star,
            theme::ACCENT,
            1.3,
        );
    }

    painter.text(
        egui::pos2(rect.min.x + 2.0, cover.max.y + 8.0),
        Align2::LEFT_TOP,
        truncate(&game.title, 22),
        FontId::proportional(13.0),
        if selected { theme::TEXT } else { theme::MUTED },
    );

    response
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        format!("{}…", text.chars().take(max - 1).collect::<String>())
    }
}
