use egui::epaint::PathStroke;
use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

use crate::shape::{contains, inset_cut, normform, rounded_top_right, vertical_gradient};
use crate::theme;

pub const CHAMFER: f32 = 0.292_893_2;
const BOX: f32 = 512.0;
const MARGIN: f32 = 24.0;
const RIM_INSET: f32 = 16.0;
const RIM_WIDTH: f32 = 14.0;
const BODY_TOP: Color32 = Color32::from_rgb(0x16, 0x37, 0x4B);
const BODY_BOTTOM: Color32 = theme::BG;
const MARK_TOP: Color32 = theme::TEXT;
const MARK_BOTTOM: Color32 = Color32::from_rgb(0xB8, 0xDF, 0xFF);

struct Layout {
    outer: Vec<Pos2>,
    rim: Vec<Pos2>,
    crossbar: Vec<Pos2>,
    stem: Rect,
    scale: f32,
}

fn layout(rect: Rect) -> Layout {
    let side = rect.width().min(rect.height());
    let origin = rect.center() - vec2(side, side) / 2.0;
    let scale = side / BOX;
    let at = |x: f32, y: f32| origin + vec2(x, y) * scale;
    let body = Rect::from_min_max(at(MARGIN, MARGIN), at(BOX - MARGIN, BOX - MARGIN));
    let cut = body.width() * CHAMFER;
    let rim_rect = body.shrink(RIM_INSET * scale);
    Layout {
        outer: normform(body, cut),
        rim: normform(rim_rect, inset_cut(cut, RIM_INSET * scale)),
        crossbar: rounded_top_right(
            Rect::from_min_max(at(150.0, 150.0), at(362.0, 210.0)),
            30.0 * scale,
        ),
        stem: Rect::from_min_max(at(226.0, 209.0), at(286.0, 372.0)),
        scale,
    }
}

pub fn paint_mark(painter: &Painter, rect: Rect) {
    let layout = layout(rect);
    painter.add(Shape::mesh(vertical_gradient(
        &layout.outer,
        BODY_TOP,
        BODY_BOTTOM,
    )));
    painter.add(Shape::closed_line(
        layout.rim,
        PathStroke::new(RIM_WIDTH * layout.scale, theme::ACCENT),
    ));
    painter.add(Shape::mesh(vertical_gradient(
        &layout.crossbar,
        MARK_TOP,
        MARK_TOP,
    )));
    painter.add(Shape::mesh(vertical_gradient(
        &[
            layout.stem.left_top(),
            layout.stem.right_top(),
            layout.stem.right_bottom(),
            layout.stem.left_bottom(),
        ],
        MARK_TOP,
        MARK_BOTTOM,
    )));
    painter.add(Shape::closed_line(
        layout.outer,
        Stroke::new(1.0, theme::ACCENT_DEEP),
    ));
}

#[must_use]
pub fn icon_rgba(size: u32) -> Vec<u8> {
    let extent = f32::from(u16::try_from(size).unwrap_or(u16::MAX));
    let layout = layout(Rect::from_min_max(pos2(0.0, 0.0), pos2(extent, extent)));
    let rim_inner = normform(
        bounds_shrunk(&layout.rim, RIM_WIDTH * layout.scale / 2.0),
        inset_cut(
            extent * (1.0 - 2.0 * MARGIN / BOX) * CHAMFER,
            (RIM_INSET + RIM_WIDTH / 2.0) * layout.scale,
        ),
    );
    let rim_outer = normform(
        bounds_shrunk(&layout.rim, -RIM_WIDTH * layout.scale / 2.0),
        inset_cut(
            extent * (1.0 - 2.0 * MARGIN / BOX) * CHAMFER,
            (RIM_INSET - RIM_WIDTH / 2.0) * layout.scale,
        ),
    );
    let samples: [f32; 4] = [0.125, 0.375, 0.625, 0.875];
    let mut pixels = Vec::with_capacity(usize::try_from(size * size * 4).unwrap_or(0));
    for row in 0..size {
        for column in 0..size {
            let mut sum = [0.0_f32; 4];
            for dy in samples {
                for dx in samples {
                    let point = pos2(to_f32(column) + dx, to_f32(row) + dy);
                    let color = sample(&layout, &rim_outer, &rim_inner, point, extent);
                    let alpha = f32::from(color.a()) / 255.0;
                    sum[0] += f32::from(color.r()) * alpha;
                    sum[1] += f32::from(color.g()) * alpha;
                    sum[2] += f32::from(color.b()) * alpha;
                    sum[3] += alpha;
                }
            }
            let coverage = sum[3] / 16.0;
            if sum[3] > 0.0 {
                pixels.extend([
                    channel(sum[0] / sum[3]),
                    channel(sum[1] / sum[3]),
                    channel(sum[2] / sum[3]),
                    channel(coverage * 255.0),
                ]);
            } else {
                pixels.extend([0, 0, 0, 0]);
            }
        }
    }
    pixels
}

fn sample(
    layout: &Layout,
    rim_outer: &[Pos2],
    rim_inner: &[Pos2],
    point: Pos2,
    side: f32,
) -> Color32 {
    if !contains(&layout.outer, point) {
        return Color32::TRANSPARENT;
    }
    let t = point.y / side;
    if contains(&layout.crossbar, point) || layout.stem.contains(point) {
        return MARK_TOP.lerp_to_gamma(MARK_BOTTOM, t);
    }
    if contains(rim_outer, point) && !contains(rim_inner, point) {
        return theme::ACCENT_SOFT.lerp_to_gamma(theme::ACCENT_DEEP, t);
    }
    BODY_TOP.lerp_to_gamma(BODY_BOTTOM, t)
}

fn bounds_shrunk(points: &[Pos2], amount: f32) -> Rect {
    let first = points.first().copied().unwrap_or_default();
    points
        .iter()
        .fold(Rect::from_min_max(first, first), |rect, point| {
            rect.union(Rect::from_min_max(*point, *point))
        })
        .shrink(amount)
}

fn to_f32(value: u32) -> f32 {
    f32::from(u16::try_from(value).unwrap_or(u16::MAX))
}

fn channel(value: f32) -> u8 {
    crate::shape::to_u8(value)
}

#[cfg(test)]
mod tests {
    use super::icon_rgba;

    #[test]
    fn icon_is_transparent_outside_and_opaque_in_the_middle() {
        let size = 64;
        let pixels = icon_rgba(size);
        assert_eq!(pixels.len(), 64 * 64 * 4);
        let alpha = |x: usize, y: usize| pixels[(y * 64 + x) * 4 + 3];
        assert_eq!(alpha(0, 0), 0);
        assert_eq!(alpha(63, 63), 0);
        assert_eq!(alpha(32, 32), 255);
        assert!(alpha(60, 4) < alpha(4, 32));
    }
}
