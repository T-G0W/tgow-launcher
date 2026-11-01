use egui::{Context, OpenUrl, Ui};
use tgow_core::catalog::App;
use tgow_core::platform::Platform;
use tgow_core::settings::{Settings, WindowGeometry};
use tgow_platform::paths::Paths;
use tgow_platform::storage;
use tgow_ui::shell::{CatalogStatus, Effect, Shell, View};

pub(crate) struct Launcher {
    shell: Shell,
    settings: Settings,
    paths: Option<Paths>,
    platform: Platform,
    locale: Option<String>,
    system_reduced_motion: bool,
    apps: Vec<App>,
    status: CatalogStatus,
    checked: Option<String>,
}

impl Launcher {
    pub(crate) fn new(ctx: &Context, paths: Option<Paths>, settings: Settings) -> Self {
        tgow_ui::theme::install(ctx);
        Self {
            shell: Shell::default(),
            settings,
            paths,
            platform: tgow_platform::detect::platform(),
            locale: tgow_platform::system_locale(),
            system_reduced_motion: tgow_platform::prefers_reduced_motion(),
            apps: Vec::new(),
            status: CatalogStatus::Ready,
            checked: None,
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
                Effect::SettingsChanged => self.save_settings(),
                Effect::CheckNow | Effect::Install(_) => {}
            }
        }
    }
}

impl eframe::App for Launcher {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let reduced = self.settings.motion.is_reduced(self.system_reduced_motion);
        tgow_ui::widgets::set_reduced_motion(&ctx, reduced);
        let language = self.settings.language.resolve(self.locale.as_deref());
        let view = View {
            language,
            platform: &self.platform,
            apps: &self.apps,
            status: self.status.clone(),
            checked: self.checked.as_deref(),
            version: env!("CARGO_PKG_VERSION"),
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
