use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use egui::{Context, OpenUrl, Ui, ViewportCommand};
use tgow_core::catalog::App;
use tgow_core::deeplink::DeepLink;
use tgow_core::platform::Platform;
use tgow_core::settings::{Language, Settings, UpdateMode, WindowGeometry};
use tgow_platform::paths::Paths;
use tgow_platform::{integration, storage};
use tgow_ui::shell::{CatalogStatus, Effect, Page, Shell, View};
use tgow_ui::text::{self, Text};

use crate::backend::{Backend, Command, Event};
use crate::library::Library;
use crate::update::{self, Guard};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lifecycle {
    Running,
    RestartPending,
    Quitting,
}

pub(crate) struct Launcher {
    shell: Shell,
    settings: Settings,
    paths: Option<Paths>,
    platform: Platform,
    locale: Option<String>,
    system_reduced_motion: bool,
    apps: Vec<App>,
    status: CatalogStatus,
    checked_at: Option<Instant>,
    backend: Backend,
    interval: u32,
    library: Library,
    messages: Option<Receiver<String>>,
    autostart: Option<bool>,
    lifecycle: Lifecycle,
    guard: Option<Guard>,
    confirmed: bool,
    #[cfg(windows)]
    tray: Option<crate::tray::Tray>,
}

impl Launcher {
    pub(crate) fn new(
        ctx: &Context,
        paths: Option<Paths>,
        settings: Settings,
        messages: Option<Receiver<String>>,
    ) -> Self {
        tgow_ui::theme::install(ctx);
        let _ = crate::instance::WAKER.set(ctx.clone());
        let interval = settings.check_interval_minutes;
        let backend = Backend::start(
            ctx.clone(),
            paths.as_ref().map(|paths| paths.cache.join("catalog.json")),
            minutes(interval),
        );
        let library = Library::new(
            ctx,
            paths.as_ref().map(Paths::installed),
            paths
                .as_ref()
                .map(|paths| (paths.downloads(), paths.apps.clone(), paths.data.clone())),
        );
        let guard = paths
            .as_ref()
            .zip(std::env::current_exe().ok())
            .map(|(paths, executable)| Guard::new(&paths.data, &executable));
        let locale = tgow_platform::system_locale();
        if let (Some(paths), Ok(executable)) = (&paths, std::env::current_exe())
            && paths.is_installed_copy(&executable)
        {
            let _ = integration::register_scheme(&executable);
        }
        #[cfg(windows)]
        let tray = crate::tray::Tray::new(ctx, settings.language.resolve(locale.as_deref()));
        let mut launcher = Self {
            shell: Shell::default(),
            settings,
            paths,
            platform: tgow_platform::detect::platform(),
            locale,
            system_reduced_motion: tgow_platform::prefers_reduced_motion(),
            apps: Vec::new(),
            status: CatalogStatus::Loading,
            checked_at: None,
            backend,
            interval,
            library,
            messages,
            autostart: None,
            lifecycle: Lifecycle::Running,
            guard,
            confirmed: false,
            #[cfg(windows)]
            tray,
        };
        launcher.apply_autostart();
        launcher
    }

    pub(crate) fn open(&mut self, message: &str) {
        if let Some(DeepLink::Open(app) | DeepLink::Install(app)) = DeepLink::parse(message) {
            self.shell.page = Page::App(app);
        }
    }

    fn language(&self) -> Language {
        self.settings.language.resolve(self.locale.as_deref())
    }

