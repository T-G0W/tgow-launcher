use egui::{Align2, Rect, Sense, Stroke, Ui, Vec2, pos2};
use tgow_core::manifest::parse_version;

use crate::icons::Icon;
use crate::pages::{accent_of, initial, library::phase_label};
use crate::shell::{CatalogStatus, Effect, Finished, Outcome, Page, Phase, View};
use crate::text::Text;
use crate::theme::{self, Font};
use crate::widgets::{self, Emphasis};
use crate::{brand, shape};

pub(crate) const WIDTH: f32 = 380.0;
const TILE: f32 = 34.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Popover {
    Downloads,
    Status,
}

pub(crate) fn downloads(ui: &mut Ui, view: &View<'_>, effects: &mut Vec<Effect>) -> Option<Page> {
    let mut open = None;
    header(ui, view.t(Text::Downloads), |ui| {
        if widgets::link(ui, view.t(Text::ShowAll)).clicked() {
            open = Some(Page::Downloads);
        }
    });
    let ongoing: Vec<(&String, &Phase)> = view
        .jobs
        .iter()
        .filter(|(_, phase)| !matches!(phase, Phase::Waiting))
        .collect();
    let waiting: Vec<(&String, &Phase)> = view
        .jobs
        .iter()
        .filter(|(_, phase)| matches!(phase, Phase::Waiting))
        .collect();
    if ongoing.is_empty() && waiting.is_empty() && view.recent.is_empty() {
        ui.add_space(theme::S);
        widgets::text(
            ui,
            view.t(Text::DownloadsEmptyTitle),
            Font::Body,
            14.0,
            theme::TEXT_MUTED,
        );
        return open;
    }
    for (title, jobs) in [(Text::Ongoing, &ongoing), (Text::UpNext, &waiting)] {
        if jobs.is_empty() {
            continue;
        }
        section(ui, view.t(title));
        for (app, phase) in jobs {
            job(ui, view, app, **phase, effects);
        }
    }
    if !view.recent.is_empty() {
        section(ui, view.t(Text::FinishedSection));
        for finished in view.recent {
            done(ui, view, finished);
        }
    }
    open
}

pub(crate) fn status(ui: &mut Ui, view: &View<'_>, effects: &mut Vec<Effect>) {
    header(ui, view.t(Text::Status), |_| {});
    let (state, color) = match &view.status {
        CatalogStatus::Ready => (view.t(Text::Operational).to_owned(), theme::POSITIVE),
        CatalogStatus::Loading => (view.t(Text::Checking).to_owned(), theme::ACCENT_SOFT),
        CatalogStatus::Offline => (view.t(Text::Unreachable).to_owned(), theme::DANGER),
        CatalogStatus::Limited { minutes } => (
            format!("{} · {minutes} min", view.t(Text::Limited)),
            theme::WARNING,
        ),
    };
    row(ui, "GitHub", |ui| {
        widgets::chip(ui, &state, color);
    });
    row(ui, view.t(Text::Catalog), |ui| {
        widgets::text(
            ui,
            &match view.apps.len() {
                1 => "1 App".to_owned(),
                count => format!("{count} Apps"),
            },
            Font::Mono,
            13.0,
            theme::TEXT,
        );
    });
    row(ui, view.t(Text::Product), |ui| {
        let newer = view
            .launcher_latest
            .and_then(parse_version)
            .zip(parse_version(view.version))
            .is_some_and(|(latest, current)| latest > current);
        if newer {
            widgets::chip(ui, view.t(Text::UpdateAvailable), theme::WARNING);
        } else {
            widgets::chip(ui, view.t(Text::UpToDate), theme::POSITIVE);
        }
        widgets::text(
            ui,
            &format!("v{}", view.version),
            Font::Mono,
            13.0,
            theme::TEXT_MUTED,
        );
    });
    row(ui, view.t(Text::LastChecked), |ui| {
        widgets::text(
            ui,
            view.checked.unwrap_or(view.t(Text::Never)),
            Font::Mono,
            13.0,
            theme::TEXT,
        );
    });
    ui.add_space(theme::M);
    let busy = view.status == CatalogStatus::Loading;
    if widgets::button(ui, view.t(Text::CheckNow), Emphasis::Secondary, !busy).clicked() {
        effects.push(Effect::CheckNow);
    }
}

