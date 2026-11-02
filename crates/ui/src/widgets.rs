use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align2, Color32, Context, CursorIcon, Galley, Id, Rect, Response, Role, Sense, Shape, Stroke,
    Ui, Vec2, WidgetInfo, pos2, vec2,
};

use crate::icons::{self, Icon};
use crate::shape;
use crate::theme::{self, Font};

const MOTION: &str = "tgow.reduced-motion";

pub fn set_reduced_motion(ctx: &Context, reduced: bool) {
    ctx.data_mut(|data| data.insert_temp(Id::unique(MOTION), reduced));
}

#[must_use]
pub fn reduced_motion(ctx: &Context) -> bool {
    ctx.data(|data| data.get_temp::<bool>(Id::unique(MOTION)))
        .unwrap_or(false)
}

#[must_use]
pub fn animate(ctx: &Context, id: Id, on: bool, seconds: f32) -> f32 {
    let seconds = if reduced_motion(ctx) { 0.0 } else { seconds };
    ctx.animate_bool_with_time(id, on, seconds)
}

#[must_use]
pub fn job(text: &str, font: Font, size: f32, color: Color32) -> LayoutJob {
    let text = if font.is_uppercase() {
        text.to_uppercase()
    } else {
        text.to_owned()
    };
    let mut job = LayoutJob::default();
    job.append(
        &text,
        0.0,
        TextFormat {
            font_id: font.id(size),
            color,
            extra_letter_spacing: font.letter_spacing(size),
            ..TextFormat::default()
        },
    );
    job
}

#[must_use]
pub fn galley(ui: &Ui, text: &str, font: Font, size: f32, color: Color32) -> Arc<Galley> {
    ui.painter().layout_job(job(text, font, size, color))
}

pub fn text(ui: &mut Ui, text: &str, font: Font, size: f32, color: Color32) -> Response {
    ui.label(job(text, font, size, color))
}

pub fn paragraph(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let mut job = job(text, Font::Body, 15.0, color);
    job.wrap.max_width = ui.available_width();
    ui.label(job)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emphasis {
    Primary,
    Secondary,
    Danger,
}

pub fn button(ui: &mut Ui, label: &str, emphasis: Emphasis, enabled: bool) -> Response {
    let size = if emphasis == Emphasis::Primary {
        15.0
    } else {
        13.0
    };
    let galley = galley(ui, label, Font::Button, size, theme::TEXT);
    let height = if emphasis == Emphasis::Primary {
        48.0
    } else {
        36.0
    };
    let padding = if emphasis == Emphasis::Primary {
        30.0
    } else {
        18.0
    };
    let desired = vec2((galley.size().x + padding * 2.0).max(height * 2.5), height);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(desired, sense);
    response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, label));
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let hover = animate(
        ui.ctx(),
        response.id.with("hover"),
        enabled && response.hovered(),
        theme::FAST,
    );
    let cut = if emphasis == Emphasis::Primary {
        10.0
    } else {
        6.0
    };
    let painter = ui.painter();
    let (fill, stroke, text_color) = match (emphasis, enabled) {
        (_, false) => (
            theme::SURFACE_1,
            Stroke::new(1.0, theme::LINE),
            theme::TEXT_FAINT,
        ),
        (Emphasis::Primary, true) => {
            shape::glow(
                painter,
                rect,
                cut,
                theme::ACCENT.gamma_multiply(0.25 + 0.3 * hover),
                24,
            );
            let fill = if response.is_pointer_button_down_on() {
                theme::ACCENT_DEEP
            } else {
                theme::ACCENT.lerp_to_gamma(theme::ACCENT_SOFT, hover)
            };
            (fill, Stroke::NONE, theme::ON_ACCENT)
        }
        (Emphasis::Secondary, true) => (
            theme::SURFACE_1.lerp_to_gamma(theme::SURFACE_2, hover),
            Stroke::new(1.0, theme::LINE_STRONG.lerp_to_gamma(theme::ACCENT, hover)),
            theme::TEXT,
        ),
        (Emphasis::Danger, true) => (
            theme::SURFACE_1.lerp_to_gamma(theme::DANGER.gamma_multiply(0.25), hover),
            Stroke::new(1.0, theme::DANGER),
            theme::TEXT,
        ),
    };
    shape::fill(painter, rect, cut, fill, stroke);
    focus_ring(ui, &response, rect, cut);
    painter.galley(
        rect.center() - galley.size() / 2.0,
        ui.painter()
            .layout_job(job(label, Font::Button, size, text_color)),
        text_color,
    );
    if enabled {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    }
}

