use std::collections::BTreeMap;
use std::time::Duration;

use egui::{Rect, Sense, Stroke, Ui, UiBuilder, Vec2, ViewportCommand, pos2, vec2};
use tgow_core::catalog::App;
use tgow_core::install::Installation;
use tgow_core::links::Destination;
use tgow_core::platform::Platform;
use tgow_core::settings::{Language, Settings};

use crate::icons::Icon;
use crate::pages;
use crate::starfield::Starfield;
use crate::text::Text;
use crate::theme::{self, Font};
use crate::widgets::{self, animate};
use crate::{brand, shape};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Library,
    Discover,
    Downloads,
    News,
    Settings,
    About,
    App(String),
    Welcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogStatus {
    Loading,
    Ready,
    Offline,
    Limited { minutes: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Waiting,
    Downloading { done: u64, total: Option<u64> },
    Verifying,
    Installing,
    Removing,
}

impl Phase {
    #[must_use]
    pub fn fraction(self) -> Option<f32> {
        match self {
            Self::Downloading {
                done,
                total: Some(total),
            } if total > 0 => Some(shape::ratio(done, total)),
            _ => None,
        }
    }
}

pub struct View<'a> {
    pub language: Language,
    pub platform: &'a Platform,
    pub apps: &'a [App],
    pub status: CatalogStatus,
    pub checked: Option<&'a str>,
    pub version: &'static str,
    pub installed: &'a BTreeMap<String, Installation>,
    pub jobs: &'a BTreeMap<String, Phase>,
    pub failures: &'a BTreeMap<String, String>,
}

