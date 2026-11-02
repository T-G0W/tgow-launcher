use std::collections::BTreeMap;
use std::time::Duration;

use egui::{Align2, Rect, Sense, Stroke, Ui, UiBuilder, Vec2, ViewportCommand, pos2, vec2};
use tgow_core::catalog::App;
use tgow_core::install::Installation;
use tgow_core::links::Destination;
use tgow_core::platform::Platform;
use tgow_core::settings::{Language, Settings};

use crate::icons::Icon;
use crate::logos::Logo;
use crate::pages;
use crate::popover::{self, Popover};
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

#[derive(Default)]
pub struct Artwork {
    pub icon: Option<egui::TextureHandle>,
    pub hero: Option<egui::TextureHandle>,
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
    pub artwork: &'a BTreeMap<String, Artwork>,
    pub recent: &'a [Finished],
    pub launcher_latest: Option<&'a str>,
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
pub enum Outcome {
    Installed(String),
    Removed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finished {
    pub app: String,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Back,
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
    history: Vec<Page>,
    popover: Option<Popover>,
    anchors: Vec<(Popover, Rect)>,
    pub(crate) settings_page: pages::settings::Section,
    pub(crate) library: pages::library::State,
    pub(crate) detail: pages::app::State,
    pub(crate) copied_at: Option<f64>,
    notices: Vec<(String, f64)>,
    ambient: f32,
    starfield: Starfield,
}

const NOTICE_SECONDS: f64 = 5.0;
const HISTORY: usize = 32;

impl Shell {
    pub fn open(&mut self, page: Page) {
        if page == self.page {
            return;
        }
        let previous = std::mem::replace(&mut self.page, page);
        if previous != Page::Welcome {
            self.history.push(previous);
            if self.history.len() > HISTORY {
                self.history.remove(0);
            }
        }
    }

    pub fn back(&mut self) -> bool {
        match self.history.pop() {
            Some(page) => {
                self.page = page;
                true
            }
            None => false,
        }
    }

    #[must_use]
    pub fn can_go_back(&self) -> bool {
        !self.history.is_empty()
    }

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

        self.shortcuts(ui, &mut effects);
        let mut title_ui = ui.new_child(UiBuilder::new().max_rect(title));
        self.title_bar(&mut title_ui, view, &mut effects);
        let mut sidebar_ui = ui.new_child(UiBuilder::new().max_rect(sidebar));
        self.sidebar(&mut sidebar_ui, view, settings, expansion, &mut effects);
        let mut content_ui = ui.new_child(UiBuilder::new().max_rect(content));
        self.content(&mut content_ui, view, settings, &mut effects);
        self.popover(ui, view, &mut effects);
        if effects.contains(&Effect::Back) {
            effects.retain(|effect| *effect != Effect::Back);
            self.go_back();
        }
        self.paint_notices(ui, content, time);
        effects
    }

