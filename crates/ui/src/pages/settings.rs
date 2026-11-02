use egui::Ui;
use tgow_core::settings::{CHECK_INTERVALS, LanguageChoice, Motion, Settings, UpdateMode};

use crate::icons::Icon;
use crate::shell::{Effect, Shell, View, padded};
use crate::text::Text;
use crate::theme;
use crate::widgets;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Section {
    #[default]
    General,
    Updates,
    Downloads,
    Notifications,
    Accessibility,
}

pub(crate) fn show(
    shell: &mut Shell,
    ui: &mut Ui,
    view: &View<'_>,
    settings: &mut Settings,
    effects: &mut Vec<Effect>,
) {
    padded(ui, |ui| {
        widgets::heading(ui, view.t(Text::Settings), None);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(230.0);
                ui.spacing_mut().item_spacing.y = 2.0;
                for (section, icon, label) in [
                    (Section::General, Icon::Settings, Text::SettingsGeneral),
                    (Section::Updates, Icon::Refresh, Text::SettingsUpdates),
                    (Section::Downloads, Icon::Downloads, Text::SettingsDownloads),
                    (
                        Section::Notifications,
                        Icon::Chat,
                        Text::SettingsNotifications,
                    ),
                    (
                        Section::Accessibility,
                        Icon::Discover,
                        Text::SettingsAccessibility,
                    ),
                ] {
                    if widgets::nav_item(
                        ui,
                        icon,
                        view.t(label),
                        shell.settings_page == section,
                        1.0,
                    )
                    .clicked()
                    {
                        shell.settings_page = section;
                    }
                }
            });
            ui.add_space(theme::L);
            ui.vertical(|ui| {
                widgets::panel(ui, 16.0, |ui| {
                    ui.set_width(ui.available_width().min(760.0));
                    let changed = match shell.settings_page {
                        Section::General => general(ui, view, settings),
                        Section::Updates => updates(ui, view, settings),
                        Section::Downloads => downloads(ui, view, settings),
                        Section::Notifications => notifications(ui, view, settings),
                        Section::Accessibility => accessibility(ui, view, settings),
                    };
                    if changed {
                        effects.push(Effect::SettingsChanged);
                    }
                });
            });
        });
    });
}

fn general(ui: &mut Ui, view: &View<'_>, settings: &mut Settings) -> bool {
    let mut changed = false;
    widgets::setting(ui, view.t(Text::Language), None, |ui| {
        changed |= widgets::segmented(
            ui,
            &mut settings.language,
            &[
                (LanguageChoice::System, view.t(Text::LanguageSystem)),
                (LanguageChoice::German, "Deutsch"),
                (LanguageChoice::English, "English"),
            ],
        )
        .changed();
    });
    widgets::setting(
        ui,
        view.t(Text::CloseToTray),
        Some(view.t(Text::CloseToTrayHint)),
        |ui| {
            changed |= widgets::toggle(ui, &mut settings.close_to_tray, view.t(Text::CloseToTray))
                .changed();
        },
    );
    widgets::setting(
        ui,
        view.t(Text::LaunchAtLogin),
        Some(view.t(Text::LaunchAtLoginHint)),
        |ui| {
            changed |= widgets::toggle(
                ui,
                &mut settings.launch_at_login,
                view.t(Text::LaunchAtLogin),
            )
            .changed();
        },
    );
    changed
}

fn update_modes(view: &View<'_>) -> [(UpdateMode, &'static str); 3] {
    [
        (UpdateMode::Automatic, view.t(Text::UpdateAutomatic)),
        (UpdateMode::Notify, view.t(Text::UpdateNotify)),
        (UpdateMode::Off, view.t(Text::UpdateOff)),
    ]
}

fn updates(ui: &mut Ui, view: &View<'_>, settings: &mut Settings) -> bool {
    let mut changed = false;
    widgets::setting(ui, view.t(Text::LauncherUpdates), None, |ui| {
        changed |=
            widgets::segmented(ui, &mut settings.launcher_updates, &update_modes(view)).changed();
    });
    widgets::setting(ui, view.t(Text::AppUpdates), None, |ui| {
        changed |= widgets::segmented(ui, &mut settings.app_updates, &update_modes(view)).changed();
    });
    let labels: Vec<String> = CHECK_INTERVALS
        .iter()
        .map(|minutes| format!("{minutes} {}", view.t(Text::Minutes)))
        .collect();
    let options: Vec<(u32, &str)> = CHECK_INTERVALS
        .iter()
        .zip(&labels)
        .map(|(minutes, label)| (*minutes, label.as_str()))
        .collect();
    widgets::setting(ui, view.t(Text::CheckInterval), None, |ui| {
        changed |= widgets::segmented(ui, &mut settings.check_interval_minutes, &options).changed();
    });
    changed
}

fn downloads(ui: &mut Ui, view: &View<'_>, settings: &mut Settings) -> bool {
    let mut changed = false;
    widgets::setting(ui, view.t(Text::ParallelDownloads), None, |ui| {
        changed |= widgets::segmented(
            ui,
            &mut settings.parallel_downloads,
            &[(1, "1"), (2, "2"), (3, "3"), (4, "4")],
        )
        .changed();
    });
    widgets::setting(
        ui,
        view.t(Text::SystemPackages),
        Some(view.t(Text::SystemPackagesHint)),
        |ui| {
            changed |= widgets::toggle(
                ui,
                &mut settings.system_packages,
                view.t(Text::SystemPackages),
            )
            .changed();
        },
    );
    changed
}

fn notifications(ui: &mut Ui, view: &View<'_>, settings: &mut Settings) -> bool {
    let mut changed = false;
    widgets::setting(
        ui,
        view.t(Text::Notifications),
        Some(view.t(Text::NotificationsHint)),
        |ui| {
            changed |=
                widgets::toggle(ui, &mut settings.notifications, view.t(Text::Notifications))
                    .changed();
        },
    );
    changed
}

fn accessibility(ui: &mut Ui, view: &View<'_>, settings: &mut Settings) -> bool {
    let mut changed = false;
    widgets::setting(ui, view.t(Text::Motion), None, |ui| {
        changed |= widgets::segmented(
            ui,
            &mut settings.motion,
            &[
                (Motion::System, view.t(Text::MotionSystem)),
                (Motion::Reduced, view.t(Text::MotionReduced)),
                (Motion::Full, view.t(Text::MotionFull)),
            ],
        )
        .changed();
    });
    widgets::setting(
        ui,
        view.t(Text::UiSounds),
        Some(view.t(Text::UiSoundsHint)),
        |ui| {
            changed |=
                widgets::toggle(ui, &mut settings.ui_sounds, view.t(Text::UiSounds)).changed();
        },
    );
    changed
}
