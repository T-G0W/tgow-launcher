#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

mod backend;
mod instance;
mod jobs;
mod launcher;
mod library;
#[cfg(windows)]
mod tray;
mod update;

use std::sync::Arc;

use eframe::{NativeOptions, Renderer};
use egui::{IconData, ViewportBuilder};
use tgow_core::settings::Settings;
use tgow_platform::integration::HIDDEN_FLAG;
use tgow_platform::paths::Paths;
use tgow_platform::storage;

use crate::instance::Claim;
use crate::launcher::Launcher;

const APP_ID: &str = "io.github.T_G0W.Launcher";
const ICON_SIZE: u32 = 128;

fn main() -> eframe::Result {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let hidden = arguments.iter().any(|argument| argument == HIDDEN_FLAG);
    let message = arguments
        .iter()
        .find(|argument| argument.starts_with("tgow:"))
        .cloned()
        .unwrap_or_default();
    if arguments
        .iter()
        .any(|argument| argument == update::AFTER_UPDATE_FLAG)
    {
        std::thread::sleep(std::time::Duration::from_millis(1500));
    }
    let paths = Paths::locate();
    if arguments
        .iter()
        .any(|argument| argument == tgow_platform::setup::UNINSTALL_FLAG)
    {
        if let Some(paths) = &paths {
            let _ = tgow_platform::setup::uninstall(paths);
        }
        return Ok(());
    }
    if let (Some(paths), Ok(executable)) = (&paths, std::env::current_exe())
        && update::Guard::new(&paths.data, &executable).on_start()
    {
        let _ = tgow_platform::process::launch(&executable, &arguments);
        return Ok(());
    }
    let messages = match instance::claim(&message) {
        Claim::Forwarded => return Ok(()),
        Claim::First(receiver) => Some(receiver),
        Claim::Unavailable => None,
    };
    let launch = Launch {
        hidden,
        messages: std::sync::Mutex::new(messages),
        initial: message,
    };
    let settings = paths
        .as_ref()
        .and_then(|paths| storage::read_text(&paths.settings()))
        .map(|text| Settings::from_json(&text))
        .unwrap_or_default();
    let result = run(Renderer::Wgpu, paths.as_ref(), &settings, &launch);
    if result.is_err() {
        run(Renderer::Glow, paths.as_ref(), &settings, &launch)
    } else {
        result
    }
}

struct Launch {
    hidden: bool,
    messages: std::sync::Mutex<Option<std::sync::mpsc::Receiver<String>>>,
    initial: String,
}

fn run(
    renderer: Renderer,
    paths: Option<&Paths>,
    settings: &Settings,
    launch: &Launch,
) -> eframe::Result {
    let options = NativeOptions {
        viewport: viewport(settings).with_visible(!launch.hidden),
        renderer,
        centered: settings.window.is_none(),
        ..NativeOptions::default()
    };
    let paths = paths.cloned();
    let settings = settings.clone();
    let messages = launch.messages.lock().ok().and_then(|mut slot| slot.take());
    let initial = launch.initial.clone();
    eframe::run_native(
        "T-GOW Launcher",
        options,
        Box::new(move |creation| {
            let mut launcher = Launcher::new(&creation.egui_ctx, paths, settings, messages);
            if !initial.is_empty() {
                launcher.open(&initial);
            }
            Ok(Box::new(launcher))
        }),
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
