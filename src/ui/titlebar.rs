use egui::{pos2, vec2, Context, Rect, Sense, Stroke};

use crate::app::App;
use crate::theme;
use crate::ui::widgets::{self, Icon};

const HEIGHT: f32 = 40.0;
const CLUSTER: egui::Vec2 = egui::Vec2::new(80.0, 28.0);

/// Franja superior arrastrable con el contenedor `[_|x]` en la esquina derecha.
pub fn show(_app: &mut App, ctx: &Context) {
    let screen = ctx.content_rect();

    egui::Area::new(egui::Id::new("titlebar"))
        .fixed_pos(screen.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let rect = Rect::from_min_size(screen.min, vec2(screen.width(), HEIGHT));
            ui.set_width(rect.width());
            ui.set_height(rect.height());

            let drag = ui.interact(
                Rect::from_min_max(rect.min, pos2(rect.max.x - CLUSTER.x - 18.0, rect.max.y)),
                ui.id().with("drag"),
                Sense::click_and_drag(),
            );
            if drag.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag.double_clicked() {
                let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }

            let cluster = Rect::from_center_size(
                pos2(rect.max.x - CLUSTER.x / 2.0 - 14.0, rect.center().y),
                CLUSTER,
            );
            let tone = widgets::glass_tone(ctx, cluster);
            widgets::glass_panel_toned(ui, cluster, 9.0, tone);

            let buttons = [
                (
                    Icon::Minimize,
                    "min",
                    false,
                    Rect::from_min_max(cluster.min, pos2(cluster.center().x, cluster.max.y)),
                ),
                (
                    Icon::Close,
                    "close",
                    true,
                    Rect::from_min_max(pos2(cluster.center().x, cluster.min.y), cluster.max),
                ),
            ];
            for (icon, id, danger, area) in buttons {
                let response = ui.interact(area, ui.id().with(id), Sense::click());
                let active = response.hovered() || response.is_pointer_button_down_on();
                if active {
                    let color = if danger {
                        theme::DANGER.gamma_multiply(0.85)
                    } else {
                        egui::Color32::from_white_alpha(18)
                    };
                    // Mismo radio que el contenedor para que el relleno del
                    // hover no asome por las esquinas redondeadas.
                    let radii = if danger {
                        [0.0, 9.0, 9.0, 0.0]
                    } else {
                        [9.0, 0.0, 0.0, 9.0]
                    };
                    ui.painter().add(egui::Shape::convex_polygon(
                        widgets::rounded_points_with(area, radii),
                        color,
                        Stroke::NONE,
                    ));
                }
                widgets::icon(
                    ui.painter(),
                    Rect::from_center_size(area.center(), vec2(11.0, 11.0)),
                    icon,
                    theme::TEXT,
                );
                if response.clicked() {
                    match id {
                        "min" => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                        _ => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                    }
                }
            }

            // El borde y el separador se pintan al final para que el hover del
            // cierre o de minimizar no los tape.
            widgets::hairline_round(ui.painter(), cluster, 9.0);
            ui.painter().line_segment(
                [
                    pos2(cluster.center().x, cluster.min.y + 6.0),
                    pos2(cluster.center().x, cluster.max.y - 6.0),
                ],
                Stroke::new(1.0_f32, egui::Color32::from_white_alpha(20)),
            );
        });
}
