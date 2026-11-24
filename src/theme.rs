use egui::{Color32, Context, FontFamily, FontId, Style, TextStyle, Visuals};

pub const BG: Color32 = Color32::from_rgb(8, 8, 11);
pub const SURFACE: Color32 = Color32::from_rgb(17, 17, 23);
pub const SURFACE_HIGH: Color32 = Color32::from_rgb(26, 26, 34);
pub const TEXT: Color32 = Color32::from_rgb(240, 240, 245);
pub const MUTED: Color32 = Color32::from_rgb(148, 148, 162);
pub const ACCENT: Color32 = Color32::from_rgb(196, 156, 255);
pub const ACCENT_DEEP: Color32 = Color32::from_rgb(122, 82, 214);
pub const ON_ACCENT: Color32 = Color32::from_rgb(20, 12, 32);
pub const DANGER: Color32 = Color32::from_rgb(226, 92, 92);
pub const OK: Color32 = Color32::from_rgb(112, 200, 140);

pub const RADIUS: f32 = 14.0;
pub const RAIL_WIDTH: f32 = 78.0;
pub const TITLEBAR_HEIGHT: f32 = 40.0;

pub fn apply(ctx: &Context) {
    let mut style = Style::default();
    style.visuals = Visuals::dark();
    style.visuals.panel_fill = Color32::TRANSPARENT;
    style.visuals.window_fill = SURFACE;
    style.visuals.extreme_bg_color = Color32::from_rgb(12, 12, 16);
    style.visuals.faint_bg_color = SURFACE_HIGH;
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, Color32::from_white_alpha(12));
    style.visuals.widgets.inactive.weak_bg_fill = SURFACE_HIGH;
    style.visuals.widgets.inactive.bg_fill = SURFACE_HIGH;
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(38, 38, 48);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(38, 38, 48);
    style.visuals.widgets.active.bg_fill = ACCENT_DEEP;
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.window_corner_radius = egui::CornerRadius::same(RADIUS as u8);
    style.visuals.window_stroke = egui::Stroke::new(1.0_f32, Color32::from_white_alpha(18));
    style.visuals.selection.bg_fill = ACCENT_DEEP;
    style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);

    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.scroll.bar_width = 6.0;
    style.spacing.scroll.floating = true;
    style.text_styles = [
        (TextStyle::Heading, FontId::new(28.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.5, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.5, FontFamily::Monospace)),
    ]
    .into();

    ctx.set_style(style);
}

pub fn tier_color(tier: crate::model::Tier) -> Color32 {
    use crate::model::Tier::*;
    match tier {
        Platinum => Color32::from_rgb(190, 235, 255),
        Gold => Color32::from_rgb(235, 200, 90),
        Silver => Color32::from_rgb(200, 205, 215),
        Bronze => Color32::from_rgb(205, 145, 100),
        Borked => DANGER,
        Unknown => MUTED,
    }
}
