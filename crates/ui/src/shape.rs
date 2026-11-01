use std::f32::consts::FRAC_PI_2;

use egui::epaint::{Mesh, Shadow};
use egui::{Color32, CornerRadius, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

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
pub fn inset_cut(cut: f32, inset: f32) -> f32 {
    (cut - inset * (2.0 - std::f32::consts::SQRT_2)).max(0.0)
}

pub fn fill(painter: &Painter, rect: Rect, cut: f32, fill: Color32, stroke: Stroke) {
    painter.add(Shape::convex_polygon(normform(rect, cut), fill, stroke));
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

pub fn glow(painter: &Painter, rect: Rect, cut: f32, color: Color32, blur: u8) {
    let radius = to_u8(cut);
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
    fn tiny_cuts_fall_back_to_a_rectangle() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(10.0, 10.0));
        assert_eq!(normform(rect, 0.1).len(), 4);
    }
}
