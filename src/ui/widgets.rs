use std::sync::Arc;

use egui::{
    epaint::Vertex, Color32, Mesh, Pos2, Rect, Response, RichText, Sense, Stroke, TextureId, Ui,
    Vec2,
};

use crate::theme;
use crate::ui::textures::Luma;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Grid,
    Gear,
    Star,
    Close,
    Minimize,
    ChevronLeft,
    ChevronDown,
    Trash,
}

impl Icon {
    /// Glifo equivalente en Material Symbols Rounded.
    pub fn glyph(self) -> &'static str {
        use egui_material_icons::icons::*;
        match self {
            Self::Grid => ICON_GRID_VIEW,
            Self::Gear => ICON_SETTINGS,
            Self::Star => ICON_STAR,
            Self::Close => ICON_CLOSE,
            Self::Minimize => ICON_REMOVE,
            Self::ChevronLeft => ICON_CHEVRON_LEFT,
            Self::ChevronDown => ICON_EXPAND_MORE,
            Self::Trash => ICON_DELETE,
        }
    }
}

/// Copia desenfocada del fondo actual, publicada por `App` cada fotograma.
/// Permite pintar un efecto de cristal esmerilado bajo los paneles flotantes.
#[derive(Clone)]
pub struct Glass {
    pub texture: TextureId,
    pub size: Vec2,
    /// Textura anterior, para fundirla con la actual al cambiar de fondo.
    pub prev: Option<TextureId>,
    pub prev_size: Vec2,
    /// Brillo de la textura actual y de la anterior, para elegir el contraste.
    pub luma: Option<Arc<Luma>>,
    pub prev_luma: Option<Arc<Luma>>,
    /// Peso de la textura actual (0 = solo `prev`, 1 = solo `texture`).
    pub fade: f32,
    pub screen: Rect,
    pub alpha: u8,
}

fn glass_data(ctx: &egui::Context) -> Option<Glass> {
    ctx.data(|d| d.get_temp::<Glass>(egui::Id::new("glass")))
}

/// Contraste que necesita un botón de cristal según el fondo que tiene detrás.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlassTone {
    Dark,
    Light,
}

/// Mira el brillo del fondo desenfocado bajo `rect`: si es claro, el cristal se
/// sustituye por un botón negro para que el contenido blanco no se pierda.
pub fn glass_tone(ctx: &egui::Context, rect: Rect) -> GlassTone {
    let Some(glass) = glass_data(ctx) else {
        return GlassTone::Dark;
    };
    if glass.alpha < 24 || glass.screen.width() <= 0.0 || glass.screen.height() <= 0.0 {
        return GlassTone::Dark;
    }
    let luma = if glass.fade >= 0.5 {
        glass.luma.as_ref()
    } else {
        glass.prev_luma.as_ref().or(glass.luma.as_ref())
    };
    let Some(grid) = luma else {
        return GlassTone::Dark;
    };
    let uv = cover_uv(glass.size, glass.screen.size());
    let points = [
        rect.center(),
        Pos2::new(rect.left() + rect.width() * 0.2, rect.center().y),
        Pos2::new(rect.left() + rect.width() * 0.8, rect.center().y),
        Pos2::new(rect.center().x, rect.top() + rect.height() * 0.25),
        Pos2::new(rect.center().x, rect.top() + rect.height() * 0.75),
    ];
    let mut total = 0.0;
    for point in &points {
        let relative = Vec2::new(
            (point.x - glass.screen.min.x) / glass.screen.width(),
            (point.y - glass.screen.min.y) / glass.screen.height(),
        );
        total += grid.sample(
            uv.min.x + relative.x * uv.width(),
            uv.min.y + relative.y * uv.height(),
        ) as f32;
    }
    if total / points.len() as f32 >= 150.0 {
        GlassTone::Light
    } else {
        GlassTone::Dark
    }
}