impl View<'_> {
    #[must_use]
    pub fn t(&self, text: Text) -> &'static str {
        text.get(self.language)
    }

    #[must_use]
    pub fn app(&self, id: &str) -> Option<&App> {
        self.apps.iter().find(|app| app.id == id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Open(String),
    Copy(String),
    SettingsChanged,
    CheckNow,
    Install(String),
    Launch(String),
    Uninstall(String),
    Reveal(String),
    Cancel(String),
    InstallSelf,
    SkipInstall,
}

#[derive(Default)]
pub struct Shell {
    pub page: Page,
    pub(crate) settings_page: pages::settings::Section,
    pub(crate) library: pages::library::State,
    pub(crate) detail: pages::app::State,
    pub(crate) copied_at: Option<f64>,
    notices: Vec<(String, f64)>,
    ambient: f32,
    starfield: Starfield,
}

const NOTICE_SECONDS: f64 = 5.0;

impl Shell {
    pub fn notify(&mut self, text: String, now: f64) {
        self.notices.push((text, now));
    }

    pub fn show(&mut self, ui: &mut Ui, view: &View<'_>, settings: &mut Settings) -> Vec<Effect> {
        let mut effects = Vec::new();
        let full = ui.max_rect();
        let time = ui.input(|input| input.time);
        let focused = ui.input(|input| input.viewport().focused.unwrap_or(true));
        let animated = !widgets::reduced_motion(ui.ctx()) && focused;
        if animated {
            self.ambient = shape::seconds(time);
        }
        self.starfield.paint(ui.painter(), full, self.ambient);
        if animated {
            ui.ctx().request_repaint_after(Duration::from_millis(50));
        }
        let expansion = animate(
            ui.ctx(),
            ui.scope_id().with("sidebar"),
            settings.sidebar_expanded,
            theme::NORMAL,
        );
        let sidebar_width = egui::lerp(
            theme::SIDEBAR_COLLAPSED..=theme::SIDEBAR_EXPANDED,
            expansion,
        );
        let title = Rect::from_min_size(full.min, vec2(full.width(), theme::TITLE_BAR));
        let sidebar = Rect::from_min_max(
            pos2(full.left(), title.bottom()),
            pos2(full.left() + sidebar_width, full.bottom()),
        );
        let content = Rect::from_min_max(pos2(sidebar.right(), title.bottom()), full.max);

        let mut title_ui = ui.new_child(UiBuilder::new().max_rect(title));
        self.title_bar(&mut title_ui, view, &mut effects);
        let mut sidebar_ui = ui.new_child(UiBuilder::new().max_rect(sidebar));
        self.sidebar(&mut sidebar_ui, view, settings, expansion, &mut effects);
        let mut content_ui = ui.new_child(UiBuilder::new().max_rect(content));
        self.content(&mut content_ui, view, settings, &mut effects);
        self.paint_notices(ui, content, time);
        effects
    }

    fn paint_notices(&mut self, ui: &Ui, area: Rect, now: f64) {
        self.notices.retain(|(_, at)| now - at < NOTICE_SECONDS);
        if self.notices.is_empty() {
            return;
        }
        ui.ctx().request_repaint_after(Duration::from_millis(100));
        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::unique("tgow.notices"),
        ));
        let mut bottom = area.bottom() - theme::L;
        for (text, at) in self.notices.iter().rev() {
            let age = now - at;
            let fade = shape::seconds((NOTICE_SECONDS - age).min(0.4) / 0.4).clamp(0.0, 1.0);
            let galley = painter.layout_job(widgets::job(
                text,
                Font::Strong,
                14.0,
                theme::TEXT.gamma_multiply(fade),
            ));
            let size = galley.size() + vec2(48.0, 28.0);
            let rect = Rect::from_min_size(
                pos2(area.right() - theme::L - size.x, bottom - size.y),
                size,
            );
            shape::glow(
                &painter,
                rect,
                8.0,
                theme::ACCENT.gamma_multiply(0.2 * fade),
                18,
            );
            shape::fill(
                &painter,
                rect,
                8.0,
                theme::SURFACE_1.gamma_multiply(fade),
                Stroke::new(1.0, theme::ACCENT.gamma_multiply(fade)),
            );
            painter.rect_filled(
                Rect::from_min_size(rect.min + vec2(0.0, 8.0), vec2(3.0, rect.height() - 16.0)),
                0.0,
                theme::ACCENT.gamma_multiply(fade),
            );
            painter.galley(
                pos2(rect.left() + 24.0, rect.center().y - galley.size().y / 2.0),
                galley,
                theme::TEXT,
            );
            bottom = rect.top() - theme::S;
        }
    }

    fn title_bar(&mut self, ui: &mut Ui, view: &View<'_>, effects: &mut Vec<Effect>) {
        let rect = ui.max_rect();
        let drag = ui.interact(rect, ui.scope_id().with("drag"), Sense::click_and_drag());
        let maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));
        if drag.double_clicked() {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
        } else if drag.drag_started() {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, theme::GLASS);
        painter.hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            Stroke::new(1.0, theme::LINE),
        );
        brand::paint_mark(
            painter,
            Rect::from_center_size(pos2(rect.left() + 26.0, rect.center().y), Vec2::splat(26.0)),
        );
        let name = widgets::galley(ui, view.t(Text::Product), Font::Button, 13.0, theme::TEXT);
        painter.galley(
            pos2(rect.left() + 48.0, rect.center().y - name.size().y / 2.0),
            name,
            theme::TEXT,
        );

        ui.scope_builder(
            UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                if widgets::window_button(ui, Icon::Close, view.t(Text::Close), true).clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
                let (icon, label) = if maximized {
                    (Icon::Restore, Text::Restore)
                } else {
                    (Icon::Maximize, Text::Maximize)
                };
                if widgets::window_button(ui, icon, view.t(label), false).clicked() {
                    ui.ctx()
                        .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                }
                if widgets::window_button(ui, Icon::Minimize, view.t(Text::Minimize), false)
                    .clicked()
                {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
                }
                ui.add_space(theme::S);
                if let Some(url) = Destination::main_discord()
                    && widgets::icon_button(ui, Icon::Chat, view.t(Text::Discord), 34.0).clicked()
                {
                    effects.push(Effect::Open(url.to_owned()));
                }
                if widgets::icon_button(ui, Icon::Downloads, view.t(Text::Downloads), 34.0)
                    .clicked()
                {
                    self.page = Page::Downloads;
                }
                ui.add_space(theme::S);
                status_dot(ui, view);
            },
        );
    }

    fn sidebar(
        &mut self,
        ui: &mut Ui,
        view: &View<'_>,
        settings: &mut Settings,
        expansion: f32,
        effects: &mut Vec<Effect>,
    ) {
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, theme::GLASS);
        ui.painter().vline(
            rect.right() - 0.5,
            rect.y_range(),
            Stroke::new(1.0, theme::LINE),
        );
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.add_space(theme::M);
        for (page, icon, label) in [
            (Page::Library, Icon::Library, Text::Library),
            (Page::Discover, Icon::Discover, Text::Discover),
            (Page::Downloads, Icon::Downloads, Text::Downloads),
            (Page::News, Icon::News, Text::News),
            (Page::Settings, Icon::Settings, Text::Settings),
            (Page::About, Icon::About, Text::About),
        ] {
            let selected =
                self.page == page || (page == Page::Library && matches!(self.page, Page::App(_)));
            if widgets::nav_item(ui, icon, view.t(label), selected, expansion).clicked() {
                self.page = page;
            }
        }
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.add_space(theme::S);
            let (icon, label) = if settings.sidebar_expanded {
                (Icon::Collapse, Text::CollapseSidebar)
            } else {
                (Icon::Expand, Text::ExpandSidebar)
            };
            if widgets::nav_item(ui, icon, view.t(label), false, expansion).clicked() {
                settings.sidebar_expanded = !settings.sidebar_expanded;
                effects.push(Effect::SettingsChanged);
            }
            let line_y = ui.cursor().bottom() - 4.0;
            ui.painter().hline(
                (rect.left() + 16.0)..=(rect.right() - 16.0),
                line_y,
                Stroke::new(1.0, theme::LINE),
            );
            ui.add_space(theme::S);
            for (destination, icon, label) in [
                (Destination::GitHub, Icon::Code, Text::GitHub),
                (Destination::Website, Icon::Globe, Text::Website),
                (Destination::DiscordCommunity, Icon::Chat, Text::Discord),
            ] {
                let Some(url) = destination.url() else {
                    continue;
                };
                if widgets::nav_item(ui, icon, view.t(label), false, expansion).clicked() {
                    effects.push(Effect::Open(url.to_owned()));
                }
            }
        });
    }

    fn content(
        &mut self,
        ui: &mut Ui,
        view: &View<'_>,
        settings: &mut Settings,
        effects: &mut Vec<Effect>,
    ) {
        let page = self.page.clone();
        match page {
            Page::App(id) => pages::app::show(self, ui, view, &id, settings, effects),
            Page::Library => pages::library::show(self, ui, view, false, effects),
            Page::Discover => pages::library::show(self, ui, view, true, effects),
            Page::Downloads => pages::downloads::show(ui, view, effects),
            Page::News => pages::news::show(self, ui, view),
            Page::Settings => pages::settings::show(self, ui, view, settings, effects),
            Page::About => pages::about::show(self, ui, view, effects),
            Page::Welcome => pages::welcome::show(ui, view, effects),
        }
    }
}

pub(crate) fn padded(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(40, 32))
                .show(ui, |ui| {
                    ui.set_max_width(ui.available_width().min(1180.0));
                    add_contents(ui);
                });
        });
}

fn status_dot(ui: &mut Ui, view: &View<'_>) {
    let (color, label) = match &view.status {
        CatalogStatus::Ready => (theme::POSITIVE, view.t(Text::CatalogUpdated).to_owned()),
        CatalogStatus::Loading => (theme::ACCENT, view.t(Text::CatalogLoading).to_owned()),
        CatalogStatus::Offline => (theme::WARNING, view.t(Text::CatalogOffline).to_owned()),
        CatalogStatus::Limited { minutes } => (theme::WARNING, format!("GitHub · {minutes} min")),
    };
    let (rect, response) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
    ui.painter().circle_stroke(
        rect.center(),
        7.0,
        Stroke::new(1.0, color.gamma_multiply(0.4)),
    );
    let detail = view
        .checked
        .map_or(label.clone(), |checked| format!("{label} · {checked}"));
    response.on_hover_text(detail);
}
