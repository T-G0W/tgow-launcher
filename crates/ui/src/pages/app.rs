use egui::{Align2, Rect, Sense, Shape, Stroke, Ui, pos2, vec2};
use tgow_core::catalog::{App, ReleaseState};
use tgow_core::manifest::Manifest;
use tgow_core::select::{self, Choice, Preference, Reason, Rejection};
use tgow_core::settings::Settings;

use super::{accent_of, initial, library::status, paint_chip};
use crate::shell::{Effect, Page, Shell, View};
use crate::text::Text;
use crate::theme::{self, Font};
use crate::widgets::{self, Emphasis};
use crate::{brand, shape};

const HERO: f32 = 340.0;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    #[default]
    Overview,
    Notes,
}

#[derive(Debug, Default)]
pub(crate) struct State {
    pub(crate) tab: Tab,
}

pub(crate) fn show(
    shell: &mut Shell,
    ui: &mut Ui,
    view: &View<'_>,
    id: &str,
    settings: &Settings,
    effects: &mut Vec<Effect>,
) {
    let Some(app) = view.app(id) else {
        shell.page = Page::Library;
        return;
    };
    let choice = app.manifest().map(|manifest| {
        select::choose(
            &manifest.assets,
            view.platform,
            Preference {
                system_packages: settings.system_packages,
                ..Preference::default()
            },
        )
    });
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            hero(ui, app, view, choice.as_ref(), effects);
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(40, 8))
                .show(ui, |ui| {
                    ui.set_max_width(ui.available_width().min(1180.0));
                    widgets::segmented(
                        ui,
                        &mut shell.detail.tab,
                        &[
                            (Tab::Overview, view.t(Text::Overview)),
                            (Tab::Notes, view.t(Text::ReleaseNotes)),
                        ],
                    );
                    ui.add_space(theme::M);
                    widgets::panel(ui, 16.0, |ui| {
                        ui.set_width(ui.available_width());
                        match shell.detail.tab {
                            Tab::Overview => overview(ui, app, view, choice.as_ref()),
                            Tab::Notes => notes(ui, app, view),
                        }
                    });
                    ui.add_space(theme::XL);
                });
        });
}

fn hero(
    ui: &mut Ui,
    app: &App,
    view: &View<'_>,
    choice: Option<&Result<Choice<'_>, Rejection>>,
    effects: &mut Vec<Effect>,
) {
    let accent = accent_of(app);
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, HERO), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.add(Shape::mesh(shape::rect_gradient(
        rect,
        accent.gamma_multiply(0.20),
        theme::BG_DEEP.gamma_multiply(0.0),
        shape::Direction::Right,
    )));
    let mark = Rect::from_center_size(
        pos2(rect.right() - HERO * 0.55, rect.center().y + 10.0),
        vec2(HERO * 1.1, HERO * 1.1),
    );
    painter.add(Shape::convex_polygon(
        shape::normform(mark, mark.width() * brand::CHAMFER),
        accent.gamma_multiply(0.05),
        Stroke::new(1.5, accent.gamma_multiply(0.25)),
    ));
    painter.text(
        mark.center(),
        Align2::CENTER_CENTER,
        initial(app.title()),
        Font::Display.id(170.0),
        accent.gamma_multiply(0.18),
    );
    painter.add(Shape::mesh(shape::rect_gradient(
        Rect::from_min_max(pos2(rect.left(), rect.bottom() - 120.0), rect.max),
        theme::BG_DEEP.gamma_multiply(0.0),
        theme::BG.gamma_multiply(0.85),
        shape::Direction::Down,
    )));
    let mut inner = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(vec2(40.0, 36.0)))
            .layout(egui::Layout::bottom_up(egui::Align::Min)),
    );
    inner.horizontal(|ui| {
        action(ui, app, view, choice, effects);
        if widgets::button(ui, view.t(Text::OpenOnGitHub), Emphasis::Secondary, true).clicked() {
            effects.push(Effect::Open(app.html_url.clone()));
        }
    });
    inner.add_space(theme::M);
    inner.horizontal(|ui| meta(ui, app, view, choice));
    inner.add_space(theme::S);
    if let Some(tagline) = app
        .manifest()
        .and_then(|manifest| manifest.tagline.get(view.language))
        .or(app.description.as_deref())
    {
        widgets::text(&mut inner, tagline, Font::Body, 17.0, theme::TEXT_MUTED);
    }
    widgets::text(&mut inner, app.title(), Font::Display, 54.0, theme::TEXT);
}

fn action(
    ui: &mut Ui,
    app: &App,
    view: &View<'_>,
    choice: Option<&Result<Choice<'_>, Rejection>>,
    effects: &mut Vec<Effect>,
) {
    let (label, enabled) = match (&app.release, choice) {
        (ReleaseState::Verified(_), Some(Ok(_))) => (Text::Install, true),
        (ReleaseState::Verified(_), _) => (Text::NotForSystem, false),
        (ReleaseState::Unsigned, _) => (Text::NotSigned, false),
        (ReleaseState::Rejected(_), _) => (Text::SignatureRejected, false),
        (ReleaseState::NoRelease, _) => (Text::NoRelease, false),
        (ReleaseState::Unknown, _) => (Text::CatalogLoading, false),
    };
    if widgets::button(ui, view.t(label), Emphasis::Primary, enabled).clicked() {
        effects.push(Effect::Install(app.id.clone()));
    }
}

