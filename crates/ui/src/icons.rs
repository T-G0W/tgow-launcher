use std::f32::consts::TAU;

use egui::epaint::PathStroke;
use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Library,
    Discover,
    Downloads,
    News,
    Settings,
    About,
    Chat,
    Globe,
    Code,
    Minimize,
    Back,
    Close,
    Search,
    More,
    Check,
    Expand,
    Collapse,
    Shield,
    Refresh,
    Play,
    Folder,
    Trash,
}

#[expect(clippy::too_many_lines)]
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let unit = rect.width().min(rect.height()) / 20.0;
    let stroke = Stroke::new(1.5 * unit.max(0.75), color);
    let c = rect.center();
    let at = |x: f32, y: f32| c + vec2(x, y) * unit;
    let line = |points: &[Pos2]| {
        painter.add(Shape::line(points.to_vec(), PathStroke::from(stroke)));
    };
    match icon {
        Icon::Library => {
            for (x, y) in [(-7.0, -7.0), (1.0, -7.0), (-7.0, 1.0), (1.0, 1.0)] {
                painter.rect_stroke(
                    Rect::from_min_max(at(x, y), at(x + 6.0, y + 6.0)),
                    0.0,
                    stroke,
                    egui::StrokeKind::Middle,
                );
            }
        }
        Icon::Discover => {
            painter.circle_stroke(c, 8.0 * unit, stroke);
            painter.add(Shape::convex_polygon(
                vec![at(0.0, -5.0), at(2.2, 0.0), at(0.0, 5.0), at(-2.2, 0.0)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Downloads => {
            line(&[at(0.0, -8.0), at(0.0, 3.0)]);
            line(&[at(-4.5, -1.5), at(0.0, 3.0), at(4.5, -1.5)]);
            line(&[at(-7.0, 7.5), at(7.0, 7.5)]);
        }
        Icon::News => {
            painter.rect_stroke(
                Rect::from_min_max(at(-8.0, -7.0), at(8.0, 7.0)),
                0.0,
                stroke,
                egui::StrokeKind::Middle,
            );
            line(&[at(-5.0, -3.0), at(5.0, -3.0)]);
            line(&[at(-5.0, 0.5), at(5.0, 0.5)]);
            line(&[at(-5.0, 4.0), at(1.0, 4.0)]);
        }
        Icon::Settings => {
            painter.circle_stroke(c, 5.0 * unit, stroke);
            painter.circle_stroke(c, 2.0 * unit, stroke);
            for tooth in 0..8_u8 {
                let angle = TAU * f32::from(tooth) / 8.0;
                let direction = vec2(angle.cos(), angle.sin());
                line(&[c + direction * 6.5 * unit, c + direction * 8.5 * unit]);
            }
        }
        Icon::About => {
            painter.circle_stroke(c, 8.0 * unit, stroke);
            line(&[at(0.0, -1.0), at(0.0, 5.0)]);
            painter.circle_filled(at(0.0, -4.5), 1.1 * unit, color);
        }
        Icon::Chat => {
            line(&[
                at(-8.0, -6.0),
                at(8.0, -6.0),
                at(8.0, 4.0),
                at(-1.0, 4.0),
                at(-5.0, 8.0),
                at(-5.0, 4.0),
                at(-8.0, 4.0),
                at(-8.0, -6.0),
            ]);
            painter.circle_filled(at(-3.5, -1.0), 1.1 * unit, color);
            painter.circle_filled(at(3.5, -1.0), 1.1 * unit, color);
        }
        Icon::Globe => {
            painter.circle_stroke(c, 8.0 * unit, stroke);
            line(&[at(-8.0, 0.0), at(8.0, 0.0)]);
            let meridian: Vec<Pos2> = (0..=16_u8)
                .map(|step| {
                    let angle = TAU * f32::from(step) / 16.0;
                    at(3.6 * angle.sin(), -8.0 * angle.cos())
                })
                .collect();
            painter.add(Shape::line(meridian, PathStroke::from(stroke)));
        }
        Icon::Code => {
            line(&[at(-3.5, -5.5), at(-8.0, 0.0), at(-3.5, 5.5)]);
            line(&[at(3.5, -5.5), at(8.0, 0.0), at(3.5, 5.5)]);
            line(&[at(1.5, -7.0), at(-1.5, 7.0)]);
        }
        Icon::Minimize => line(&[at(-5.0, 0.0), at(5.0, 0.0)]),
        Icon::Back => {
            line(&[at(7.0, 0.0), at(-6.0, 0.0)]);
            line(&[at(-1.0, -5.0), at(-6.0, 0.0), at(-1.0, 5.0)]);
        }
        Icon::Close => {
            line(&[at(-5.0, -5.0), at(5.0, 5.0)]);
            line(&[at(5.0, -5.0), at(-5.0, 5.0)]);
        }
        Icon::Search => {
            painter.circle_stroke(at(-1.5, -1.5), 5.5 * unit, stroke);
            line(&[at(2.5, 2.5), at(7.5, 7.5)]);
        }
        Icon::More => {
            for x in [-6.0, 0.0, 6.0] {
                painter.circle_filled(at(x, 0.0), 1.4 * unit, color);
            }
        }
        Icon::Check => line(&[at(-6.0, 0.5), at(-2.0, 4.5), at(6.5, -4.5)]),
        Icon::Play => {
            painter.add(Shape::convex_polygon(
                vec![at(-4.5, -7.0), at(7.0, 0.0), at(-4.5, 7.0)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Folder => line(&[
            at(-8.0, -6.0),
            at(-3.0, -6.0),
            at(-1.0, -3.5),
            at(8.0, -3.5),
            at(8.0, 6.5),
            at(-8.0, 6.5),
            at(-8.0, -6.0),
        ]),
        Icon::Trash => {
            line(&[at(-7.5, -5.0), at(7.5, -5.0)]);
            line(&[at(-2.5, -5.0), at(-2.0, -7.5), at(2.0, -7.5), at(2.5, -5.0)]);
            line(&[at(-5.5, -5.0), at(-4.5, 7.5), at(4.5, 7.5), at(5.5, -5.0)]);
            line(&[at(-1.5, -1.5), at(-1.5, 4.5)]);
            line(&[at(1.5, -1.5), at(1.5, 4.5)]);
        }
        Icon::Expand => line(&[at(-2.5, -6.0), at(3.5, 0.0), at(-2.5, 6.0)]),
        Icon::Collapse => line(&[at(2.5, -6.0), at(-3.5, 0.0), at(2.5, 6.0)]),
        Icon::Shield => {
            line(&[
                at(0.0, -8.0),
                at(7.0, -5.0),
                at(7.0, 0.0),
                at(0.0, 8.0),
                at(-7.0, 0.0),
                at(-7.0, -5.0),
                at(0.0, -8.0),
            ]);
            line(&[at(-3.0, 0.0), at(-0.5, 2.5), at(3.5, -2.5)]);
        }
        Icon::Refresh => {
            let arc: Vec<Pos2> = (0..=20_u8)
                .map(|step| {
                    let angle = -0.4 * TAU + 0.8 * TAU * f32::from(step) / 20.0;
                    pos2(
                        c.x + 7.0 * unit * angle.cos(),
                        c.y + 7.0 * unit * angle.sin(),
                    )
                })
                .collect();
            let end = arc.last().copied().unwrap_or(c);
            painter.add(Shape::line(arc, PathStroke::from(stroke)));
            line(&[
                end + vec2(-3.5, -1.0) * unit,
                end,
                end + vec2(0.5, -3.8) * unit,
            ]);
        }
    }
}
