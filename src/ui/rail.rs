use egui::{pos2, vec2, Context, Rect, Sense, Stroke};

use crate::app::{App, View};
use crate::i18n::Txt;
use crate::theme;
use crate::ui::widgets::{self, Icon};

const PAD: f32 = 14.0;

/// Rail flotante y translúcido, con el icono de la app y navegación sin etiquetas.
pub fn show(app: &mut App, ctx: &Context) {
    let screen = ctx.content_rect();
    let top = theme::TITLEBAR_HEIGHT + 10.0;
    let rect = Rect::from_min_size(
        pos2(PAD, top),
        vec2(theme::RAIL_WIDTH, screen.height() - top - PAD),
    );

    egui::Area::new(egui::Id::new("rail"))
        .fixed_pos(rect.min)
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui.set_width(rect.width());
            ui.set_height(rect.height());
            let painter = ui.painter_at(rect);

            painter.add(egui::Shape::convex_polygon(
                widgets::rounded_points(rect, 22.0),
                egui::Color32::from_black_alpha(122),
                Stroke::new(1.0_f32, egui::Color32::from_white_alpha(16)),
            ));

            if let Some(logo) = &app.logo {
                let logo_rect = Rect::from_center_size(pos2(rect.center().x, rect.min.y + 36.0), vec2(34.0, 34.0));
                widgets::texture_rounded(&painter, logo_rect, 11.0, logo.id(), logo.size_vec2());
            }

            let items = [
                (Icon::Grid, Txt::Library, View::Library),
                (Icon::Gear, Txt::Settings, View::Settings),
            ];
            let mut y = rect.min.y + 92.0;
            for (icon, label, target) in items {
                let active = match (&app.view, &target) {
                    (View::Detail(_), View::Library) => true,
                    (current, target) => current == target,
                };
                let item = Rect::from_center_size(pos2(rect.center().x, y), vec2(44.0, 44.0));
                let response = ui.interact(item, ui.id().with(("rail", y as i32)), Sense::click());

                if active {
                    painter.add(egui::Shape::convex_polygon(
                        widgets::rounded_points(item, 14.0),
                        theme::ACCENT,
                        Stroke::NONE,
                    ));
                } else if response.hovered() {
                    painter.add(egui::Shape::convex_polygon(
                        widgets::rounded_points(item, 14.0),
                        egui::Color32::from_white_alpha(22),
                        Stroke::NONE,
                    ));
                }

                widgets::icon(
                    ui.painter(),
                    Rect::from_center_size(item.center(), vec2(20.0, 20.0)),
                    icon,
                    if active { theme::ON_ACCENT } else { theme::MUTED },
                    1.6,
                );

                let response = response.on_hover_text(app.t(label));
                if response.clicked() {
                    app.view = target;
                    app.game_dialog = false;
                }
                y += 58.0;
            }
        });
}
