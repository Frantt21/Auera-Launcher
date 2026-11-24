use egui::{vec2, Align2, Context, FontId, Rect, RichText, Sense, Ui};

use crate::app::{App, View};
use crate::i18n::Txt;
use crate::theme;
use crate::ui::titlebar::paint_logo;
use crate::ui::widgets::{self, Icon};

pub fn show(app: &mut App, ctx: &Context) {
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new("rail"))
        .fixed_pos(egui::pos2(0.0, theme::TITLEBAR_HEIGHT))
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui.set_width(theme::RAIL_WIDTH);
            ui.set_height(screen.height() - theme::TITLEBAR_HEIGHT);
            let rect = Rect::from_min_size(
                egui::pos2(0.0, theme::TITLEBAR_HEIGHT),
                vec2(theme::RAIL_WIDTH, screen.height() - theme::TITLEBAR_HEIGHT),
            );
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 0.0, theme::SURFACE);
            painter.line_segment(
                [rect.right_top(), rect.right_bottom()],
                egui::Stroke::new(1.0_f32, egui::Color32::from_white_alpha(10)),
            );

            paint_logo(
                &painter,
                Rect::from_center_size(
                    egui::pos2(rect.center().x, rect.min.y + 40.0),
                    vec2(26.0, 26.0),
                ),
            );

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
                let item = Rect::from_min_size(egui::pos2(9.0, y), vec2(theme::RAIL_WIDTH - 18.0, 62.0));
                let response = ui.interact(item, ui.id().with(("rail", y as i32)), Sense::click());
                if active {
                    painter.rect_filled(item, egui::CornerRadius::same(16), theme::SURFACE_HIGH);
                    painter.rect_filled(
                        Rect::from_min_size(
                            egui::pos2(item.min.x - 9.0, item.center().y - 12.0),
                            vec2(3.0, 24.0),
                        ),
                        egui::CornerRadius::same(2),
                        theme::ACCENT,
                    );
                } else if response.hovered() {
                    painter.rect_filled(item, egui::CornerRadius::same(16), egui::Color32::from_white_alpha(8));
                }

                let color = if active { theme::TEXT } else { theme::MUTED };
                widgets::icon(
                    &painter,
                    Rect::from_center_size(item.center() - vec2(0.0, 7.0), vec2(20.0, 20.0)),
                    icon,
                    color,
                    1.5,
                );
                painter.text(
                    item.center() + vec2(0.0, 18.0),
                    Align2::CENTER_CENTER,
                    app.t(label),
                    FontId::proportional(10.0),
                    color,
                );
                if response.clicked() {
                    app.view = target;
                }
                y += 70.0;
            }
        });
}

#[allow(dead_code)]
fn label(ui: &mut Ui, text: &str) -> egui::Response {
    ui.label(RichText::new(text))
}
