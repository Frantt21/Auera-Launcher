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
    let accent = app.accent();
    let mut hovered: Option<String> = None;
    // Sin puntero en la ventana no hay hover: evita que el último se quede
    // pintado cuando el cursor sale por un borde.
    let pointer_here = ui.ctx().input(|i| i.pointer.has_pointer());
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
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            // Filtro por launcher
            let all = app.source_filter.is_none();
            if widgets::chip(ui, tr(lang, Txt::AllSources), all, accent).clicked() {
                app.source_filter = None;
            }
            for source in Source::ALL {
                if app.library.count_by_source(source) == 0 {
                    continue;
                }
                let active = app.source_filter == Some(source);
                if widgets::chip(ui, source.label(), active, accent).clicked() {
                    app.source_filter = Some(source);
                }
            }

            widgets::divider(ui);
            if widgets::chip(
                ui,
                tr(lang, Txt::AllPlatforms),
                app.platform_filter.is_none(),
                accent,
            )
            .clicked()
            {
                app.platform_filter = None;
            }
            for platform in [Platform::Windows, Platform::Linux] {
                let active = app.platform_filter == Some(platform);
                if widgets::chip(ui, platform.label(lang), active, accent).clicked() {
                    app.platform_filter = Some(platform);
                }
            }

            widgets::divider(ui);
            if widgets::chip(ui, tr(lang, Txt::FavoritesOnly), app.favorites_only, accent).clicked()
            {
                app.favorites_only = !app.favorites_only;
            }

            widgets::divider(ui);
            sort_menu(app, ui, lang);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let hint = tr(lang, Txt::Search);
                widgets::search(ui, &mut app.search, hint, 230.0);
            });
        });
    });

    let grid = Rect::from_min_max(egui::pos2(area.min.x, header.max.y + 6.0), area.max);
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
                            if pointer_here && response.hovered() {
                                hovered = Some(game.id.clone());
                            }
                            if index == app.cursor && app.nav_mode {
                                let painter = ui.painter();
                                let rect = response.rect;
                                painter.add(egui::Shape::convex_polygon(
                                    widgets::rounded_points(rect, 12.0),
                                    egui::Color32::TRANSPARENT,
                                    Stroke::new(1.5_f32, accent.gamma_multiply(0.5)),
                                ));
                            }
                        }
                    });
                    ui.add_space(GAP);
                }
            });
    });

    // El fondo animado se pinta fuera de este panel en el fotograma siguiente.
    if app.hover_bg != hovered {
        app.hover_bg = hovered;
        ui.ctx().request_repaint();
    }
}

fn sort_menu(app: &mut App, ui: &mut Ui, lang: crate::config::Lang) {
    let label = match app.sort {
        Sort::Name => tr(lang, Txt::Name),
        Sort::Recent => tr(lang, Txt::Recent),
        Sort::Playtime => tr(lang, Txt::Playtime),
        Sort::Size => tr(lang, Txt::Size),
    };
    let probe = Rect::from_min_size(ui.cursor().min, vec2(120.0, 30.0));
    let tone = widgets::glass_tone(ui.ctx(), probe);
    let accent = app.accent();
    let chevron = widgets::Icon::ChevronDown.glyph();
    let button = widgets::chip_button(&format!("{label}  {chevron}"), false, accent, tone);
    let _ = egui::containers::menu::MenuButton::from_button(button).ui(ui, |ui| {
        ui.set_min_width(170.0);
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.add_space(2.0);
        let ink = theme::accent_ink(accent);
        for (sort, text) in [
            (Sort::Recent, Txt::Recent),
            (Sort::Name, Txt::Name),
            (Sort::Playtime, Txt::Playtime),
            (Sort::Size, Txt::Size),
        ] {
            let selected = app.sort == sort;
            let (rect, response) = ui.allocate_exact_size(vec2(170.0, 30.0), Sense::click());
            let fill = if selected {
                ink.fill
            } else if response.hovered() {
                egui::Color32::from_white_alpha(16)
            } else {
                egui::Color32::TRANSPARENT
            };
            widgets::fill_round(ui.painter(), rect, rect.height() / 2.0, fill);
            ui.painter().text(
                egui::pos2(rect.min.x + 12.0, rect.center().y),
                Align2::LEFT_CENTER,
                tr(lang, text),
                FontId::proportional(12.5),
                if selected { ink.text } else { theme::TEXT },
            );
            if response.clicked() {
                app.sort = sort;
                ui.close();
            }
        }
        ui.add_space(2.0);
    });
}

fn card(app: &mut App, ui: &mut Ui, game: &Game, selected: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(CARD_WIDTH, COVER_HEIGHT + 34.0), Sense::click());
    let cover = Rect::from_min_size(rect.min, vec2(CARD_WIDTH, COVER_HEIGHT));
    let painter = ui.painter();

    // Sombra del acento por detrás de la carátula: sustituye al botón de play
    // del hover y solo se ve el halo que asoma fuera de la portada.
    if response.hovered() {
        let accent = app.cover_accent(game);
        let shadow = egui::epaint::Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 1,
            color: egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 120),
        };
        painter.add(shadow.as_shape(cover, 12.0));
    }

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

    // Halo de selección y hover. El velo va debajo del halo para no taparlo.
    if response.hovered() {
        // Velo uniforme, sin degradado: el mismo tono oscuro del cristal. Sobre
        // la carátula no se puede desenfocar el fondo, así que solo se oscurece.
        widgets::fill_round(painter, cover, 12.0, theme::GLASS);
    }
    if selected {
        painter.add(egui::Shape::convex_polygon(
            widgets::rounded_points(cover, 12.0),
            egui::Color32::TRANSPARENT,
            Stroke::new(2.0_f32, app.accent()),
        ));
    }
    if response.hovered() {
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
        if game.compatible() {
            theme::TEXT
        } else {
            theme::DANGER
        },
    );

    if game.tier != Tier::Unknown {
        let color = theme::tier_color(game.tier);
        painter.circle_filled(
            egui::pos2(cover.max.x - 15.0, cover.min.y + 16.0),
            4.5,
            color,
        );
    }
    if game.favorite {
        widgets::icon(
            painter,
            Rect::from_min_size(
                egui::pos2(cover.max.x - 30.0, cover.max.y - 30.0),
                vec2(16.0, 16.0),
            ),
            widgets::Icon::Star,
            theme::ACCENT,
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