    fn has_tray(&self) -> bool {
        #[cfg(windows)]
        {
            self.tray.is_some()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    fn save_settings(&self) {
        if let Some(paths) = &self.paths {
            let _ = storage::write_atomic(&paths.settings(), self.settings.to_json().as_bytes());
        }
    }

    fn apply_autostart(&mut self) {
        let wanted = self.settings.launch_at_login
            && self.paths.as_ref().is_some_and(|paths| !paths.portable);
        if self.autostart == Some(wanted) {
            return;
        }
        if let Ok(executable) = std::env::current_exe()
            && integration::set_autostart(wanted, &executable).is_ok()
        {
            self.autostart = Some(wanted);
        }
    }

    fn remember_window(&mut self, ctx: &Context) {
        let geometry = ctx.input(|input| {
            let viewport = input.viewport();
            let maximized = viewport.maximized.unwrap_or(false);
            let outer = viewport.outer_rect?;
            let inner = viewport.inner_rect?;
            Some(WindowGeometry {
                x: outer.left(),
                y: outer.top(),
                width: inner.width(),
                height: inner.height(),
                maximized,
            })
        });
        if let Some(geometry) = geometry {
            if geometry.maximized {
                if let Some(window) = &mut self.settings.window {
                    window.maximized = true;
                }
            } else {
                self.settings.window = Some(geometry);
            }
        }
    }

    fn show_window(ctx: &Context) {
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
    }

    fn handle_message(&mut self, ctx: &Context, message: &str) {
        match DeepLink::parse(message) {
            Some(DeepLink::Open(app) | DeepLink::Install(app)) => {
                self.shell.page = Page::App(app);
                Self::show_window(ctx);
            }
            Some(DeepLink::Show) | None => Self::show_window(ctx),
        }
    }

    fn poll_catalog(&mut self) {
        for event in self.backend.poll() {
            match event {
                Event::Apps(apps) => {
                    self.apps = apps;
                    self.library.detect(&self.apps);
                    self.library
                        .update_outdated(&self.apps, &self.platform, &self.settings);
                }
                Event::Launcher(app) => self.consider_self_update(&app),
                Event::Status(status) => self.status = status,
                Event::Checked => self.checked_at = Some(Instant::now()),
            }
        }
    }

    fn consider_self_update(&mut self, app: &App) {
        let Some(manifest) = app.manifest() else {
            return;
        };
        let version = manifest.version.to_string();
        let allowed = !cfg!(debug_assertions)
            && self.settings.launcher_updates == UpdateMode::Automatic
            && self.lifecycle == Lifecycle::Running
            && !self.library.phases.contains_key(&app.id)
            && update::is_newer(&version, env!("CARGO_PKG_VERSION"))
            && self
                .guard
                .as_ref()
                .is_some_and(|guard| !guard.is_skipped(&version));
        if allowed {
            self.library.self_update(app, &self.platform);
        }
    }

    fn restart(&mut self, ctx: &Context, hidden: bool) {
        let Ok(executable) = std::env::current_exe() else {
            return;
        };
        let mut arguments = vec![update::AFTER_UPDATE_FLAG.to_owned()];
        if hidden {
            arguments.push(integration::HIDDEN_FLAG.to_owned());
        }
        if tgow_platform::process::launch(&executable, &arguments).is_ok() {
            self.lifecycle = Lifecycle::Quitting;
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    fn poll_library(&mut self, now: f64) {
        for (app, version) in self.library.poll() {
            if app == tgow_core::catalog::LAUNCHER_REPO {
                self.lifecycle = Lifecycle::RestartPending;
                let text = match self.language() {
                    Language::German => format!(
                        "T-GOW Launcher {version} ist bereit und startet beim Schließen neu"
                    ),
                    Language::English => {
                        format!("T-GOW Launcher {version} is ready and restarts when closed")
                    }
                };
                self.shell.notify(text, now);
                continue;
            }
            let title = self
                .apps
                .iter()
                .find(|known| known.id == app)
                .map_or(app.clone(), |known| known.title().to_owned());
            let text = match self.language() {
                Language::German => format!("{title} {version} ist installiert"),
                Language::English => format!("{title} {version} is installed"),
            };
            self.shell.notify(text, now);
        }
    }

    #[cfg(windows)]
    fn poll_tray(&mut self, ctx: &Context) {
        let Some(tray) = &self.tray else {
            return;
        };
        for action in tray.poll() {
            match action {
                crate::tray::Action::Open => Self::show_window(ctx),
                crate::tray::Action::Check => self.backend.send(Command::Refresh),
                crate::tray::Action::Visit(url) => ctx.open_url(OpenUrl::new_tab(url)),
                crate::tray::Action::Quit => {
                    self.lifecycle = Lifecycle::Quitting;
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }
    }

    fn apply(&mut self, ctx: &Context, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Open(url) => ctx.open_url(OpenUrl::new_tab(url)),
                Effect::Copy(text) => ctx.copy_text(text),
                Effect::SettingsChanged => {
                    self.save_settings();
                    self.apply_autostart();
                    if self.settings.check_interval_minutes != self.interval {
                        self.interval = self.settings.check_interval_minutes;
                        self.backend.send(Command::Interval(minutes(self.interval)));
                    }
                }
                Effect::CheckNow => self.backend.send(Command::Refresh),
                Effect::Install(id) => {
                    if let Some(app) = self.apps.iter().find(|app| app.id == id) {
                        self.library.install(app, &self.platform, &self.settings);
                    }
                }
                Effect::Launch(id) => {
                    if !self.library.launch(&id, &self.settings) {
                        let now = ctx.input(|input| input.time);
                        self.shell
                            .notify(Text::Failed.get(self.language()).to_owned(), now);
                    }
                }
                Effect::Uninstall(id) => self.library.uninstall(&id),
                Effect::Reveal(id) => self.library.reveal(&id),
                Effect::Cancel(id) => self.library.cancel(&id),
            }
        }
    }
}

impl eframe::App for Launcher {
    fn logic(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let messages: Vec<String> = self
            .messages
            .as_ref()
            .map(|receiver| receiver.try_iter().collect())
            .unwrap_or_default();
        for message in messages {
            self.handle_message(ctx, &message);
        }
        #[cfg(windows)]
        self.poll_tray(ctx);
        self.poll_catalog();
        self.poll_library(ctx.input(|input| input.time));
        let close = ctx.input(|input| input.viewport().close_requested());
        if close && self.lifecycle == Lifecycle::RestartPending && !self.library.is_busy() {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            let to_tray = self.settings.close_to_tray && self.has_tray();
            self.restart(ctx, to_tray);
        } else if close
            && self.lifecycle != Lifecycle::Quitting
            && self.settings.close_to_tray
            && self.has_tray()
        {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        }
        let hidden = !ctx.input(|input| input.viewport().visible().unwrap_or(true));
        if hidden && self.lifecycle == Lifecycle::RestartPending && !self.library.is_busy() {
            self.restart(ctx, true);
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let reduced = self.settings.motion.is_reduced(self.system_reduced_motion);
        tgow_ui::widgets::set_reduced_motion(&ctx, reduced);
        let language = self.language();
        let checked = self
            .checked_at
            .map(|at| text::minutes_ago(language, at.elapsed().as_secs() / 60));
        let view = View {
            language,
            platform: &self.platform,
            apps: &self.apps,
            status: self.status.clone(),
            checked: checked.as_deref(),
            version: env!("CARGO_PKG_VERSION"),
            installed: &self.library.installed,
            jobs: &self.library.phases,
            failures: &self.library.failures,
        };
        let effects = self.shell.show(ui, &view, &mut self.settings);
        self.apply(&ctx, effects);
        self.remember_window(&ctx);
        if !self.confirmed {
            self.confirmed = true;
            if let Some(guard) = &self.guard {
                guard.confirm();
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_settings();
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::from(tgow_ui::theme::BG).to_array()
    }
}

fn minutes(value: u32) -> Duration {
    Duration::from_secs(u64::from(value) * 60)
}
