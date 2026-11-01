#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

mod backend;
mod launcher;

use std::sync::Arc;

use eframe::{NativeOptions, Renderer};
use egui::{IconData, ViewportBuilder};
use tgow_core::settings::Settings;
use tgow_platform::paths::Paths;
use tgow_platform::storage;

use crate::launcher::Launcher;

const APP_ID: &str = "io.github.T_G0W.Launcher";
const ICON_SIZE: u32 = 128;

fn main() -> eframe::Result {
    let paths = Paths::locate();
    let settings = paths
        .as_ref()
        .and_then(|paths| storage::read_text(&paths.settings()))
        .map(|text| Settings::from_json(&text))
        .unwrap_or_default();
    let result = run(Renderer::Wgpu, paths.as_ref(), &settings);
    if result.is_err() {
        run(Renderer::Glow, paths.as_ref(), &settings)
    } else {
        result
    }
}

fn run(renderer: Renderer, paths: Option<&Paths>, settings: &Settings) -> eframe::Result {
    let options = NativeOptions {
        viewport: viewport(settings),
        renderer,
        centered: settings.window.is_none(),
        ..NativeOptions::default()
    };
    let paths = paths.cloned();
    let settings = settings.clone();
    eframe::run_native(
        "T-GOW Launcher",
        options,
        Box::new(move |creation| Ok(Box::new(Launcher::new(&creation.egui_ctx, paths, settings)))),
    )
}

fn viewport(settings: &Settings) -> ViewportBuilder {
    let icon = IconData {
        rgba: tgow_ui::brand::icon_rgba(ICON_SIZE),
        width: ICON_SIZE,
        height: ICON_SIZE,
    };
    let mut viewport = ViewportBuilder::default()
        .with_title("T-GOW Launcher")
        .with_app_id(APP_ID)
        .with_decorations(false)
        .with_inner_size([1280.0, 760.0])
        .with_min_inner_size([1024.0, 640.0])
        .with_icon(Arc::new(icon));
    if let Some(window) = settings.window {
        viewport = viewport
            .with_position([window.x, window.y])
            .with_inner_size([window.width, window.height])
            .with_maximized(window.maximized);
    }
    viewport
}
