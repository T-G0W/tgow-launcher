use std::f32::consts::{FRAC_PI_2, PI};

use egui::epaint::{Mesh, Shadow};
use egui::{Color32, CornerRadius, Painter, Pos2, Rect, Stroke, StrokeKind, pos2, vec2};

const ARC_STEPS: u16 = 10;

#[must_use]
pub fn normform(rect: Rect, cut: f32) -> Vec<Pos2> {
    let cut = cut.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let (left, top, right, bottom) = (rect.left(), rect.top(), rect.right(), rect.bottom());
    if cut < 0.5 {
        return vec![
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ];
    }
    let mut points = Vec::with_capacity(usize::from(ARC_STEPS) + 7);
    points.push(pos2(left + cut, top));
    points.extend(quarter_arc(pos2(right - cut, top + cut), cut));
    points.extend([
        pos2(right, bottom - cut),
        pos2(right - cut, bottom),
        pos2(left + cut, bottom),
        pos2(left, bottom - cut),
        pos2(left, top + cut),
    ]);
    points
}

#[must_use]
pub fn rounded(rect: Rect, radius: f32) -> Vec<Pos2> {
    let radius = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    if radius < 0.5 {
        return vec![
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ];
    }
    let corners = [
        (pos2(rect.left() + radius, rect.top() + radius), PI),
        (
            pos2(rect.right() - radius, rect.top() + radius),
            PI + FRAC_PI_2,
        ),
        (pos2(rect.right() - radius, rect.bottom() - radius), 0.0),
        (
            pos2(rect.left() + radius, rect.bottom() - radius),
            FRAC_PI_2,
        ),
    ];
    corners
        .into_iter()
        .flat_map(|(center, start)| corner(center, radius, start))
        .collect()
}

#[must_use]
pub fn rounded_top(rect: Rect, radius: f32) -> Vec<Pos2> {
    let radius = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let mut points: Vec<Pos2> =
        corner(pos2(rect.left() + radius, rect.top() + radius), radius, PI).collect();
    points.extend(corner(
        pos2(rect.right() - radius, rect.top() + radius),
        radius,
        PI + FRAC_PI_2,
    ));
    points.extend([rect.right_bottom(), rect.left_bottom()]);
    points
}

fn corner(center: Pos2, radius: f32, start: f32) -> impl Iterator<Item = Pos2> {
    let steps = arc_steps(radius);
    (0..=steps).map(move |step| {
        let angle = start + FRAC_PI_2 * f32::from(step) / f32::from(steps);
        center + radius * vec2(angle.cos(), angle.sin())
    })
}

fn arc_steps(radius: f32) -> u16 {
    if radius < 3.0 {
        2
    } else if radius < 8.0 {
        4
    } else {
        ARC_STEPS
    }
}

#[must_use]
pub fn rounded_top_right(rect: Rect, radius: f32) -> Vec<Pos2> {
    let radius = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let mut points = vec![rect.left_top()];
    points.extend(quarter_arc(
        pos2(rect.right() - radius, rect.top() + radius),
        radius,
    ));
    points.extend([rect.right_bottom(), rect.left_bottom()]);
    points
}

fn quarter_arc(center: Pos2, radius: f32) -> impl Iterator<Item = Pos2> {
    (0..=ARC_STEPS).map(move |step| {
        let angle = -FRAC_PI_2 + FRAC_PI_2 * f32::from(step) / f32::from(ARC_STEPS);
        center + radius * vec2(angle.cos(), angle.sin())
    })
}

#[must_use]
pub fn cover_uv(image: egui::Vec2, target: egui::Vec2) -> Rect {
    if image.x <= 0.0 || image.y <= 0.0 || target.x <= 0.0 || target.y <= 0.0 {
        return Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    }
    let image_ratio = image.x / image.y;
    let target_ratio = target.x / target.y;
    if image_ratio > target_ratio {
        let width = target_ratio / image_ratio;
        let left = (1.0 - width) / 2.0;
        Rect::from_min_max(pos2(left, 0.0), pos2(left + width, 1.0))
    } else {
        let height = image_ratio / target_ratio;
        let top = (1.0 - height) / 2.0;
        Rect::from_min_max(pos2(0.0, top), pos2(1.0, top + height))
    }
}

