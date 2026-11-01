use egui::{Align2, Rect, Response, Sense, Stroke, Ui, Vec2, pos2, vec2};
use tgow_core::catalog::{App, ReleaseState};

use super::{accent_of, initial, paint_chip};
use crate::icons::Icon;
use crate::shell::{CatalogStatus, Effect, Page, Shell, View, padded};
use crate::text::Text;
use crate::theme::{self, Font};
use crate::widgets::{self, Emphasis, animate};
use crate::{icons, shape};

const CARD_MIN_WIDTH: f32 = 300.0;
const CARD_HEIGHT: f32 = 214.0;
const GAP: f32 = 20.0;

#[derive(Debug, Default)]
pub(crate) struct State {
    pub(crate) search: String,
}

pub(crate) fn show(
    shell: &mut Shell,
    ui: &mut Ui,
    view: &View<'_>,
    discover: bool,
    effects: &mut Vec<Effect>,
) {
    padded(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                let title = if discover {
                    Text::Discover
                } else {
                    Text::Library
                };
                widgets::text(ui, view.t(title), Font::Display, 30.0, theme::TEXT);
                if discover {
                    widgets::paragraph(ui, view.t(Text::DiscoverBody), theme::TEXT_MUTED);
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let busy = view.status == CatalogStatus::Loading;
                if widgets::button(ui, view.t(Text::CheckNow), Emphasis::Secondary, !busy).clicked()
                {
                    effects.push(Effect::CheckNow);
                }
                ui.add_space(theme::S);
                search_box(ui, &mut shell.library.search, view);
            });
        });
        ui.add_space(theme::L);
        let needle = shell.library.search.trim().to_lowercase();
        let apps: Vec<&App> = view
            .apps
            .iter()
            .filter(|app| {
                needle.is_empty()
                    || app.title().to_lowercase().contains(&needle)
                    || app
                        .description
                        .as_deref()
                        .is_some_and(|text| text.to_lowercase().contains(&needle))
            })
            .collect();
        if apps.is_empty() {
            let (title, body) = if view.status == CatalogStatus::Loading {
                (Text::CatalogLoading, Text::CatalogEmptyBody)
            } else {
                (Text::CatalogEmptyTitle, Text::CatalogEmptyBody)
            };
            widgets::empty_state(ui, Icon::Library, view.t(title), view.t(body));
            return;
        }
        let available = ui.available_width();
        let columns = ((available + GAP) / (CARD_MIN_WIDTH + GAP))
            .floor()
            .max(1.0);
        let width = (available - GAP * (columns - 1.0)) / columns;
        let per_row = shape::to_usize(columns);
        for row in apps.chunks(per_row) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP;
                for app in row {
                    if card(ui, app, view, width).clicked() {
                        shell.page = Page::App(app.id.clone());
                    }
                }
            });
            ui.add_space(GAP - ui.spacing().item_spacing.y);
        }
    });
}

fn search_box(ui: &mut Ui, search: &mut String, view: &View<'_>) {
    let (rect, _) = ui.allocate_exact_size(vec2(260.0, 36.0), Sense::hover());
    shape::fill(
        ui.painter(),
        rect,
        6.0,
        theme::SURFACE_0,
        Stroke::new(1.0, theme::LINE),
    );
    icons::paint(
        ui.painter(),
        Rect::from_center_size(pos2(rect.left() + 18.0, rect.center().y), Vec2::splat(16.0)),
        Icon::Search,
        theme::TEXT_FAINT,
    );
    let field = Rect::from_min_max(
        pos2(rect.left() + 34.0, rect.top() + 4.0),
        pos2(rect.right() - 8.0, rect.bottom() - 4.0),
    );
    ui.put(
        field,
        egui::TextEdit::singleline(search)
            .hint_text(view.t(Text::Search))
            .frame(egui::Frame::NONE)
            .font(Font::Body.id(14.0))
            .align(egui::Align2::LEFT_CENTER),
    );
}

fn card(ui: &mut Ui, app: &App, view: &View<'_>, width: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, CARD_HEIGHT), Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let hover = animate(
        ui.ctx(),
        response.id.with("hover"),
        response.hovered(),
        theme::FAST,
    );
    let accent = accent_of(app);
    let painter = ui.painter();
    if hover > 0.0 {
        shape::glow(painter, rect, 12.0, accent.gamma_multiply(0.18 * hover), 28);
    }
    let outline = shape::normform(rect, 12.0);
    painter.add(egui::Shape::convex_polygon(
        outline.clone(),
        theme::GLASS,
        Stroke::NONE,
    ));
    let band = Rect::from_min_size(rect.min, vec2(rect.width(), 86.0));
    painter.add(egui::Shape::mesh(shape::vertical_gradient(
        &shape::normform(band, 12.0)[..],
        accent.gamma_multiply(0.22 + 0.12 * hover),
        accent.gamma_multiply(0.0),
    )));
    painter.add(egui::Shape::closed_line(
        outline,
        Stroke::new(1.0, theme::LINE.lerp_to_gamma(accent, hover * 0.8)),
    ));
    let tile = Rect::from_min_size(rect.min + vec2(20.0, 22.0), Vec2::splat(54.0));
    shape::fill(
        painter,
        tile,
        10.0,
        theme::SURFACE_2,
        Stroke::new(1.0, accent.gamma_multiply(0.7)),
    );
    painter.text(
        tile.center(),
        Align2::CENTER_CENTER,
        initial(app.title()),
        Font::Display.id(28.0),
        accent,
    );
    let name = painter.layout_job(widgets::job(app.title(), Font::Heading, 20.0, theme::TEXT));
    painter.galley(
        pos2(tile.right() + 16.0, tile.top() + 4.0),
        name,
        theme::TEXT,
    );
    let version = app.manifest().map_or_else(
        || app.tag.clone().unwrap_or_default(),
        |manifest| format!("v{}", manifest.version),
    );
    painter.text(
        pos2(tile.right() + 16.0, tile.top() + 34.0),
        Align2::LEFT_TOP,
        version,
        Font::Mono.id(12.0),
        theme::TEXT_FAINT,
    );
    let description = app
        .manifest()
        .and_then(|manifest| manifest.tagline.get(view.language))
        .or(app.description.as_deref())
        .unwrap_or_default();
    let mut job = widgets::job(description, Font::Body, 13.5, theme::TEXT_MUTED);
    job.wrap.max_width = rect.width() - 40.0;
    job.wrap.max_rows = 2;
    job.wrap.overflow_character = Some('…');
    painter.galley(
        pos2(rect.left() + 20.0, tile.bottom() + 18.0),
        painter.layout_job(job),
        theme::TEXT_MUTED,
    );
    let (label, color) = status(app);
    paint_chip(
        painter,
        pos2(rect.left() + 20.0, rect.bottom() - 40.0),
        view.t(label),
        color,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(crate) fn status(app: &App) -> (Text, egui::Color32) {
    match app.release {
        ReleaseState::Verified(_) => (Text::Signed, theme::POSITIVE),
        ReleaseState::Unknown => (Text::CatalogLoading, theme::TEXT_FAINT),
        ReleaseState::NoRelease => (Text::NoRelease, theme::TEXT_FAINT),
        ReleaseState::Unsigned => (Text::NotSigned, theme::WARNING),
        ReleaseState::Rejected(_) => (Text::SignatureRejected, theme::DANGER),
    }
}
