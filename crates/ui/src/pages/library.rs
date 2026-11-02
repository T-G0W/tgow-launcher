use egui::{Align2, Rect, Response, Role, Sense, Stroke, Ui, Vec2, WidgetInfo, pos2, vec2};
use tgow_core::catalog::{App, ReleaseState};

use super::{accent_of, initial, paint_chip};
use crate::icons::Icon;
use crate::shell::{CatalogStatus, Effect, Page, Phase, Shell, View, padded};
use crate::text::Text;
use crate::theme::{self, Font};
use crate::widgets::{self, Emphasis, animate};
use crate::{icons, shape};

const CARD_MIN_WIDTH: f32 = 300.0;
const CARD_HEIGHT: f32 = 214.0;
const GAP: f32 = 20.0;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Filter {
    #[default]
    All,
    Installed,
    Updates,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Order {
    #[default]
    Name,
    Newest,
}

#[derive(Debug, Default)]
pub(crate) struct State {
    pub(crate) search: String,
    pub(crate) filter: Filter,
    pub(crate) order: Order,
    pub(crate) focus_search: bool,
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
                search_box(ui, &mut shell.library, view);
            });
        });
        ui.add_space(theme::M);
        ui.horizontal(|ui| {
            if !discover {
                widgets::segmented(
                    ui,
                    &mut shell.library.filter,
                    &[
                        (Filter::All, view.t(Text::FilterAll)),
                        (Filter::Installed, view.t(Text::Installed)),
                        (Filter::Updates, view.t(Text::SettingsUpdates)),
                    ],
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                widgets::segmented(
                    ui,
                    &mut shell.library.order,
                    &[
                        (Order::Newest, view.t(Text::SortNewest)),
                        (Order::Name, view.t(Text::SortName)),
                    ],
                );
            });
        });
        ui.add_space(theme::M);
        let filter = if discover {
            Filter::All
        } else {
            shell.library.filter
        };
        let needle = shell.library.search.trim().to_lowercase();
        let mut apps: Vec<&App> = view
            .apps
            .iter()
            .filter(|app| matches_filter(app, view, filter))
            .filter(|app| {
                needle.is_empty()
                    || app.title().to_lowercase().contains(&needle)
                    || app
                        .description
                        .as_deref()
                        .is_some_and(|text| text.to_lowercase().contains(&needle))
            })
            .collect();
        sort(&mut apps, shell.library.order);
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

fn sort(apps: &mut [&App], order: Order) {
    match order {
        Order::Name => apps.sort_by_cached_key(|app| app.title().to_lowercase()),
        Order::Newest => apps.sort_by(|left, right| {
            published(right).cmp(&published(left)).then_with(|| {
                left.title()
                    .to_lowercase()
                    .cmp(&right.title().to_lowercase())
            })
        }),
    }
}

fn published(app: &App) -> Option<&str> {
    app.manifest()
        .and_then(|manifest| manifest.published.as_deref())
}

fn matches_filter(app: &App, view: &View<'_>, filter: Filter) -> bool {
    let installation = view.installed.get(&app.id);
    match filter {
        Filter::All => true,
        Filter::Installed => installation.is_some(),
        Filter::Updates => installation
            .zip(app.manifest())
            .is_some_and(|(installation, manifest)| installation.is_outdated(manifest)),
    }
}

fn search_box(ui: &mut Ui, state: &mut State, view: &View<'_>) {
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
    let response = ui.put(
        field,
        egui::TextEdit::singleline(&mut state.search)
            .hint_text(view.t(Text::Search))
            .frame(egui::Frame::NONE)
            .font(Font::Body.id(14.0))
            .align(egui::Align2::LEFT_CENTER),
    );
    if state.focus_search {
        state.focus_search = false;
        response.request_focus();
    }
}

fn card(ui: &mut Ui, app: &App, view: &View<'_>, width: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, CARD_HEIGHT), Sense::click());
    let (label, color) = status(app, view);
    response.widget_info(|| {
        WidgetInfo::labeled(Role::Button, true, format!("{}, {label}", app.title()))
    });
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
    let artwork = view.artwork.get(&app.id);
    if let Some(hero) = artwork.and_then(|artwork| artwork.hero.as_ref()) {
        let uv = shape::cover_uv(hero.size_vec2(), band.size());
        painter.add(egui::Shape::mesh(shape::textured_polygon(
            &shape::normform(band, 12.0),
            band,
            uv,
            hero.id(),
            0.55 + 0.3 * hover,
            0.0,
        )));
    }
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
    paint_icon(painter, app, artwork, tile, accent);
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
    paint_chip(
        painter,
        pos2(rect.left() + 20.0, rect.bottom() - 40.0),
        &label,
        color,
    );
    widgets::focus_ring(ui, &response, rect, 12.0);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn paint_icon(
    painter: &egui::Painter,
    app: &App,
    artwork: Option<&crate::shell::Artwork>,
    tile: Rect,
    accent: egui::Color32,
) {
    shape::fill(
        painter,
        tile,
        10.0,
        theme::SURFACE_2,
        Stroke::new(1.0, accent.gamma_multiply(0.7)),
    );
    if let Some(icon) = artwork.and_then(|artwork| artwork.icon.as_ref()) {
        painter.image(
            icon.id(),
            tile.shrink(7.0),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    } else {
        painter.text(
            tile.center(),
            Align2::CENTER_CENTER,
            initial(app.title()),
            Font::Display.id(28.0),
            accent,
        );
    }
}

pub(crate) fn status(app: &App, view: &View<'_>) -> (String, egui::Color32) {
    if let Some(phase) = view.jobs.get(&app.id) {
        return (phase_label(view, *phase), theme::ACCENT_SOFT);
    }
    if view.failures.contains_key(&app.id) {
        return (view.t(Text::Failed).to_owned(), theme::DANGER);
    }
    if let Some(installation) = view.installed.get(&app.id) {
        let outdated = app
            .manifest()
            .is_some_and(|manifest| installation.is_outdated(manifest));
        return if outdated {
            (view.t(Text::UpdateAvailable).to_owned(), theme::WARNING)
        } else {
            (view.t(Text::Installed).to_owned(), theme::POSITIVE)
        };
    }
    let (text, color) = match app.release {
        ReleaseState::Verified(_) => (Text::Signed, theme::ACCENT_SOFT),
        ReleaseState::Unknown => (Text::CatalogLoading, theme::TEXT_FAINT),
        ReleaseState::NoRelease => (Text::NoRelease, theme::TEXT_FAINT),
        ReleaseState::Unsigned => (Text::NotSigned, theme::WARNING),
        ReleaseState::Rejected(_) => (Text::SignatureRejected, theme::DANGER),
    };
    (view.t(text).to_owned(), color)
}

pub(crate) fn phase_label(view: &View<'_>, phase: Phase) -> String {
    match phase {
        Phase::Waiting => view.t(Text::PhaseWaiting).to_owned(),
        Phase::Downloading { .. } => {
            let percent = phase
                .fraction()
                .map_or(0.0, |fraction| (fraction * 100.0).floor());
            format!("{} {percent} %", view.t(Text::PhaseDownloading))
        }
        Phase::Verifying => view.t(Text::PhaseVerifying).to_owned(),
        Phase::Installing => view.t(Text::PhaseInstalling).to_owned(),
        Phase::Removing => view.t(Text::PhaseRemoving).to_owned(),
    }
}