#[must_use]
pub fn inset_cut(cut: f32, inset: f32) -> f32 {
    (cut - inset * (2.0 - std::f32::consts::SQRT_2)).max(0.0)
}

pub fn fill(painter: &Painter, rect: Rect, radius: f32, fill: Color32, stroke: Stroke) {
    painter.rect(
        rect,
        CornerRadius::same(to_u8(radius)),
        fill,
        stroke,
        StrokeKind::Inside,
    );
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[must_use]
pub fn to_u8(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[must_use]
pub fn to_usize(value: f32) -> usize {
    value.max(0.0) as usize
}

#[expect(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
#[must_use]
pub fn ratio(part: u64, whole: u64) -> f32 {
    (part.min(whole) as f64 / whole.max(1) as f64) as f32
}

#[expect(clippy::cast_possible_truncation)]
#[must_use]
pub fn seconds(time: f64) -> f32 {
    (time % 86_400.0) as f32
}

pub fn glow(painter: &Painter, rect: Rect, radius: f32, color: Color32, blur: u8) {
    let radius = to_u8(radius);
    let shadow = Shadow {
        offset: [0, 0],
        blur,
        spread: 0,
        color,
    };
    painter.add(shadow.as_shape(rect, CornerRadius::same(radius)));
}

#[must_use]
pub fn vertical_gradient(points: &[Pos2], top: Color32, bottom: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    let Some(bounds) = bounds(points) else {
        return mesh;
    };
    let color_at = |y: f32| {
        let t = if bounds.height() > 0.0 {
            (y - bounds.top()) / bounds.height()
        } else {
            0.0
        };
        top.lerp_to_gamma(bottom, t)
    };
    mesh.colored_vertex(bounds.center(), color_at(bounds.center().y));
    for point in points {
        mesh.colored_vertex(*point, color_at(point.y));
    }
    let count = u32::try_from(points.len()).unwrap_or(0);
    for index in 1..=count {
        let next = if index == count { 1 } else { index + 1 };
        mesh.add_triangle(0, index, next);
    }
    mesh
}

#[must_use]
pub fn textured_polygon(
    points: &[Pos2],
    frame: Rect,
    uv: Rect,
    texture: egui::TextureId,
    top: f32,
    bottom: f32,
) -> Mesh {
    let mut mesh = Mesh::with_texture(texture);
    let vertex = |mesh: &mut Mesh, point: Pos2| {
        let along =
            ((point - frame.min) / frame.size()).clamp(egui::Vec2::ZERO, egui::Vec2::splat(1.0));
        let alpha = top + (bottom - top) * along.y;
        mesh.vertices.push(egui::epaint::Vertex {
            pos: point,
            uv: uv.min + along * uv.size(),
            color: Color32::WHITE.gamma_multiply(alpha.clamp(0.0, 1.0)),
        });
    };
    vertex(&mut mesh, frame.center());
    for point in points {
        vertex(&mut mesh, *point);
    }
    let count = u32::try_from(points.len()).unwrap_or(0);
    for index in 1..=count {
        let next = if index == count { 1 } else { index + 1 };
        mesh.add_triangle(0, index, next);
    }
    mesh
}

#[must_use]
pub fn faded_image(rect: Rect, uv: Rect, texture: egui::TextureId, fade_from: f32) -> Mesh {
    let mut mesh = Mesh::with_texture(texture);
    let split = rect.top() + rect.height() * fade_from.clamp(0.0, 1.0);
    let split_uv = uv.top() + uv.height() * fade_from.clamp(0.0, 1.0);
    let rows = [
        (rect.top(), uv.top(), Color32::WHITE),
        (split, split_uv, Color32::WHITE),
        (rect.bottom(), uv.bottom(), Color32::TRANSPARENT),
    ];
    for (y, v, color) in rows {
        for (x, u) in [(rect.left(), uv.left()), (rect.right(), uv.right())] {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: pos2(x, y),
                uv: pos2(u, v),
                color,
            });
        }
    }
    for row in 0..2_u32 {
        let base = row * 2;
        mesh.add_triangle(base, base + 1, base + 3);
        mesh.add_triangle(base, base + 3, base + 2);
    }
    mesh
}