/// Pinta el fondo desenfocado dentro de `rect`, con un tinte encima.
/// `opacity` deja ver el fondo real bajo el cristal (translúcido).
/// Devuelve `false` si todavía no hay textura de cristal disponible.
pub fn glass_fill(ui: &Ui, rect: Rect, radius: f32, tint: Color32, opacity: f32) -> bool {
    let Some(shapes) = glass_shape(ui.ctx(), rect, radius, tint, opacity) else {
        return false;
    };
    for shape in shapes {
        ui.painter().add(shape);
    }
    true
}

/// Cristal estándar: mismo tinte y mismo desenfoque en toda la interfaz.
pub fn glass(ui: &Ui, rect: Rect, radius: f32) -> bool {
    glass_fill(ui, rect, radius, theme::GLASS, theme::GLASS_OPACITY)
}

/// Panel flotante de cristal: oscuro, desenfocado y con un borde fino común.
pub fn glass_panel(ui: &Ui, rect: Rect, radius: f32) {
    if !glass(ui, rect, radius) {
        fill_round(ui.painter(), rect, radius, theme::SURFACE);
    }
    hairline_round(ui.painter(), rect, radius);
}

/// Igual que `glass_panel`, pero con el tono ya calculado: sobre un fondo claro
/// el panel pasa a negro para mantener el contraste.
pub fn glass_panel_toned(ui: &Ui, rect: Rect, radius: f32, tone: GlassTone) {
    if tone == GlassTone::Light {
        fill_round(ui.painter(), rect, radius, theme::BG);
    } else if !glass(ui, rect, radius) {
        fill_round(ui.painter(), rect, radius, theme::SURFACE);
    }
    hairline_round(ui.painter(), rect, radius);
}

/// Igual que `glass_panel`, sobre un pintor ya abierto (avisos, capas altas).
pub fn glass_paint(painter: &egui::Painter, ctx: &egui::Context, rect: Rect, radius: f32) {
    match glass_shape(ctx, rect, radius, theme::GLASS, theme::GLASS_OPACITY) {
        Some(shapes) => {
            for shape in shapes {
                painter.add(shape);
            }
        }
        None => fill_round(painter, rect, radius, theme::SURFACE),
    }
    hairline_round(painter, rect, radius);
}

/// Borde casi invisible, por encima del relleno: así el hover no lo tapa.
pub fn hairline_round(painter: &egui::Painter, rect: Rect, radius: f32) {
    painter.add(egui::Shape::convex_polygon(
        rounded_points(rect.shrink(0.5), radius),
        Color32::TRANSPARENT,
        theme::hairline(),
    ));
}

/// Relleno opaco con esquinas redondeadas, sin contorno.
pub fn fill_round(painter: &egui::Painter, rect: Rect, radius: f32, color: Color32) {
    painter.add(egui::Shape::convex_polygon(
        rounded_points(rect, radius),
        color,
        Stroke::NONE,
    ));
}

fn glass_shape(
    ctx: &egui::Context,
    rect: Rect,
    radius: f32,
    tint: Color32,
    opacity: f32,
) -> Option<Vec<egui::Shape>> {
    let glass = glass_data(ctx)?;
    if glass.screen.width() <= 0.0 || glass.screen.height() <= 0.0 {
        return None;
    }
    let alpha = (glass.alpha as f32 * opacity.clamp(0.0, 1.0)).round() as u8;
    if alpha < 24 {
        return None;
    }
    let darken = 1.0 - tint.a() as f32 / 255.0;

    let mut shapes = Vec::new();
    if let Some(prev) = glass.prev {
        let weight = (1.0 - glass.fade).clamp(0.0, 1.0);
        let layer_alpha = (alpha as f32 * weight).round() as u8;
        if layer_alpha >= 8 {
            shapes.push(egui::Shape::mesh(glass_mesh(
                &glass,
                rect,
                radius,
                prev,
                glass.prev_size,
                layer_alpha,
                darken,
            )));
        }
    }
    let weight = glass.fade.clamp(0.0, 1.0);
    let layer_alpha = (alpha as f32 * weight).round() as u8;
    if layer_alpha >= 8 {
        shapes.push(egui::Shape::mesh(glass_mesh(
            &glass,
            rect,
            radius,
            glass.texture,
            glass.size,
            layer_alpha,
            darken,
        )));
    }

    Some(shapes).filter(|shapes| !shapes.is_empty())
}

