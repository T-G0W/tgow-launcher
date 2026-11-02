pub(crate) mod about;
pub(crate) mod app;
pub(crate) mod downloads;
pub(crate) mod library;
pub(crate) mod news;
pub(crate) mod settings;
pub(crate) mod welcome;

use egui::{Color32, Painter, Pos2, Rect, Stroke, vec2};

use crate::shape;
use crate::theme::{self, Font};
use crate::widgets;

pub(crate) fn paint_chip(painter: &Painter, at: Pos2, label: &str, color: Color32) -> f32 {
    let galley = painter.layout_job(widgets::job(label, Font::Label, 10.0, color));
    let rect = Rect::from_min_size(at, galley.size() + vec2(18.0, 10.0));
    shape::fill(
        painter,
        rect,
        4.0,
        color.gamma_multiply(0.12),
        Stroke::new(1.0, color.gamma_multiply(0.6)),
    );
    painter.galley(rect.center() - galley.size() / 2.0, galley, color);
    rect.width()
}

pub(crate) fn accent_of(app: &tgow_core::catalog::App) -> Color32 {
    app.manifest()
        .and_then(|manifest| manifest.accent)
        .map_or(theme::ACCENT, |accent| {
            let [r, g, b] = accent.0;
            Color32::from_rgb(r, g, b)
        })
}

pub(crate) fn initial(name: &str) -> String {
    name.chars()
        .find(|c| c.is_alphanumeric())
        .map_or_else(|| "T".to_owned(), |c| c.to_uppercase().collect())
}