fn header(ui: &mut Ui, title: &str, trailing: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        widgets::text(ui, &title.to_uppercase(), Font::Button, 14.0, theme::TEXT);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), trailing);
    });
    ui.add_space(theme::XS);
}

fn section(ui: &mut Ui, title: &str) {
    ui.add_space(theme::S);
    widgets::text(
        ui,
        &title.to_uppercase(),
        Font::Label,
        10.5,
        theme::TEXT_FAINT,
    );
    let y = ui.cursor().top();
    ui.painter().hline(
        ui.max_rect().x_range(),
        y,
        Stroke::new(1.0, theme::LINE.gamma_multiply(0.7)),
    );
    ui.add_space(theme::S);
}

fn row(ui: &mut Ui, label: &str, value: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.set_min_height(30.0);
        widgets::text(ui, label, Font::Body, 14.0, theme::TEXT_MUTED);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), value);
    });
}

fn job(ui: &mut Ui, view: &View<'_>, app: &str, phase: Phase, effects: &mut Vec<Effect>) {
    let title = view.app(app).map_or(app, |found| found.title());
    ui.horizontal(|ui| {
        tile(ui, view, app, title);
        ui.vertical(|ui| {
            widgets::text(ui, title, Font::Strong, 14.0, theme::TEXT);
            widgets::text(
                ui,
                &phase_label(view, phase),
                Font::Mono,
                11.5,
                theme::ACCENT_SOFT,
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let cancellable = matches!(phase, Phase::Waiting | Phase::Downloading { .. });
            if cancellable
                && widgets::icon_button(ui, Icon::Close, view.t(Text::Cancel), 26.0).clicked()
            {
                effects.push(Effect::Cancel(app.to_owned()));
            }
        });
    });
    widgets::progress_bar(ui, phase.fraction());
    ui.add_space(theme::XS);
}

fn done(ui: &mut Ui, view: &View<'_>, finished: &Finished) {
    let title = view
        .app(&finished.app)
        .map_or(view.t(Text::Product), |found| found.title());
    let (detail, color) = match &finished.outcome {
        Outcome::Installed(version) => (
            format!("{} · v{version}", view.t(Text::Installed)),
            theme::POSITIVE,
        ),
        Outcome::Removed => (view.t(Text::Removed).to_owned(), theme::TEXT_MUTED),
        Outcome::Failed => (view.t(Text::Failed).to_owned(), theme::DANGER),
    };
    ui.horizontal(|ui| {
        tile(ui, view, &finished.app, title);
        ui.vertical(|ui| {
            widgets::text(ui, title, Font::Strong, 14.0, theme::TEXT);
            widgets::text(ui, &detail, Font::Mono, 11.5, color);
        });
    });
}

fn tile(ui: &mut Ui, view: &View<'_>, app: &str, title: &str) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(TILE), Sense::hover());
    let found = view.app(app);
    let accent = found.map_or(theme::ACCENT, accent_of);
    let painter = ui.painter();
    shape::fill(
        painter,
        rect,
        theme::RADIUS,
        theme::SURFACE_2,
        Stroke::new(1.0, accent.gamma_multiply(0.6)),
    );
    if let Some(texture) = view
        .artwork
        .get(app)
        .and_then(|artwork| artwork.icon.as_ref())
    {
        painter.image(
            texture.id(),
            rect.shrink(5.0),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    } else if found.is_none() {
        brand::paint_mark(painter, rect.shrink(4.0));
    } else {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            initial(title),
            Font::Display.id(18.0),
            accent,
        );
    }
}
