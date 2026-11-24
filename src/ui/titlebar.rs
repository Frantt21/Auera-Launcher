use egui::{pos2, vec2, Align2, Context, FontId, Rect, RichText, Sense, Stroke};

use crate::app::App;
use crate::theme;
use crate::ui::widgets::{self, Icon};

pub fn show(app: &mut App, ctx: &Context) {
    let screen = ctx.content_rect();
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));

    egui::Area::new(egui::Id::new("titlebar"))
        .fixed_pos(screen.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.set_width(screen.width());
            ui.set_height(theme::TITLEBAR_HEIGHT);
            let rect = Rect::from_min_size(screen.min, vec2(screen.width(), theme::TITLEBAR_HEIGHT));
            let painter = ui.painter_at(rect);

            // Barra y línea inferior
            painter.rect_filled(rect, 0.0, theme::SURFACE);
            painter.line_segment(
                [rect.left_bottom(), rect.right_bottom()],
                Stroke::new(1.0_f32, egui::Color32::from_white_alpha(10)),
            );

            paint_logo(&painter, Rect::from_min_size(rect.min + vec2(18.0, 12.0), vec2(16.0, 16.0)));
            painter.text(
                pos2(rect.min.x + 44.0, rect.center().y),
                Align2::LEFT_CENTER,
                "AUERA",
                FontId::proportional(13.0),
                theme::TEXT,
            );

            if app.scanning {
                ui.put(
                    Rect::from_center_size(
                        pos2(rect.center().x, rect.center().y),
                        vec2(160.0, 24.0),
                    ),
                    egui::Label::new(
                        RichText::new(app.t(crate::i18n::Txt::Scanning)).size(11.5).color(theme::MUTED),
                    ),
                );
            }

            let drag = ui.interact(
                Rect::from_min_max(rect.min, pos2(rect.max.x - 138.0, rect.max.y)),
                ui.id().with("drag"),
                Sense::click_and_drag(),
            );
            if drag.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag.double_clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }

            let buttons = [
                (Icon::Minimize, "min", false),
                (Icon::Maximize, "max", false),
                (Icon::Close, "close", true),
            ];
            for (index, (icon, id, danger)) in buttons.into_iter().enumerate() {
                let button_rect = Rect::from_min_size(
                    pos2(rect.max.x - 138.0 + index as f32 * 46.0, rect.min.y),
                    vec2(46.0, theme::TITLEBAR_HEIGHT),
                );
                let response = ui.interact(button_rect, ui.id().with(id), Sense::click());
                if response.hovered() {
                    let color = if danger { theme::DANGER } else { theme::SURFACE_HIGH };
                    ui.painter().rect_filled(button_rect, 0.0, color);
                }
                widgets::icon(
                    ui.painter(),
                    Rect::from_center_size(button_rect.center(), vec2(12.0, 12.0)),
                    icon,
                    if danger && response.hovered() { theme::TEXT } else { theme::MUTED },
                    1.4,
                );
                if response.clicked() {
                    match id {
                        "min" => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                        "max" => ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized)),
                        _ => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                    }
                }
            }
        });
}

pub fn paint_logo(painter: &egui::Painter, rect: Rect) {
    let centers = [
        (pos2(rect.center().x, rect.min.y + rect.height() * 0.22), rect.width() * 0.22),
        (pos2(rect.min.x + rect.width() * 0.24, rect.max.y - rect.height() * 0.24), rect.width() * 0.22),
        (pos2(rect.max.x - rect.width() * 0.24, rect.max.y - rect.height() * 0.24), rect.width() * 0.22),
    ];
    for (center, radius) in centers {
        painter.circle_stroke(center, radius, Stroke::new(1.6_f32, theme::TEXT));
    }
}