#[allow(clippy::too_many_arguments)]
fn glass_mesh(
    glass: &Glass,
    rect: Rect,
    radius: f32,
    texture: TextureId,
    source: Vec2,
    alpha: u8,
    darken: f32,
) -> Mesh {
    let uv = cover_uv(source, glass.screen.size());
    let sample = |point: Pos2| {
        let rel = Vec2::new(
            (point.x - glass.screen.min.x) / glass.screen.width(),
            (point.y - glass.screen.min.y) / glass.screen.height(),
        );
        Pos2::new(
            uv.min.x + rel.x * uv.width(),
            uv.min.y + rel.y * uv.height(),
        )
    };

    // El tinte se aplica en el propio vértice: así todo el cristal es una sola
    // malla y el borde se funde con el fondo sin una costura extra.
    let value = (darken * alpha as f32).round().clamp(0.0, 255.0) as u8;
    let inner_color = Color32::from_rgba_premultiplied(value, value, value, alpha);
    let outer_color = Color32::from_rgba_premultiplied(0, 0, 0, 0);

    // Borde de 1 px que se desvanece: el equivalente a feathering de epaint
    // para mallas con textura, que de otro modo salen con dientes de sierra.
    let inner = rounded_points(rect, radius);
    let outer = rounded_points(rect.expand(1.0), radius + 1.0);

    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex {
        pos: rect.center(),
        uv: sample(rect.center()),
        color: inner_color,
    });
    let inner_start = mesh.vertices.len() as u32;
    for point in &inner {
        mesh.vertices.push(Vertex {
            pos: *point,
            uv: sample(*point),
            color: inner_color,
        });
    }
    let outer_start = mesh.vertices.len() as u32;
    for point in &outer {
        mesh.vertices.push(Vertex {
            pos: *point,
            uv: sample(*point),
            color: outer_color,
        });
    }

    let count = inner.len();
    for index in 0..count {
        let next = (index + 1) % count;
        mesh.add_triangle(0, inner_start + index as u32, inner_start + next as u32);
        mesh.add_triangle(
            inner_start + index as u32,
            inner_start + next as u32,
            outer_start + next as u32,
        );
        mesh.add_triangle(
            inner_start + index as u32,
            outer_start + next as u32,
            outer_start + index as u32,
        );
    }
    mesh.texture_id = texture;
    mesh
}

/// Botón de navegación con el mismo cristal y borde que el rail lateral.
pub fn nav_button(ui: &mut Ui, icon_kind: Icon, size: Vec2) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let radius = 14.0;
    let toned = glass_tone(ui.ctx(), rect);
    if toned == GlassTone::Light {
        fill_round(ui.painter(), rect, radius, theme::BG);
    } else if !glass(ui, rect, radius) {
        fill_round(ui.painter(), rect, radius, theme::SURFACE);
    }
    if response.hovered() {
        ui.painter().add(egui::Shape::convex_polygon(
            rounded_points(rect, radius),
            Color32::from_white_alpha(22),
            Stroke::NONE,
        ));
    }
    hairline_round(ui.painter(), rect, radius);
    icon(
        ui.painter(),
        Rect::from_center_size(rect.center(), size * 0.44),
        icon_kind,
        theme::TEXT,
    );
    response
}

/// Pinta un icono centrado en `rect`, escalado al alto de la caja.
pub fn icon(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon.glyph(),
        egui::FontId::proportional(rect.width().min(rect.height())),
        color,
    );
}

pub fn rounded_points(rect: Rect, radius: f32) -> Vec<Pos2> {
    rounded_points_with(rect, [radius; 4])
}

