use egui::{
    epaint::Vertex, Color32, Mesh, Pos2, Rect, Response, RichText, Sense, Stroke, TextureId, Ui, Vec2,
};

use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Grid,
    Gear,
    Play,
    Star,
    Plus,
    Close,
    Minimize,
    Maximize,
    ChevronLeft,
    Trash,
}

pub fn icon(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32, width: f32) {
    let p = |x: f32, y: f32| rect.lerp_inside(Vec2::new(x, y));
    let stroke = Stroke::new(width, color);
    let line = |a: Pos2, b: Pos2| painter.line_segment([a, b], stroke);
    let poly = |points: &[(f32, f32)]| {
        let pts: Vec<Pos2> = points.iter().map(|(x, y)| p(*x, *y)).collect();
        painter.add(egui::Shape::line(pts, stroke));
    };

    match icon {
        Icon::Grid => {
            for (x, y) in [(0.14, 0.14), (0.56, 0.14), (0.14, 0.56), (0.56, 0.56)] {
                painter.rect_stroke(
                    Rect::from_min_size(p(x, y), Vec2::new(rect.width() * 0.3, rect.height() * 0.3)),
                    egui::CornerRadius::same(2),
                    stroke,
                    egui::StrokeKind::Middle,
                );
            }
        }
        Icon::Gear => {
            let center = rect.center();
            let radius = rect.width() * 0.26;
            painter.circle_stroke(center, radius, stroke);
            painter.circle_stroke(center, radius * 0.38, stroke);
            for step in 0..8 {
                let angle = step as f32 * std::f32::consts::TAU / 8.0;
                let dir = Vec2::angled(angle);
                line(center + dir * radius, center + dir * (radius + rect.width() * 0.14));
            }
        }
        Icon::Play => {
            painter.add(egui::Shape::convex_polygon(
                vec![p(0.3, 0.2), p(0.84, 0.5), p(0.3, 0.8)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Star => {
            let mut points = Vec::new();
            for i in 0..10 {
                let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
                let radius = if i % 2 == 0 { 0.4 } else { 0.17 };
                points.push((0.5 + radius * angle.cos(), 0.5 + radius * angle.sin()));
            }
            poly(&points);
            let first = points[0];
            line(p(first.0, first.1), p(points[points.len() - 1].0, points[points.len() - 1].1));
        }
        Icon::Plus => {
            line(p(0.5, 0.2), p(0.5, 0.8));
            line(p(0.2, 0.5), p(0.8, 0.5));
        }
        Icon::Close => {
            line(p(0.28, 0.28), p(0.72, 0.72));
            line(p(0.72, 0.28), p(0.28, 0.72));
        }
        Icon::Minimize => {
            line(p(0.28, 0.5), p(0.72, 0.5));
        }
        Icon::Maximize => {
            painter.rect_stroke(
                Rect::from_center_size(rect.center(), Vec2::splat(rect.width() * 0.42)),
                egui::CornerRadius::same(1),
                stroke,
                egui::StrokeKind::Middle,
            );
        }
        Icon::ChevronLeft => {
            line(p(0.6, 0.24), p(0.36, 0.5));
            line(p(0.36, 0.5), p(0.6, 0.76));
        }
        Icon::Trash => {
            poly(&[(0.22, 0.3), (0.78, 0.3)]);
            poly(&[(0.34, 0.3), (0.38, 0.18), (0.62, 0.18), (0.66, 0.3)]);
            poly(&[(0.3, 0.3), (0.34, 0.82), (0.66, 0.82), (0.7, 0.3)]);
            line(p(0.46, 0.42), p(0.47, 0.7));
            line(p(0.54, 0.42), p(0.53, 0.7));
        }
    }
}

pub fn rounded_points(rect: Rect, radius: f32) -> Vec<Pos2> {
    let radius = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    if radius <= 0.5 {
        return vec![
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ];
    }
    let centers = [
        (Pos2::new(rect.min.x + radius, rect.min.y + radius), std::f32::consts::PI),
        (Pos2::new(rect.max.x - radius, rect.min.y + radius), 1.5 * std::f32::consts::PI),
        (Pos2::new(rect.max.x - radius, rect.max.y - radius), 0.0),
        (Pos2::new(rect.min.x + radius, rect.max.y - radius), 0.5 * std::f32::consts::PI),
    ];
    let mut points = Vec::with_capacity(4 * 5);
    for (center, start) in centers {
        for step in 0..=4 {
            let angle = start + step as f32 * (std::f32::consts::FRAC_PI_2 / 4.0);
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
    if rect.width() < 2.0 || rect.height() < 2.0 {
        return;
    }
    let points = rounded_points(rect, radius);
    let uv = cover_uv(texture_size, rect.size());
    let mut mesh = Mesh::default();
    let center = rect.center();
    mesh.vertices.push(Vertex {
        pos: center,
        uv: Pos2::new(uv.center().x, uv.center().y),
        color: Color32::WHITE,
    });
    for point in &points {
        let rel = Vec2::new(
            (point.x - rect.min.x) / rect.width(),
            (point.y - rect.min.y) / rect.height(),
        );
        mesh.vertices.push(Vertex {
            pos: *point,
            uv: Pos2::new(
                uv.min.x + rel.x * uv.width(),
                uv.min.y + rel.y * uv.height(),
            ),
            color: Color32::WHITE,
        });
    }
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        mesh.add_triangle(0, index as u32 + 1, next as u32 + 1);
    }
    mesh.texture_id = texture;
    painter.add(egui::Shape::mesh(mesh));
}

/// Degradado lineal de dos colores, útil para scrims sobre el arte.
pub fn gradient(painter: &egui::Painter, rect: Rect, from: Color32, to: Color32, vertical: bool) {
    let mut mesh = Mesh::default();
    let corners = if vertical {
        [
            (rect.left_top(), from),
            (rect.right_top(), from),
            (rect.right_bottom(), to),
            (rect.left_bottom(), to),
        ]
    } else {
        [
            (rect.left_top(), from),
            (rect.right_top(), to),
            (rect.right_bottom(), to),
            (rect.left_bottom(), from),
        ]
    };
    for (pos, color) in corners {
        mesh.vertices.push(Vertex {
            pos,
            uv: Pos2::ZERO,
            color,
        });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

pub fn icon_button(ui: &mut Ui, icon_kind: Icon, size: Vec2, active: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let fill = if active {
        theme::ACCENT
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
    let color = if active { theme::ON_ACCENT } else { theme::TEXT };
    icon(
        ui.painter(),
        Rect::from_center_size(rect.center(), size * 0.44),
        icon_kind,
        color,
        1.6,
    );
    response
}

pub fn pill(ui: &mut Ui, text: &str, filled: bool, width: f32) -> Response {
    let (fill, color, stroke) = if filled {
        (theme::ACCENT, theme::ON_ACCENT, Stroke::NONE)
    } else {
        (
            Color32::from_white_alpha(16),
            theme::TEXT,
            Stroke::new(1.0_f32, Color32::from_white_alpha(28)),
        )
    };
    let button = egui::Button::new(RichText::new(text).color(color).strong())
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(100));
    ui.add_sized([width, 38.0], button)
}

pub fn chip(ui: &mut Ui, label: &str, active: bool) -> Response {
    let button = egui::Button::new(RichText::new(label).size(12.5).color(if active {
        theme::ON_ACCENT
    } else {
        theme::MUTED
    }))
    .fill(if active {
        theme::ACCENT
    } else {
        Color32::from_white_alpha(12)
    })
    .stroke(Stroke::NONE)
    .corner_radius(egui::CornerRadius::same(100));
    ui.add(button)
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
