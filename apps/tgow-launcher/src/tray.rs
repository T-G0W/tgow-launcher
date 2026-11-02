use std::sync::mpsc::{self, Receiver};

use tgow_core::links::Destination;
use tgow_core::settings::Language;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

const ICON_SIZE: u32 = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    Open,
    Check,
    Visit(&'static str),
    Quit,
}

pub(crate) struct Tray {
    _icon: TrayIcon,
    events: Receiver<Action>,
}

impl Tray {
    pub(crate) fn new(ctx: &egui::Context, language: Language) -> Option<Self> {
        let german = language == Language::German;
        let label = |de: &'static str, en: &'static str| if german { de } else { en };
        let menu = Menu::new();
        let mut items = Vec::new();
        let mut add = |text: &str, action: Action| -> Option<()> {
            let item = MenuItem::new(text, true, None);
            menu.append(&item).ok()?;
            items.push((item.id().clone(), action));
            Some(())
        };
        add(label("Öffnen", "Open"), Action::Open)?;
        add(
            label("Nach Updates suchen", "Check for updates"),
            Action::Check,
        )?;
        if let Some(url) = Destination::main_discord() {
            add("Discord", Action::Visit(url))?;
        }
        if let Some(url) = Destination::Website.url() {
            add("Website", Action::Visit(url))?;
        }
        menu.append(&PredefinedMenuItem::separator()).ok()?;
        add(label("Beenden", "Quit"), Action::Quit)?;
        let icon =
            Icon::from_rgba(tgow_ui::brand::icon_rgba(ICON_SIZE), ICON_SIZE, ICON_SIZE).ok()?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip("T-GOW Launcher")
            .with_icon(icon)
            .build()
            .ok()?;
        let (sender, events) = mpsc::channel();
        let lookup = items;
        let menu_sender = sender.clone();
        let menu_ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if let Some((_, action)) = lookup.iter().find(|(id, _)| *id == event.id) {
                let _ = menu_sender.send(action.clone());
                menu_ctx.request_repaint();
            }
        }));
        let click_ctx = ctx.clone();
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = sender.send(Action::Open);
                click_ctx.request_repaint();
            }
        }));
        Some(Self {
            _icon: tray,
            events,
        })
    }

    pub(crate) fn poll(&self) -> Vec<Action> {
        self.events.try_iter().collect()
    }
}