pub fn icon_button(ui: &mut Ui, icon: Icon, tooltip: &str, side: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, tooltip));
    focus_ring(ui, &response, rect, 4.0);
    let hover = animate(
        ui.ctx(),
        response.id.with("hover"),
        response.hovered(),
        theme::FAST,
    );
    if hover > 0.0 {
        shape::fill(
            ui.painter(),
            rect,
            4.0,
            theme::SURFACE_2.gamma_multiply(hover),
            Stroke::NONE,
        );
    }
    let color = theme::TEXT_MUTED.lerp_to_gamma(theme::ACCENT_SOFT, hover);
    icons::paint(ui.painter(), rect.shrink(side * 0.25), icon, color);
    response
        .on_hover_text(tooltip)
        .on_hover_cursor(CursorIcon::PointingHand)
}

pub fn window_button(ui: &mut Ui, icon: Icon, tooltip: &str, danger: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(46.0, theme::TITLE_BAR), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, tooltip));
    let hover = animate(
        ui.ctx(),
        response.id.with("hover"),
        response.hovered(),
        theme::FAST,
    );
    let fill = if danger {
        theme::DANGER
    } else {
        theme::SURFACE_2
    };
    if hover > 0.0 {
        ui.painter()
            .rect_filled(rect, 0.0, fill.gamma_multiply(hover));
    }
    let color = theme::TEXT_MUTED.lerp_to_gamma(theme::TEXT, hover);
    icons::paint(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(18.0)),
        icon,
        color,
    );
    response.on_hover_text(tooltip)
}

pub fn nav_item(ui: &mut Ui, icon: Icon, label: &str, selected: bool, expansion: f32) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, 48.0), Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::Tab, true, selected, label));
    let hover = animate(
        ui.ctx(),
        response.id.with("hover"),
        response.hovered(),
        theme::FAST,
    );
    let active = animate(
        ui.ctx(),
        response.id.with("active"),
        selected,
        theme::NORMAL,
    );
    let painter = ui.painter();
    let tile = rect.shrink2(vec2(12.0, 4.0));
    if hover > 0.0 || active > 0.0 {
        let fill = theme::SURFACE_2
            .gamma_multiply(hover.max(active * 0.8))
            .lerp_to_gamma(theme::ACCENT_DEEP.gamma_multiply(0.55), active * 0.5);
        shape::fill(painter, tile, 6.0, fill, Stroke::NONE);
    }
    if active > 0.0 {
        let bar = Rect::from_min_size(
            pos2(rect.left(), tile.top() + 8.0),
            vec2(3.0, tile.height() - 16.0),
        );
        painter.rect_filled(bar, 0.0, theme::ACCENT.gamma_multiply(active));
    }
    let icon_rect =
        Rect::from_center_size(pos2(rect.left() + 36.0, rect.center().y), Vec2::splat(22.0));
    let color = theme::TEXT_MUTED
        .lerp_to_gamma(theme::TEXT, hover)
        .lerp_to_gamma(theme::ACCENT_SOFT, active);
    icons::paint(painter, icon_rect, icon, color);
    focus_ring(ui, &response, tile, 6.0);
    if expansion > 0.05 {
        let galley = painter.layout_job(job(
            label,
            Font::Strong,
            14.0,
            color.gamma_multiply(expansion),
        ));
        painter.galley(
            pos2(rect.left() + 62.0, rect.center().y - galley.size().y / 2.0),
            galley,
            color,
        );
    }
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    if expansion < 0.5 {
        response.on_hover_text(label)
    } else {
        response
    }
}