/// Igual que `rounded_points`, con un radio por esquina
/// (arriba-izquierda, arriba-derecha, abajo-derecha, abajo-izquierda).
pub fn rounded_points_with(rect: Rect, radii: [f32; 4]) -> Vec<Pos2> {
    let limit = (rect.width().min(rect.height()) / 2.0).max(0.0);
    let [tl, tr, br, bl] = radii.map(|radius| radius.clamp(0.0, limit));
    if tl <= 0.5 && tr <= 0.5 && br <= 0.5 && bl <= 0.5 {
        return vec![
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ];
    }
    let corners = [
        (
            Pos2::new(rect.min.x + tl, rect.min.y + tl),
            std::f32::consts::PI,
            tl,
        ),
        (
            Pos2::new(rect.max.x - tr, rect.min.y + tr),
            1.5 * std::f32::consts::PI,
            tr,
        ),
        (Pos2::new(rect.max.x - br, rect.max.y - br), 0.0, br),
        (
            Pos2::new(rect.min.x + bl, rect.max.y - bl),
            0.5 * std::f32::consts::PI,
            bl,
        ),
    ];
    let flat = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ];
    let mut points = Vec::with_capacity(4 * 9);
    for (index, (center, start, radius)) in corners.into_iter().enumerate() {
        if radius <= 0.5 {
            points.push(flat[index]);
            continue;
        }
        for step in 0..=8 {
            let angle = start + step as f32 * (std::f32::consts::FRAC_PI_2 / 8.0);
            points.push(center + Vec2::angled(angle) * radius);
        }
    }
    points
}

/// Recorte tipo "cover" para rellenar el rectángulo sin deformar la imagen.
pub fn cover_uv(texture: Vec2, target: Vec2) -> Rect {
    if texture.x <= 0.0 || texture.y <= 0.0 || target.x <= 0.0 || target.y <= 0.0 {
        return Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    }
    let source_aspect = texture.x / texture.y;
    let target_aspect = target.x / target.y;
    let center = Pos2::new(0.5, 0.5);
    if source_aspect > target_aspect {
        let width = target_aspect / source_aspect;
        Rect::from_center_size(center, Vec2::new(width, 1.0))
    } else {
        let height = source_aspect / target_aspect;
        Rect::from_center_size(center, Vec2::new(1.0, height))
    }
}

/// Dibuja una textura recortada dentro de un rectángulo con esquinas redondeadas.
pub fn texture_rounded(
    painter: &egui::Painter,
    rect: Rect,
    radius: f32,
    texture: TextureId,
    texture_size: Vec2,
) {
    texture_rounded_tinted(painter, rect, radius, texture, texture_size, Color32::WHITE);
}

/// Igual que `texture_rounded`, con un tinte (útil para fundidos).
pub fn texture_rounded_tinted(
    painter: &egui::Painter,
    rect: Rect,
    radius: f32,
    texture: TextureId,
    texture_size: Vec2,
    tint: Color32,
) {
    if rect.width() < 2.0 || rect.height() < 2.0 {
        return;
    }
    let uv = cover_uv(texture_size, rect.size());
    let sample = |point: Pos2| {
        let rel = Vec2::new(
            (point.x - rect.min.x) / rect.width(),
            (point.y - rect.min.y) / rect.height(),
        );
        Pos2::new(
            uv.min.x + rel.x * uv.width(),
            uv.min.y + rel.y * uv.height(),
        )
    };

    // Anillo exterior transparente de 1 px: suaviza las esquinas de la imagen
    // (y el borde de la ventana) igual que hace epaint con las formas sólidas.
    let inner = rounded_points(rect, radius);
    let outer = rounded_points(rect.expand(1.0), radius + 1.0);
    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex {
        pos: rect.center(),
        uv: sample(rect.center()),
        color: tint,
    });
    let inner_start = mesh.vertices.len() as u32;
    for point in &inner {
        mesh.vertices.push(Vertex {
            pos: *point,
            uv: sample(*point),
            color: tint,
        });
    }
    let outer_start = mesh.vertices.len() as u32;
    let clear = Color32::from_rgba_premultiplied(0, 0, 0, 0);
    for point in &outer {
        mesh.vertices.push(Vertex {
            pos: *point,
            uv: sample(*point),
            color: clear,
        });
    }
    let count = inner.len();
    for index in 0..count {
        let next = (index + 1) % count;
        mesh.add_triangle(0, inner_start + index as u32, inner_start + next as u32);
        mesh.add_triangle(
            inner_start + index as u32,
            inner_start + next as u32,
            outer_start + next as u32,
        );
        mesh.add_triangle(
            inner_start + index as u32,
            outer_start + next as u32,
            outer_start + index as u32,
        );
    }
    mesh.texture_id = texture;
    painter.add(egui::Shape::mesh(mesh));
}

