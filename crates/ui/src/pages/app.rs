use egui::{Align2, Rect, Sense, Shape, Stroke, Ui, Vec2, pos2, vec2};
use tgow_core::catalog::{App, ReleaseState};
use tgow_core::manifest::Manifest;
use tgow_core::select::{self, Choice, Preference, Reason, Rejection};
use tgow_core::settings::{Language, Settings, UpdateMode};

use super::library::{phase_label, status};
use super::{MenuChoice, accent_of, app_menu, initial, paint_icon};
use crate::icons::Icon;
use crate::shell::{Effect, Page, Phase, Shell, View};
use crate::text::Text;
use crate::theme::{self, Font};
use crate::widgets::{self, Emphasis};
use crate::{brand, shape};

const HERO: f32 = 380.0;

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
    settings: &mut Settings,
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
                    widgets::panel(ui, theme::RADIUS, |ui| {
                        ui.set_width(ui.available_width());
                        match shell.detail.tab {
                            Tab::Overview => {
                                let banner = view
                                    .artwork
                                    .get(&app.id)
                                    .and_then(|artwork| artwork.hero.as_ref());
                                let total = ui.available_width();
                                ui.horizontal_top(|ui| {
                                    ui.vertical(|ui| {
                                        if banner.is_some() {
                                            ui.set_width(total * 0.5);
                                        }
                                        overview(ui, app, view, choice.as_ref(), effects);
                                        updates(ui, app, view, settings, effects);
                                    });
                                    if let Some(banner) = banner {
                                        ui.add_space(theme::L);
                                        screenshot(ui, banner);
                                    }
                                });
                            }
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
    painter.add(Shape::mesh(shape::shade(
        rect,
        accent.gamma_multiply(0.20),
        1.0,
        0.45,
    )));
    let hero_image = view
        .artwork
        .get(&app.id)
        .and_then(|artwork| artwork.hero.as_ref());
    if let Some(hero) = hero_image {
        let uv = shape::cover_uv(hero.size_vec2(), rect.size());
        painter.add(Shape::mesh(shape::faded_image(rect, uv, hero.id(), 0.45)));
        painter.rect_filled(rect, 0.0, theme::BG.gamma_multiply(0.45));
        painter.add(Shape::mesh(shape::rect_gradient(
            rect,
            theme::BG.gamma_multiply(0.96),
            theme::BG.gamma_multiply(0.15),
            shape::Direction::Right,
        )));
        painter.add(Shape::mesh(shape::shade(
            rect,
            theme::BG.gamma_multiply(0.95),
            0.55,
            0.45,
        )));
    }
    if hero_image.is_none() {
        let mark = Rect::from_center_size(
            pos2(rect.right() - HERO * 0.55, rect.center().y),
            Vec2::splat(HERO * 0.82),
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
            Font::Display.id(132.0),
            accent.gamma_multiply(0.18),
        );
    }
    let mut top = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(
        rect.min + vec2(40.0, 24.0),
        vec2(rect.width() - 80.0, 32.0),
    )));
    if widgets::ghost_button(&mut top, Icon::Back, view.t(Text::Back)).clicked() {
        effects.push(Effect::Back);
    }
    let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(
        pos2(rect.left() + 40.0, rect.top() + 84.0),
        pos2(rect.right() - 40.0, rect.bottom() - 24.0),
    )));
    inner.horizontal(|ui| {
        let (tile, _) = ui.allocate_exact_size(Vec2::splat(68.0), Sense::hover());
        paint_icon(ui.painter(), app, view.artwork.get(&app.id), tile, accent);
        ui.add_space(theme::M);
        widgets::text(ui, app.title(), Font::Display, 52.0, theme::TEXT);
    });
    inner.add_space(theme::S);
    if let Some(tagline) = app
        .manifest()
        .and_then(|manifest| manifest.tagline.get(view.language))
        .or(app.description.as_deref())
    {
        let mut job = widgets::job(tagline, Font::Body, 17.0, theme::TEXT_MUTED);
        job.wrap.max_width = (inner.available_width() * 0.62).max(320.0);
        job.wrap.max_rows = 2;
        job.wrap.overflow_character = Some('…');
        inner.label(job);
    }
    inner.add_space(theme::S);
    inner.horizontal(|ui| meta(ui, app, view, choice));
    inner.add_space(theme::M);
    inner.horizontal(|ui| {
        action(ui, app, view, choice, effects);
        let more = widgets::square_button(ui, Icon::More, view.t(Text::More), 48.0);
        egui::Popup::menu(&more).show(|ui| {
            if let Some(MenuChoice::Do(effect)) = app_menu(ui, app, view, false) {
                effects.push(effect);
            }
        });
    });
}