    fn shortcuts(&mut self, ui: &Ui, effects: &mut Vec<Effect>) {
        let (search, settings, downloads, refresh, escape, back) = ui.input_mut(|input| {
            (
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::F),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::J),
                input.consume_key(egui::Modifiers::NONE, egui::Key::F5),
                input.key_pressed(egui::Key::Escape),
                input.consume_key(egui::Modifiers::ALT, egui::Key::ArrowLeft)
                    || input.pointer.button_clicked(egui::PointerButton::Extra1),
            )
        });
        if search {
            self.open(Page::Library);
            self.library.focus_search = true;
        }
        if settings {
            self.open(Page::Settings);
        }
        if downloads {
            self.open(Page::Downloads);
        }
        if refresh {
            effects.push(Effect::CheckNow);
        }
        if escape && self.popover.is_some() {
            self.popover = None;
        } else if back || (escape && matches!(self.page, Page::App(_))) {
            self.go_back();
        }
    }

    fn toggle(&mut self, kind: Popover, anchor: Rect) {
        self.anchors.retain(|(existing, _)| *existing != kind);
        self.anchors.push((kind, anchor));
        self.popover = if self.popover == Some(kind) {
            None
        } else {
            Some(kind)
        };
    }

    fn popover(&mut self, ui: &Ui, view: &View<'_>, effects: &mut Vec<Effect>) {
        let Some(kind) = self.popover else {
            return;
        };
        let Some(anchor) = self
            .anchors
            .iter()
            .find(|(existing, _)| *existing == kind)
            .map(|(_, rect)| *rect)
        else {
            return;
        };
        let screen = ui.ctx().content_rect();
        let left = (anchor.right() + theme::S - popover::WIDTH).clamp(
            screen.left() + theme::S,
            screen.right() - popover::WIDTH - theme::S,
        );
        let mut open = None;
        let area = egui::Area::new(egui::Id::unique("tgow.popover"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(left, anchor.bottom() + 6.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .inner_margin(egui::Margin::same(16))
                    .show(ui, |ui| {
                        ui.set_width(popover::WIDTH - 32.0);
                        match kind {
                            Popover::Downloads => open = popover::downloads(ui, view, effects),
                            Popover::Status => popover::status(ui, view, effects),
                        }
                    });
            });
        if let Some(page) = open {
            self.popover = None;
            self.open(page);
            return;
        }
        let outside = ui.input(|input| {
            input.pointer.any_click()
                && input.pointer.interact_pos().is_some_and(|pointer| {
                    !area.response.rect.contains(pointer)
                        && !self.anchors.iter().any(|(_, rect)| rect.contains(pointer))
                })
        });
        if outside {
            self.popover = None;
        }
    }

    pub fn go_back(&mut self) {
        if !self.back() && matches!(self.page, Page::App(_)) {
            self.page = Page::Library;
        }
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
        if drag.drag_started() {
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
        let name_right = rect.left() + 48.0 + name.size().x;
        painter.galley(
            pos2(rect.left() + 48.0, rect.center().y - name.size().y / 2.0),
            name,
            theme::TEXT,
        );
        if self.can_go_back() {
            let back =
                Rect::from_center_size(pos2(name_right + 30.0, rect.center().y), Vec2::splat(30.0));
            let clicked = ui
                .scope_builder(UiBuilder::new().max_rect(back), |ui| {
                    widgets::icon_button(ui, Icon::Back, view.t(Text::Back), 30.0).clicked()
                })
                .inner;
            if clicked {
                self.go_back();
            }
        }

        ui.scope_builder(
            UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                if widgets::window_button(ui, Icon::Close, view.t(Text::Close), true).clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
                if widgets::window_button(ui, Icon::Minimize, view.t(Text::Minimize), false)
                    .clicked()
                {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
                }
                ui.add_space(theme::S);
                if let Some(url) = Destination::main_discord()
                    && widgets::icon_button(ui, Logo::Discord, view.t(Text::Discord), 34.0)
                        .clicked()
                {
                    effects.push(Effect::Open(url.to_owned()));
                }
                let downloads =
                    widgets::icon_button(ui, Icon::Downloads, view.t(Text::Downloads), 34.0);
                paint_badge(ui, downloads.rect, view.jobs.len());
                if downloads.clicked() {
                    self.toggle(Popover::Downloads, downloads.rect);
                }
                ui.add_space(theme::S);
                let status = status_dot(ui, view);
                if status.clicked() {
                    self.toggle(Popover::Status, status.rect);
                }
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
                self.open(page);
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
                (Destination::DiscordCommunity, Logo::Discord, Text::Discord),
                (Destination::Website, Logo::Website, Text::Website),
                (Destination::GitHub, Logo::GitHub, Text::GitHub),
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

fn paint_badge(ui: &Ui, rect: Rect, count: usize) {
    if count == 0 {
        return;
    }
    let center = pos2(rect.right() - 7.0, rect.top() + 8.0);
    ui.painter().circle_filled(center, 7.5, theme::ACCENT);
    ui.painter().text(
        center,
        Align2::CENTER_CENTER,
        count.min(9).to_string(),
        Font::Label.id(10.0),
        theme::ON_ACCENT,
    );
}

fn status_dot(ui: &mut Ui, view: &View<'_>) -> egui::Response {
    let (color, label) = match &view.status {
        CatalogStatus::Ready => (theme::POSITIVE, view.t(Text::CatalogUpdated).to_owned()),
        CatalogStatus::Loading => (theme::ACCENT, view.t(Text::CatalogLoading).to_owned()),
        CatalogStatus::Offline => (theme::WARNING, view.t(Text::CatalogOffline).to_owned()),
        CatalogStatus::Limited { minutes } => (theme::WARNING, format!("GitHub · {minutes} min")),
    };
    let (rect, response) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::click());
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, view.t(Text::Status)));
    ui.painter().circle_filled(rect.center(), 4.0, color);
    ui.painter().circle_stroke(
        rect.center(),
        7.0,
        Stroke::new(1.0, color.gamma_multiply(0.4)),
    );
    let detail = view
        .checked
        .map_or(label.clone(), |checked| format!("{label} · {checked}"));
    response
        .on_hover_text(detail)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}
