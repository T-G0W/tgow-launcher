use std::f32::consts::TAU;

use egui::{Color32, Painter, Rect, Shape, pos2};

use crate::shape::{Direction, rect_gradient};
use crate::theme;

const STARS: usize = 220;
const DRIFT: f32 = 0.004;

#[derive(Debug, Clone, Copy)]
struct Star {
    x: f32,
    y: f32,
    depth: f32,
    phase: f32,
}

#[derive(Debug, Clone)]
pub(crate) struct Starfield {
    stars: Vec<Star>,
}

impl Default for Starfield {
    fn default() -> Self {
        let mut seed: u32 = 0x9E37_79B9;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            f32::from(u16::try_from(seed >> 16).unwrap_or(0)) / f32::from(u16::MAX)
        };
        let stars = (0..STARS)
            .map(|_| Star {
                x: next(),
                y: next(),
                depth: next().powi(2),
                phase: next() * TAU,
            })
            .collect();
        Self { stars }
    }
}

impl Starfield {
    pub(crate) fn paint(&self, painter: &Painter, rect: Rect, time: f32) {
        painter.add(Shape::mesh(rect_gradient(
            rect,
            theme::BG,
            theme::BG_DEEP,
            Direction::Down,
        )));
        for (center, radius, alpha) in
            [(pos2(0.78, 0.18), 0.55, 14_u8), (pos2(0.15, 0.85), 0.45, 9)]
        {
            let center = pos2(
                rect.left() + center.x * rect.width(),
                rect.top() + center.y * rect.height(),
            );
            let radius = radius * rect.width().max(rect.height());
            for ring in (1..=14_u8).rev() {
                let fraction = f32::from(ring) / 14.0;
                painter.circle_filled(
                    center,
                    radius * fraction,
                    Color32::from_rgba_unmultiplied(0x4F, 0xB8, 0xFF, alpha / 4),
                );
            }
        }
        for star in &self.stars {
            let x = (star.x + time * DRIFT * (0.3 + star.depth)).fract();
            let twinkle = 0.55 + 0.45 * (time * (0.6 + star.depth) + star.phase).sin();
            let brightness = (40.0 + 180.0 * star.depth) * twinkle;
            let tint =
                Color32::from_rgba_unmultiplied(0xC8, 0xE6, 0xFF, crate::shape::to_u8(brightness));
            painter.circle_filled(
                pos2(
                    rect.left() + x * rect.width(),
                    rect.top() + star.y * rect.height(),
                ),
                0.5 + 1.2 * star.depth,
                tint,
            );
        }
    }
}