pub fn icon_button(
    ui: &mut Ui,
    icon_kind: Icon,
    size: Vec2,
    active: bool,
    accent: Color32,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let ink = theme::accent_ink(accent);
    let tone = glass_tone(ui.ctx(), rect);
    let fill = if active {
        ink.fill
    } else if tone == GlassTone::Light {
        theme::BG
    } else if response.hovered() {
        Color32::from_white_alpha(34)
    } else {
        Color32::from_white_alpha(16)
    };
    ui.painter().add(egui::Shape::convex_polygon(
        rounded_points(rect, size.y.min(size.x) / 2.0),
        fill,
        Stroke::new(1.0_f32, Color32::from_white_alpha(30)),
    ));
    icon(
        ui.painter(),
        Rect::from_center_size(rect.center(), size * 0.44),
        icon_kind,
        if active { ink.icon } else { theme::TEXT },
    );
    response
}

pub fn search(ui: &mut Ui, text: &mut String, hint: &str, width: f32) -> Response {
    // El hueco se reserva antes del texto para que el cristal quede debajo.
    let painter = ui.painter().clone();
    let slots = [
        painter.add(egui::Shape::Noop),
        painter.add(egui::Shape::Noop),
    ];
    let has_glass = ui.layer_id() == egui::LayerId::background();
    let frame = egui::Frame::new()
        .fill(Color32::TRANSPARENT)
        .corner_radius(egui::CornerRadius::same(100))
        .stroke(theme::hairline())
        .inner_margin(egui::Margin::symmetric(12, 6));
    let shown = frame.show(ui, |ui| {
        ui.add(
            egui::TextEdit::singleline(text)
                .hint_text(hint)
                .font(egui::FontId::proportional(12.5))
                .text_color(theme::TEXT)
                .frame(false)
                .desired_width(width)
                .margin(egui::Margin::ZERO),
        )
    });
    let response = shown.inner;
    let rect = shown.response.rect;
    let radius = rect.height() / 2.0;
    if glass_tone(ui.ctx(), rect) == GlassTone::Light {
        painter.set(
            slots[0],
            egui::Shape::convex_polygon(rounded_points(rect, radius), theme::BG, Stroke::NONE),
        );
    } else {
        match glass_shape(ui.ctx(), rect, radius, theme::GLASS, theme::GLASS_OPACITY) {
            Some(shapes) if has_glass => {
                for (slot, shape) in slots.into_iter().zip(shapes) {
                    painter.set(slot, shape);
                }
            }
            _ => {
                painter.set(
                    slots[0],
                    egui::Shape::convex_polygon(
                        rounded_points(rect, radius),
                        Color32::from_white_alpha(12),
                        Stroke::NONE,
                    ),
                );
            }
        }
    }
    response
}

