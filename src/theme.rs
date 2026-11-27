use egui::{Color32, Context, FontFamily, FontId, Style, TextStyle, Visuals};

pub const BG: Color32 = Color32::from_rgb(8, 8, 11);
pub const SURFACE: Color32 = Color32::from_rgb(17, 17, 23);
pub const SURFACE_HIGH: Color32 = Color32::from_rgb(26, 26, 34);
pub const TEXT: Color32 = Color32::from_rgb(240, 240, 245);
pub const MUTED: Color32 = Color32::from_rgb(148, 148, 162);
pub const ACCENT: Color32 = Color32::from_rgb(196, 156, 255);
pub const ACCENT_DEEP: Color32 = Color32::from_rgb(122, 82, 214);
pub const DANGER: Color32 = Color32::from_rgb(226, 92, 92);
pub const OK: Color32 = Color32::from_rgb(112, 200, 140);

pub const RADIUS: f32 = 14.0;
pub const RAIL_WIDTH: f32 = 64.0;
pub const TITLEBAR_HEIGHT: f32 = 40.0;
/// Duración del fundido del acento, igual que el cruce de fondos.
pub const ACCENT_FADE: f32 = 0.4;

/// Tinte compartido por todas las superficies translúcidas: oscuro suave y
/// desenfoque de fondo, sin degradados. Al ser uno solo, no se marcan costuras
/// entre paneles que antes usaban alfas distintos.
pub const GLASS: Color32 = Color32::from_black_alpha(104);
/// Opacidad del desenfoque en todos los cristales.
pub const GLASS_OPACITY: f32 = 0.9;
/// Velo uniforme sobre el arte del fondo. Un solo tono, sin degradados.
pub const BACKDROP_DIM: f32 = 0.38;

/// Trazo único y casi invisible para los bordes de cristal.
pub fn hairline() -> egui::Stroke {
    egui::Stroke::new(1.0_f32, Color32::from_white_alpha(10))
}

pub fn apply(ctx: &Context) {
    let mut style = Style::default();
    style.visuals = Visuals::dark();
    style.visuals.panel_fill = Color32::TRANSPARENT;
    style.visuals.window_fill = SURFACE;
    style.visuals.extreme_bg_color = Color32::from_rgb(12, 12, 16);
    style.visuals.faint_bg_color = SURFACE_HIGH;
    style.visuals.widgets.noninteractive.bg_stroke = hairline();
    style.visuals.widgets.inactive.weak_bg_fill = SURFACE_HIGH;
    style.visuals.widgets.inactive.bg_fill = SURFACE_HIGH;
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(38, 38, 48);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(38, 38, 48);
    style.visuals.widgets.active.bg_fill = ACCENT_DEEP;
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.window_corner_radius = egui::CornerRadius::same(RADIUS as u8);
    style.visuals.window_stroke = hairline();
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
        (
            TextStyle::Heading,
            FontId::new(28.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(14.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(11.5, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(12.5, FontFamily::Monospace),
        ),
    ]
    .into();

    ctx.set_style(style);
}

/// Convierte el color del núcleo al tipo de egui.
pub fn rgb(color: crate::accent::Rgb) -> Color32 {
    Color32::from_rgb(color.r, color.g, color.b)
}

/// Mezcla lineal entre dos colores; el alfa se conserva opaco.
pub fn blend(from: Color32, to: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let channel = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
    Color32::from_rgb(
        channel(from.r(), to.r()),
        channel(from.g(), to.g()),
        channel(from.b(), to.b()),
    )
}

/// Brillo percibido de un color, en la misma escala que sus canales (0..255).
pub fn luminance(color: Color32) -> f32 {
    0.2126 * color.r() as f32 + 0.7152 * color.g() as f32 + 0.0722 * color.b() as f32
}

/// Colores de un control pintado sobre el acento.
pub struct Ink {
    pub fill: Color32,
    pub icon: Color32,
    pub text: Color32,
}

/// Icono blanco y texto negro sobre el acento. Si el fondo del control es casi
/// blanco, se invierte a negro para que el icono y el texto sigan leyéndose; en
/// acentos muy oscuros el texto vuelve a blanco por el mismo motivo.
pub fn accent_ink(accent: Color32) -> Ink {
    let lum = luminance(accent);
    if lum >= 228.0 {
        return Ink {
            fill: BG,
            icon: TEXT,
            text: TEXT,
        };
    }
    Ink {
        fill: accent,
        icon: TEXT,
        text: if lum < 120.0 { TEXT } else { Color32::BLACK },
    }
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
