use std::collections::BTreeMap;

use egui::{Key, Modifiers, Role, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use tgow_core::catalog::{App, Repository};
use tgow_core::install::Installation;
use tgow_core::platform::{Arch, Os, Platform};
use tgow_core::settings::{Language, Settings};
use tgow_ui::shell::{Artwork, CatalogStatus, Effect, Finished, Page, Phase, Shell, View};
use tgow_ui::{theme, widgets};

struct Fixture {
    shell: Shell,
    settings: Settings,
    platform: Platform,
    apps: Vec<App>,
    installed: BTreeMap<String, Installation>,
    jobs: BTreeMap<String, Phase>,
    failures: BTreeMap<String, String>,
    artwork: BTreeMap<String, Artwork>,
    recent: Vec<Finished>,
    effects: Vec<Effect>,
    ready: bool,
}

impl Fixture {
    fn new(names: &[&str]) -> Self {
        Self {
            shell: Shell::default(),
            settings: Settings::default(),
            platform: Platform::new(Os::Windows, Arch::X86_64),
            apps: names
                .iter()
                .map(|name| App::from_repository(&repository(name)))
                .collect(),
            installed: BTreeMap::new(),
            jobs: BTreeMap::new(),
            failures: BTreeMap::new(),
            artwork: BTreeMap::new(),
            recent: Vec::new(),
            effects: Vec::new(),
            ready: false,
        }
    }
}

fn repository(name: &str) -> Repository {
    Repository {
        name: name.to_owned(),
        full_name: format!("T-G0W/{name}"),
        description: Some(format!("{name} description")),
        topics: vec!["tgow-launcher".to_owned()],
        archived: false,
        fork: false,
        private: false,
        html_url: format!("https://github.com/T-G0W/{name}"),
        pushed_at: None,
    }
}

fn frame(ui: &mut egui::Ui, fixture: &mut Fixture) {
    if !fixture.ready {
        theme::install(ui.ctx());
        widgets::set_reduced_motion(ui.ctx(), true);
        fixture.ready = true;
    }
    let view = View {
        language: Language::German,
        platform: &fixture.platform,
        apps: &fixture.apps,
        status: CatalogStatus::Ready,
        checked: None,
        version: "0.1.0",
        installed: &fixture.installed,
        jobs: &fixture.jobs,
        failures: &fixture.failures,
        artwork: &fixture.artwork,
        recent: &fixture.recent,
        launcher_latest: None,
    };
    let effects = fixture.shell.show(ui, &view, &mut fixture.settings);
    fixture.effects.extend(effects);
}

fn harness(names: &[&str]) -> Harness<'static, Fixture> {
    let mut harness = Harness::builder()
        .with_size(vec2(1280.0, 760.0))
        .build_ui_state(frame, Fixture::new(names));
    harness.run();
    harness
}

#[test]
fn sidebar_tabs_switch_pages() {
    let mut harness = harness(&[]);
    assert_eq!(harness.state().shell.page, Page::Library);
    harness.get_by_role_and_label(Role::Tab, "Über").click();
    harness.run();
    assert_eq!(harness.state().shell.page, Page::About);
    harness
        .get_by_role_and_label(Role::Tab, "Einstellungen")
        .click();
    harness.run();
    assert_eq!(harness.state().shell.page, Page::Settings);
}

#[test]
fn app_cards_carry_their_name_and_open_the_app_page() {
    let mut harness = harness(&["ferrovox", "neocross"]);
    let prefixes: Vec<String> = harness
        .state()
        .apps
        .iter()
        .map(|app| format!("{}, ", app.title()))
        .collect();
    for prefix in &prefixes {
        harness.get_by_label_contains(prefix);
    }
    harness.get_by_label_contains(&prefixes[0]).click();
    harness.run();
    assert_eq!(harness.state().shell.page, Page::App("ferrovox".to_owned()));
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().shell.page, Page::Library);
}

#[test]
fn check_now_button_and_shortcuts_raise_effects() {
    let mut harness = harness(&[]);
    harness
        .get_by_role_and_label(Role::Button, "Jetzt prüfen")
        .click();
    harness.run();
    harness.key_press(Key::F5);
    harness.run();
    let checks = harness
        .state()
        .effects
        .iter()
        .filter(|effect| **effect == Effect::CheckNow)
        .count();
    assert_eq!(checks, 2);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Comma);
    harness.run();
    assert_eq!(harness.state().shell.page, Page::Settings);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::J);
    harness.run();
    assert_eq!(harness.state().shell.page, Page::Downloads);
}

#[test]
fn list_arrangement_keeps_apps_reachable() {
    let mut harness = harness(&["ferrovox", "neocross"]);
    harness
        .get_by_role_and_label(Role::RadioButton, "Liste")
        .click();
    harness.run();
    let title = harness.state().apps[1].title().to_owned();
    harness.get_by_label_contains(&format!("{title}, ")).click();
    harness.run();
    assert_eq!(harness.state().shell.page, Page::App("neocross".to_owned()));
}

#[test]
fn back_returns_through_the_visited_pages() {
    let mut harness = harness(&[]);
    harness.get_by_role_and_label(Role::Tab, "Über").click();
    harness.run();
    harness
        .get_by_role_and_label(Role::Tab, "Einstellungen")
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Zurück")
        .click();
    harness.run();
    assert_eq!(harness.state().shell.page, Page::About);
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().shell.page, Page::Library);
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Zurück")
            .is_none()
    );
}

#[test]
fn title_bar_opens_and_closes_the_popovers() {
    let mut harness = harness(&[]);
    harness
        .get_by_role_and_label(Role::Button, "Status")
        .click();
    harness.run();
    assert!(harness.query_by_label("Betriebsbereit").is_some());
    assert_eq!(
        harness
            .get_all_by_role_and_label(Role::Button, "Jetzt prüfen")
            .count(),
        2
    );
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.query_by_label("Betriebsbereit").is_none());
    harness
        .get_by_role_and_label(Role::Button, "Downloads")
        .click();
    harness.run();
    assert!(harness.query_by_label("Keine Downloads").is_some());
    harness
        .get_by_role_and_label(Role::Link, "Alle anzeigen")
        .click();
    harness.run();
    assert_eq!(harness.state().shell.page, Page::Downloads);
}

#[test]
fn right_click_on_a_card_opens_its_menu() {
    let mut harness = harness(&["ferrovox"]);
    let title = harness.state().apps[0].title().to_owned();
    harness
        .get_by_label_contains(&format!("{title}, "))
        .click_secondary();
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::MenuItem, "Auf GitHub ansehen")
            .is_some()
    );
    harness
        .get_by_role_and_label(Role::MenuItem, "Details anzeigen")
        .click();
    harness.run();
    assert_eq!(harness.state().shell.page, Page::App("ferrovox".to_owned()));
}