pub fn pill(ui: &mut Ui, text: &str, filled: bool, width: f32, accent: Color32) -> Response {
    if filled {
        let ink = theme::accent_ink(accent);
        let button = egui::Button::new(RichText::new(text).color(ink.text).strong())
            .fill(ink.fill)
            .stroke(Stroke::NONE)
            .corner_radius(egui::CornerRadius::same(100));
        return ui.add_sized([width, 38.0], button);
    }

    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 38.0), Sense::hover());
    let tone = glass_tone(ui.ctx(), rect);
    let glassed = tone == GlassTone::Dark
        && ui.layer_id() == egui::LayerId::background()
        && glass_fill(ui, rect, 19.0, theme::GLASS, theme::GLASS_OPACITY);
    let button = egui::Button::new(RichText::new(text).color(theme::TEXT).strong())
        .fill(if tone == GlassTone::Light {
            theme::BG
        } else if glassed {
            Color32::TRANSPARENT
        } else {
            Color32::from_white_alpha(16)
        })
        .stroke(Stroke::new(1.0_f32, Color32::from_white_alpha(28)))
        .corner_radius(egui::CornerRadius::same(100));
    ui.put(rect, button)
}

/// Base común de los filtros: así el menú de orden mide igual que los chips.
pub fn chip_button(
    label: &str,
    active: bool,
    accent: Color32,
    tone: GlassTone,
) -> egui::Button<'static> {
    let ink = theme::accent_ink(accent);
    egui::Button::new(
        RichText::new(label.to_string())
            .size(12.5)
            .color(if active { ink.text } else { theme::TEXT }),
    )
    .fill(if active {
        ink.fill
    } else if tone == GlassTone::Light {
        theme::BG
    } else {
        Color32::from_white_alpha(12)
    })
    .stroke(Stroke::NONE)
    .corner_radius(egui::CornerRadius::same(100))
}

pub fn chip(ui: &mut Ui, label: &str, active: bool, accent: Color32) -> Response {
    let probe = Rect::from_min_size(ui.cursor().min, Vec2::new(120.0, 30.0));
    let tone = glass_tone(ui.ctx(), probe);
    if active {
        return ui.add(chip_button(label, true, accent, tone));
    }

    let painter = ui.painter().clone();
    let mesh_slot = painter.add(egui::Shape::Noop);
    let tint_slot = painter.add(egui::Shape::Noop);
    let has_glass = tone == GlassTone::Dark
        && ui.layer_id() == egui::LayerId::background()
        && glass_data(ui.ctx()).is_some_and(|glass| glass.alpha >= 24);
    let button = egui::Button::new(
        RichText::new(label.to_string())
            .size(12.5)
            .color(theme::TEXT),
    )
    .fill(if tone == GlassTone::Light {
        theme::BG
    } else if has_glass {
        Color32::TRANSPARENT
    } else {
        Color32::from_white_alpha(12)
    })
    .stroke(Stroke::NONE)
    .corner_radius(egui::CornerRadius::same(100));
    let response = ui.add(button);
    if has_glass {
        let rect = response.rect;
        if let Some(shapes) = glass_shape(
            ui.ctx(),
            rect,
            rect.height() / 2.0,
            theme::GLASS,
            theme::GLASS_OPACITY,
        ) {
            for (slot, shape) in [mesh_slot, tint_slot].into_iter().zip(shapes) {
                painter.set(slot, shape);
            }
        }
    }
    response
}

/// Separador fino que no altera la altura de la fila de filtros.
pub fn divider(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(9.0, 16.0), Sense::hover());
    ui.painter().vline(
        rect.center().x,
        rect.y_range(),
        Stroke::new(1.0_f32, Color32::from_white_alpha(26)),
    );
}

pub fn card(ui: &mut Ui, add: impl FnOnce(&mut Ui)) -> Response {
    let width = ui.available_width();
    let frame = egui::Frame::new()
        .fill(theme::SURFACE)
        .stroke(Stroke::new(1.0_f32, Color32::from_white_alpha(10)))
        .corner_radius(egui::CornerRadius::same(theme::RADIUS as u8))
        .inner_margin(egui::Margin::same(14));
    frame
        .show(ui, |ui| {
            ui.set_min_width((width - 28.0).max(120.0));
            add(ui);
        })
        .response
}

pub fn section_title(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(13.0).color(theme::MUTED).strong());
    ui.add_space(2.0);
}