fn screenshot(ui: &mut Ui, banner: &egui::TextureHandle) {
    let width = ui.available_width();
    let size = banner.size_vec2();
    let height = (width * size.y / size.x.max(1.0)).clamp(120.0, 380.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let uv = shape::cover_uv(size, rect.size());
    ui.painter().add(Shape::mesh(shape::textured_polygon(
        &shape::rounded(rect, theme::RADIUS),
        rect,
        uv,
        banner.id(),
        1.0,
        1.0,
    )));
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(shape::to_u8(theme::RADIUS)),
        Stroke::new(1.0, theme::LINE),
        egui::StrokeKind::Inside,
    );
}

fn action(
    ui: &mut Ui,
    app: &App,
    view: &View<'_>,
    choice: Option<&Result<Choice<'_>, Rejection>>,
    effects: &mut Vec<Effect>,
) {
    if let Some(phase) = view.jobs.get(&app.id) {
        widgets::progress_button(ui, &phase_label(view, *phase), phase.fraction());
        if matches!(phase, Phase::Waiting | Phase::Downloading { .. })
            && widgets::button(ui, view.t(Text::Cancel), Emphasis::Secondary, true).clicked()
        {
            effects.push(Effect::Cancel(app.id.clone()));
        }
        return;
    }
    let installable = matches!(choice, Some(Ok(_)));
    if let Some(installation) = view.installed.get(&app.id) {
        let outdated = app
            .manifest()
            .is_some_and(|manifest| installation.is_outdated(manifest));
        if outdated && installable {
            if widgets::button(ui, view.t(Text::Update), Emphasis::Primary, true).clicked() {
                effects.push(Effect::Install(app.id.clone()));
            }
        } else if widgets::button(
            ui,
            view.t(Text::Launch),
            Emphasis::Primary,
            installation.launch.is_some(),
        )
        .clicked()
        {
            effects.push(Effect::Launch(app.id.clone()));
        }
        return;
    }
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
    let (label, color) = status(app, view);
    widgets::chip(ui, &label, color);
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
        widgets::chip(ui, &megabytes(asset.size, view.language), theme::TEXT_MUTED);
    }
}

fn overview(
    ui: &mut Ui,
    app: &App,
    view: &View<'_>,
    choice: Option<&Result<Choice<'_>, Rejection>>,
    effects: &mut Vec<Effect>,
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
            .map(|choice| (Text::Size, megabytes(choice.asset.size, view.language))),
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
    if let Some(installation) = view.installed.get(&app.id) {
        ui.add_space(theme::M);
        ui.horizontal(|ui| {
            if widgets::button(ui, view.t(Text::ShowFolder), Emphasis::Secondary, true).clicked() {
                effects.push(Effect::Reveal(app.id.clone()));
            }
            let idle = !view.jobs.contains_key(&app.id);
            if installation.removal.is_some()
                && widgets::button(ui, view.t(Text::Uninstall), Emphasis::Danger, idle).clicked()
            {
                effects.push(Effect::Uninstall(app.id.clone()));
            }
        });
    }
    if let Some(message) = view.failures.get(&app.id) {
        ui.add_space(theme::M);
        widgets::paragraph(
            ui,
            &format!("{} · {message}", view.t(Text::Failed)),
            theme::DANGER,
        );
    }
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
                    3.0,
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
}

fn megabytes(bytes: u64, language: Language) -> String {
    let tenths = bytes.saturating_mul(10) / (1024 * 1024);
    let separator = match language {
        Language::German => ',',
        Language::English => '.',
    };
    format!("{}{separator}{} MB", tenths / 10, tenths % 10)
}

fn updates(
    ui: &mut Ui,
    app: &App,
    view: &View<'_>,
    settings: &mut Settings,
    effects: &mut Vec<Effect>,
) {
    if !view.installed.contains_key(&app.id) {
        return;
    }
    ui.add_space(theme::M);
    let mut mode = settings.update_mode(&app.id);
    let mut changed = false;
    widgets::setting(ui, view.t(Text::SettingsUpdates), None, |ui| {
        changed = widgets::segmented(
            ui,
            &mut mode,
            &[
                (UpdateMode::Automatic, view.t(Text::UpdateAutomatic)),
                (UpdateMode::Notify, view.t(Text::UpdateNotify)),
                (UpdateMode::Off, view.t(Text::Off)),
            ],
        )
        .changed();
    });
    if changed {
        settings.apps.entry(app.id.clone()).or_default().updates = Some(mode);
        effects.push(Effect::SettingsChanged);
    }
}

#[cfg(test)]
mod tests {
    use tgow_core::settings::Language;

    use super::megabytes;

    #[test]
    fn writes_sizes_with_the_decimal_separator_of_the_language() {
        assert_eq!(megabytes(6_675_488, Language::German), "6,3 MB");
        assert_eq!(megabytes(6_675_488, Language::English), "6.3 MB");
    }
}