pub fn toggle(ui: &mut Ui, value: &mut bool, label: &str) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(vec2(46.0, 24.0), Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    let state = *value;
    response.widget_info(|| WidgetInfo::selected(Role::Switch, true, state, label));
    focus_ring(ui, &response, rect, 6.0);
    let on = animate(ui.ctx(), response.id.with("on"), *value, theme::NORMAL);
    let painter = ui.painter();
    let track = theme::SURFACE_3.lerp_to_gamma(theme::ACCENT_DEEP, on);
    shape::fill(
        painter,
        rect,
        6.0,
        track,
        Stroke::new(1.0, theme::LINE_STRONG.lerp_to_gamma(theme::ACCENT, on)),
    );
    let knob_x = egui::lerp((rect.left() + 13.0)..=(rect.right() - 13.0), on);
    let knob = Rect::from_center_size(pos2(knob_x, rect.center().y), vec2(18.0, 16.0));
    shape::fill(
        painter,
        knob,
        4.0,
        theme::TEXT_MUTED.lerp_to_gamma(theme::ACCENT_SOFT, on),
        Stroke::NONE,
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}

pub fn segmented<T: Copy + PartialEq>(
    ui: &mut Ui,
    value: &mut T,
    options: &[(T, &str)],
) -> Response {
    let galleys: Vec<_> = options
        .iter()
        .map(|(_, label)| galley(ui, label, Font::Label, 11.0, theme::TEXT))
        .collect();
    let widths: Vec<f32> = galleys
        .iter()
        .map(|galley| galley.size().x + 28.0)
        .collect();
    let total: f32 = widths.iter().sum();
    let (rect, mut response) = ui.allocate_exact_size(vec2(total, 34.0), Sense::hover());
    let painter = ui.painter();
    shape::fill(
        painter,
        rect,
        6.0,
        theme::SURFACE_0,
        Stroke::new(1.0, theme::LINE),
    );
    let mut left = rect.left();
    for (index, ((option, label), width)) in options.iter().zip(&widths).enumerate() {
        let cell = Rect::from_min_size(pos2(left, rect.top()), vec2(*width, rect.height()));
        left += width;
        let selected_now = *option == *value;
        let cell_response = ui
            .interact(cell, response.id.with(index), Sense::click())
            .on_hover_cursor(CursorIcon::PointingHand);
        cell_response
            .widget_info(|| WidgetInfo::selected(Role::RadioButton, true, selected_now, *label));
        if cell_response.clicked() && *option != *value {
            *value = *option;
            response.mark_changed();
        }
        let selected = *option == *value;
        let hover = animate(
            ui.ctx(),
            cell_response.id.with("hover"),
            cell_response.hovered(),
            theme::FAST,
        );
        if selected {
            shape::fill(
                ui.painter(),
                cell.shrink(3.0),
                4.0,
                theme::ACCENT_DEEP,
                Stroke::new(1.0, theme::ACCENT),
            );
        } else if hover > 0.0 {
            shape::fill(
                ui.painter(),
                cell.shrink(3.0),
                4.0,
                theme::SURFACE_2.gamma_multiply(hover),
                Stroke::NONE,
            );
        }
        let color = if selected {
            theme::TEXT
        } else {
            theme::TEXT_MUTED
        };
        ui.painter().text(
            cell.center(),
            Align2::CENTER_CENTER,
            label.to_uppercase(),
            Font::Label.id(11.0),
            color,
        );
    }
    response
}

pub fn chip(ui: &mut Ui, label: &str, color: Color32) -> Response {
    let galley = galley(ui, label, Font::Label, 10.0, color);
    let (rect, response) = ui.allocate_exact_size(galley.size() + vec2(18.0, 10.0), Sense::hover());
    shape::fill(
        ui.painter(),
        rect,
        4.0,
        color.gamma_multiply(0.12),
        Stroke::new(1.0, color.gamma_multiply(0.6)),
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, color);
    response
}

pub fn empty_state(ui: &mut Ui, icon: Icon, title: &str, body: &str) {
    ui.add_space(theme::XXL);
    ui.vertical_centered(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(64.0), Sense::hover());
        shape::fill(
            ui.painter(),
            rect,
            14.0,
            theme::SURFACE_1,
            Stroke::new(1.0, theme::LINE),
        );
        icons::paint(ui.painter(), rect.shrink(18.0), icon, theme::ACCENT_SOFT);
        ui.add_space(theme::M);
        text(ui, title, Font::Heading, 20.0, theme::TEXT);
        ui.add_space(theme::XS);
        ui.set_max_width(460.0);
        let mut job = job(body, Font::Body, 14.0, theme::TEXT_MUTED);
        job.wrap.max_width = 460.0;
        job.halign = egui::Align::Center;
        ui.label(job);
    });
}

