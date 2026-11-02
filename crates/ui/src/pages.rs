pub(crate) mod about;
pub(crate) mod app;
pub(crate) mod downloads;
pub(crate) mod library;
pub(crate) mod news;
pub(crate) mod settings;
pub(crate) mod welcome;

use egui::{Align2, Color32, Painter, Pos2, Rect, Shape, Stroke, Ui, pos2, vec2};
use tgow_core::catalog::{App, ReleaseState};

use crate::icons::Icon;
use crate::shape;
use crate::shell::{Artwork, Effect, Phase, View};
use crate::text::Text;
use crate::theme::{self, Font};
use crate::widgets;

pub(crate) fn paint_chip(painter: &Painter, at: Pos2, label: &str, color: Color32) -> f32 {
    let galley = painter.layout_job(widgets::job(label, Font::Label, 10.0, color));
    let rect = Rect::from_min_size(at, galley.size() + vec2(18.0, 10.0));
    shape::fill(
        painter,
        rect,
        theme::RADIUS_S,
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

pub(crate) fn paint_icon(
    painter: &Painter,
    app: &App,
    artwork: Option<&Artwork>,
    tile: Rect,
    accent: Color32,
) {
    shape::fill(
        painter,
        tile,
        theme::RADIUS,
        theme::SURFACE_2,
        Stroke::new(1.0, accent.gamma_multiply(0.7)),
    );
    if let Some(icon) = artwork.and_then(|artwork| artwork.icon.as_ref()) {
        let inner = tile.shrink(tile.width() * 0.12);
        painter.add(Shape::mesh(shape::textured_polygon(
            &shape::rounded(inner, theme::RADIUS_S),
            inner,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            icon.id(),
            1.0,
            1.0,
        )));
    } else {
        painter.text(
            tile.center(),
            Align2::CENTER_CENTER,
            initial(app.title()),
            Font::Display.id(tile.height() * 0.52),
            accent,
        );
    }
}

pub(crate) enum MenuChoice {
    Details,
    Do(Effect),
}

pub(crate) fn app_menu(
    ui: &mut Ui,
    app: &App,
    view: &View<'_>,
    details: bool,
) -> Option<MenuChoice> {
    let mut choice = None;
    let mut pick = |clicked: bool, value: MenuChoice| {
        if clicked {
            choice = Some(value);
        }
    };
    if details {
        pick(
            widgets::menu_item(ui, Icon::About, view.t(Text::Details), false).clicked(),
            MenuChoice::Details,
        );
    }
    let installation = view.installed.get(&app.id);
    let id = || app.id.clone();
    match (view.jobs.get(&app.id), installation) {
        (Some(phase), _) => {
            if matches!(phase, Phase::Waiting | Phase::Downloading { .. }) {
                pick(
                    widgets::menu_item(ui, Icon::Close, view.t(Text::Cancel), false).clicked(),
                    MenuChoice::Do(Effect::Cancel(id())),
                );
            }
        }
        (None, Some(installation)) => {
            if app
                .manifest()
                .is_some_and(|manifest| installation.is_outdated(manifest))
            {
                pick(
                    widgets::menu_item(ui, Icon::Refresh, view.t(Text::Update), false).clicked(),
                    MenuChoice::Do(Effect::Install(id())),
                );
            }
            if installation.launch.is_some() {
                pick(
                    widgets::menu_item(ui, Icon::Play, view.t(Text::Launch), false).clicked(),
                    MenuChoice::Do(Effect::Launch(id())),
                );
            }
            pick(
                widgets::menu_item(ui, Icon::Folder, view.t(Text::ShowFolder), false).clicked(),
                MenuChoice::Do(Effect::Reveal(id())),
            );
        }
        (None, None) => {
            if matches!(app.release, ReleaseState::Verified(_)) {
                pick(
                    widgets::menu_item(ui, Icon::Downloads, view.t(Text::Install), false).clicked(),
                    MenuChoice::Do(Effect::Install(id())),
                );
            }
        }
    }
    widgets::menu_separator(ui);
    pick(
        widgets::menu_item(ui, Icon::Code, view.t(Text::OpenOnGitHub), false).clicked(),
        MenuChoice::Do(Effect::Open(app.html_url.clone())),
    );
    let removable = installation.is_some_and(|installation| installation.removal.is_some());
    if removable && !view.jobs.contains_key(&app.id) {
        widgets::menu_separator(ui);
        pick(
            widgets::menu_item(ui, Icon::Trash, view.t(Text::Uninstall), true).clicked(),
            MenuChoice::Do(Effect::Uninstall(id())),
        );
    }
    if choice.is_some() {
        ui.close();
    }
    choice
}