fn meta(ui: &mut Ui, app: &App, view: &View<'_>, choice: Option<&Result<Choice<'_>, Rejection>>) {
    let (label, color) = status(app);
    widgets::chip(ui, view.t(label), color);
    if let Some(manifest) = app.manifest() {
        widgets::chip(ui, &format!("v{}", manifest.version), theme::ACCENT_SOFT);
    }
    if let Some(Ok(choice)) = choice {
        let asset = choice.asset;
        widgets::chip(
            ui,
            &format!("{} {}", asset.os.label(), asset.arch.label()),
            theme::TEXT_MUTED,
        );
        widgets::chip(ui, asset.kind.label(), theme::TEXT_MUTED);
        widgets::chip(ui, &megabytes(asset.size), theme::TEXT_MUTED);
    }
}

fn overview(
    ui: &mut Ui,
    app: &App,
    view: &View<'_>,
    choice: Option<&Result<Choice<'_>, Rejection>>,
) {
    if let Some(description) = &app.description {
        widgets::paragraph(ui, description, theme::TEXT);
        ui.add_space(theme::M);
    }
    let rows: Vec<(Text, String)> = [
        Some((Text::DetectedSystem, view.platform.to_string())),
        app.manifest()
            .map(|manifest| (Text::Version, manifest.version.to_string())),
        choice
            .and_then(|choice| choice.as_ref().ok())
            .map(|choice| (Text::Package, describe(choice, view))),
        choice
            .and_then(|choice| choice.as_ref().ok())
            .map(|choice| (Text::Size, megabytes(choice.asset.size))),
    ]
    .into_iter()
    .flatten()
    .collect();
    egui::Grid::new("facts")
        .num_columns(2)
        .spacing(vec2(28.0, 10.0))
        .show(ui, |ui| {
            for (label, value) in rows {
                widgets::text(ui, view.t(label), Font::Label, 11.0, theme::TEXT_FAINT);
                widgets::text(ui, &value, Font::Mono, 13.0, theme::TEXT);
                ui.end_row();
            }
        });
    if let Some(Err(Rejection::Unsupported { offered })) = choice {
        ui.add_space(theme::M);
        let offered: Vec<String> = offered
            .iter()
            .map(|(os, arch)| format!("{} {}", os.label(), arch.label()))
            .collect();
        widgets::paragraph(
            ui,
            &format!("{} · {}", view.t(Text::NotForSystem), offered.join(", ")),
            theme::WARNING,
        );
    }
}

fn describe(choice: &Choice<'_>, view: &View<'_>) -> String {
    let asset = choice.asset;
    let mut text = format!("{} · {}", asset.kind.label(), asset.file);
    if choice.emulated {
        text.push_str(" · ");
        text.push_str(view.t(Text::Emulated));
    }
    if choice.reason == Reason::KeepsInstalledKind {
        text.push_str(" · ✓");
    }
    text
}

fn notes(ui: &mut Ui, app: &App, view: &View<'_>) {
    let Some(text) = app
        .manifest()
        .and_then(|manifest: &Manifest| manifest.notes.get(view.language))
    else {
        widgets::paragraph(ui, view.t(Text::NoRelease), theme::TEXT_FAINT);
        return;
    };
    for line in text.lines() {
        let line = line.trim_end();
        let clean = line.replace("**", "");
        if let Some(heading) = clean
            .strip_prefix("## ")
            .or_else(|| clean.strip_prefix("# "))
        {
            ui.add_space(theme::S);
            widgets::text(ui, heading, Font::Heading, 17.0, theme::TEXT);
        } else if let Some(item) = clean
            .strip_prefix("- ")
            .or_else(|| clean.strip_prefix("* "))
        {
            ui.horizontal_top(|ui| {
                let (dot, _) = ui.allocate_exact_size(vec2(14.0, 20.0), Sense::hover());
                shape::fill(
                    ui.painter(),
                    dot.shrink2(vec2(3.0, 7.0)),
                    1.5,
                    theme::ACCENT,
                    Stroke::NONE,
                );
                widgets::paragraph(ui, item, theme::TEXT_MUTED);
            });
        } else if clean.is_empty() {
            ui.add_space(theme::XS);
        } else {
            widgets::paragraph(ui, &clean, theme::TEXT_MUTED);
        }
    }
    let _ = paint_chip;
}

fn megabytes(bytes: u64) -> String {
    let tenths = bytes.saturating_mul(10) / (1024 * 1024);
    format!("{},{} MB", tenths / 10, tenths % 10)
}