pub fn panel<R>(ui: &mut Ui, cut: f32, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let background = ui.painter().add(Shape::Noop);
    let inner = egui::Frame::new()
        .inner_margin(egui::Margin::same(20))
        .show(ui, add_contents);
    let rect = inner.response.rect;
    ui.painter().set(
        background,
        Shape::convex_polygon(
            shape::normform(rect, cut),
            theme::GLASS,
            Stroke::new(1.0, theme::LINE),
        ),
    );
    inner.inner
}

pub fn heading(ui: &mut Ui, title: &str, subtitle: Option<&str>) {
    text(ui, title, Font::Display, 30.0, theme::TEXT);
    if let Some(subtitle) = subtitle {
        paragraph(ui, subtitle, theme::TEXT_MUTED);
    }
    ui.add_space(theme::M);
}

pub fn setting(ui: &mut Ui, title: &str, hint: Option<&str>, control: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.set_min_height(44.0);
        ui.vertical(|ui| {
            ui.set_max_width((ui.available_width() - 260.0).max(200.0));
            text(ui, title, Font::Strong, 15.0, theme::TEXT);
            if let Some(hint) = hint {
                let mut job = job(hint, Font::Body, 13.0, theme::TEXT_FAINT);
                job.wrap.max_width = ui.available_width();
                ui.label(job);
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), control);
    });
    let y = ui.cursor().top() + 2.0;
    let x = ui.max_rect().x_range();
    ui.painter()
        .hline(x, y, Stroke::new(1.0, theme::LINE.gamma_multiply(0.6)));
    ui.add_space(theme::S);
}

pub fn progress_bar(ui: &mut Ui, fraction: Option<f32>) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 6.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, theme::SURFACE_0);
    let fraction = fraction.unwrap_or_else(|| {
        let time = crate::shape::seconds(ui.input(|input| input.time));
        ui.ctx().request_repaint();
        0.15 + 0.1 * (time * 3.0).sin()
    });
    let filled = Rect::from_min_size(
        rect.min,
        vec2(rect.width() * fraction.clamp(0.0, 1.0), rect.height()),
    );
    painter.add(Shape::convex_polygon(
        shape::rounded_top_right(filled, 3.0),
        theme::ACCENT,
        Stroke::NONE,
    ));
    response
}

pub fn progress_button(ui: &mut Ui, label: &str, fraction: Option<f32>) -> Response {
    let galley = galley(ui, label, Font::Button, 15.0, theme::TEXT);
    let desired = vec2((galley.size().x + 60.0).max(220.0), 48.0);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::hover());
    let painter = ui.painter();
    shape::fill(
        painter,
        rect,
        10.0,
        theme::SURFACE_2,
        Stroke::new(1.0, theme::ACCENT_DEEP),
    );
    let fraction = fraction.unwrap_or_else(|| {
        ui.ctx().request_repaint();
        let time = crate::shape::seconds(ui.input(|input| input.time));
        0.5 + 0.5 * (time * 2.0).sin()
    });
    let clip = Rect::from_min_size(
        rect.min,
        vec2(rect.width() * fraction.clamp(0.0, 1.0), rect.height()),
    );
    painter.with_clip_rect(clip).add(Shape::convex_polygon(
        shape::normform(rect, 10.0),
        theme::ACCENT_DEEP,
        Stroke::NONE,
    ));
    painter.galley(rect.center() - galley.size() / 2.0, galley, theme::TEXT);
    response
}

pub(crate) fn focus_ring(ui: &Ui, response: &Response, rect: Rect, cut: f32) {
    if response.has_focus() {
        ui.painter().add(Shape::closed_line(
            shape::normform(rect.expand(3.0), cut + 1.0),
            Stroke::new(2.0, theme::ACCENT_SOFT),
        ));
    }
}