#[must_use]
pub fn shade(rect: Rect, color: Color32, reach: f32, fade_from: f32) -> Mesh {
    let mut mesh = Mesh::default();
    let right = rect.left() + rect.width() * reach.clamp(0.0, 1.0);
    let split = rect.top() + rect.height() * fade_from.clamp(0.0, 1.0);
    for (y, strength) in [(rect.top(), 1.0), (split, 1.0), (rect.bottom(), 0.0)] {
        mesh.colored_vertex(pos2(rect.left(), y), color.gamma_multiply(strength));
        mesh.colored_vertex(pos2(right, y), Color32::TRANSPARENT);
    }
    for row in 0..2_u32 {
        let base = row * 2;
        mesh.add_triangle(base, base + 1, base + 3);
        mesh.add_triangle(base, base + 3, base + 2);
    }
    mesh
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Down,
    Right,
}

#[must_use]
pub fn rect_gradient(rect: Rect, from: Color32, to: Color32, direction: Direction) -> Mesh {
    let mut mesh = Mesh::default();
    let (a, b, c, d) = match direction {
        Direction::Down => (from, from, to, to),
        Direction::Right => (from, to, to, from),
    };
    mesh.colored_vertex(rect.left_top(), a);
    mesh.colored_vertex(rect.right_top(), b);
    mesh.colored_vertex(rect.right_bottom(), c);
    mesh.colored_vertex(rect.left_bottom(), d);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    mesh
}

fn bounds(points: &[Pos2]) -> Option<Rect> {
    let first = *points.first()?;
    Some(
        points
            .iter()
            .fold(Rect::from_min_max(first, first), |rect, point| {
                rect.union(Rect::from_min_max(*point, *point))
            }),
    )
}

#[must_use]
pub fn contains(points: &[Pos2], point: Pos2) -> bool {
    let mut side: Option<bool> = None;
    for (index, a) in points.iter().enumerate() {
        let b = points[(index + 1) % points.len()];
        let cross = (b - *a).x * (point - *a).y - (b - *a).y * (point - *a).x;
        if cross.abs() < f32::EPSILON {
            continue;
        }
        let positive = cross > 0.0;
        match side {
            None => side = Some(positive),
            Some(expected) if expected != positive => return false,
            Some(_) => {}
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use egui::{Rect, pos2};

    use super::{contains, normform};

    #[test]
    fn normform_has_square_corners_except_the_rounded_top_right() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0));
        let points = normform(rect, 20.0);
        assert!(contains(&points, pos2(50.0, 50.0)));
        assert!(!contains(&points, pos2(2.0, 2.0)));
        assert!(!contains(&points, pos2(98.0, 98.0)));
        assert!(!contains(&points, pos2(97.0, 3.0)));
        assert!(contains(&points, pos2(85.0, 15.0)));
        assert!(
            points
                .iter()
                .all(|point| rect.expand(0.01).contains(*point))
        );
    }

    #[test]
    fn cover_crops_the_longer_side_symmetrically() {
        let wide = super::cover_uv(egui::vec2(400.0, 100.0), egui::vec2(200.0, 100.0));
        assert!((wide.left() - 0.25).abs() < 1e-6 && (wide.right() - 0.75).abs() < 1e-6);
        assert!((wide.top()).abs() < 1e-6 && (wide.bottom() - 1.0).abs() < 1e-6);
        let tall = super::cover_uv(egui::vec2(100.0, 400.0), egui::vec2(100.0, 100.0));
        assert!((tall.top() - 0.375).abs() < 1e-6 && (tall.bottom() - 0.625).abs() < 1e-6);
    }

    #[test]
    fn tiny_cuts_fall_back_to_a_rectangle() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(10.0, 10.0));
        assert_eq!(normform(rect, 0.1).len(), 4);
    }
}
