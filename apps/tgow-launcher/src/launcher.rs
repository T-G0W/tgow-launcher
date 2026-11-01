use std::time::{Duration, Instant};

use egui::{Context, OpenUrl, Ui};
use tgow_core::catalog::App;
use tgow_core::platform::Platform;
use tgow_core::settings::{Settings, WindowGeometry};
use tgow_platform::paths::Paths;
use tgow_platform::storage;
use tgow_ui::shell::{CatalogStatus, Effect, Shell, View};
use tgow_ui::text;

use crate::backend::{Backend, Command, Event};
use crate::library::Library;

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
}

impl Launcher {
    pub(crate) fn new(ctx: &Context, paths: Option<Paths>, settings: Settings) -> Self {
        tgow_ui::theme::install(ctx);
        let interval = settings.check_interval_minutes;
        let backend = Backend::start(
            ctx.clone(),
            paths.as_ref().map(|paths| paths.cache.join("catalog.json")),
            minutes(interval),
        );
        let library = Library::new(
            ctx,
            paths.as_ref().map(Paths::installed),
            paths.as_ref().map(Paths::downloads),
            paths.as_ref().map(|paths| paths.apps.clone()),
        );
        Self {
            shell: Shell::default(),
            settings,
            paths,
            platform: tgow_platform::detect::platform(),
            locale: tgow_platform::system_locale(),
            system_reduced_motion: tgow_platform::prefers_reduced_motion(),
            apps: Vec::new(),
            status: CatalogStatus::Loading,
            checked_at: None,
            backend,
            interval,
            library,
        }
    }

    fn save_settings(&self) {
        if let Some(paths) = &self.paths {
            let _ = storage::write_atomic(&paths.settings(), self.settings.to_json().as_bytes());
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

    fn apply(&mut self, ctx: &Context, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Open(url) => ctx.open_url(OpenUrl::new_tab(url)),
                Effect::Copy(text) => ctx.copy_text(text),
                Effect::SettingsChanged => {
                    self.save_settings();
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
                    let _ = self.library.launch(&id, &self.settings);
                }
                Effect::Uninstall(id) => self.library.uninstall(&id),
                Effect::Reveal(id) => self.library.reveal(&id),
                Effect::Cancel(id) => self.library.cancel(&id),
            }
        }
    }
}

impl eframe::App for Launcher {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        for event in self.backend.poll() {
            match event {
                Event::Apps(apps) => {
                    self.apps = apps;
                    self.library.detect(&self.apps);
                    self.library
                        .update_outdated(&self.apps, &self.platform, &self.settings);
                }
                Event::Status(status) => self.status = status,
                Event::Checked => self.checked_at = Some(Instant::now()),
            }
        }
        self.library.poll();
        let reduced = self.settings.motion.is_reduced(self.system_reduced_motion);
        tgow_ui::widgets::set_reduced_motion(&ctx, reduced);
        let language = self.settings.language.resolve(self.locale.as_deref());
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
