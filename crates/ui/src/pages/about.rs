use egui::{Align2, Rect, Sense, Ui, Vec2, pos2, vec2};
use tgow_core::links::{self, Destination};

use crate::brand;
use crate::shell::{Effect, Shell, View, padded};
use crate::text::Text;
use crate::theme::{self, FONT_LICENSES, Font};
use crate::widgets::{self, Emphasis};

const COPIED_SECONDS: f64 = 2.0;
const DEPENDENCIES: &str = include_str!("../../../../assets/licenses/dependencies.txt");
const LICENSE_FILE: &str = "THIRD-PARTY-LICENSES.txt";
const ROW: f32 = 20.0;

pub(crate) fn show(shell: &mut Shell, ui: &mut Ui, view: &View<'_>, effects: &mut Vec<Effect>) {
    padded(ui, |ui| {
        widgets::heading(ui, view.t(Text::About), None);
        widgets::panel(ui, 16.0, |ui| {
            ui.set_width(ui.available_width().min(820.0));
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(112.0), Sense::hover());
                brand::paint_mark(
                    ui.painter(),
                    Rect::from_center_size(rect.center(), Vec2::splat(112.0)),
                );
                ui.add_space(theme::L);
                ui.vertical(|ui| {
                    ui.add_space(theme::S);
                    widgets::text(ui, view.t(Text::Product), Font::Display, 32.0, theme::TEXT);
                    widgets::text(
                        ui,
                        &format!("{} {}", view.t(Text::Version), view.version),
                        Font::Mono,
                        13.0,
                        theme::ACCENT_SOFT,
                    );
                    ui.add_space(theme::S);
                    widgets::text(ui, view.t(Text::MadeBy), Font::Strong, 16.0, theme::TEXT);
                });
            });
            ui.add_space(theme::L);
            ui.horizontal_wrapped(|ui| {
                let mut any_link = false;
                for (destination, label) in [
                    (Destination::DiscordCommunity, Text::DiscordCommunity),
                    (Destination::DiscordEditingHub, Text::DiscordEditingHub),
                    (Destination::DiscordServices, Text::DiscordServices),
                    (Destination::Website, Text::Website),
                    (Destination::GitHub, Text::GitHub),
                    (Destination::Repository, Text::Repository),
                ] {
                    let Some(url) = destination.url() else {
                        continue;
                    };
                    any_link |= destination.is_discord() || destination == Destination::Website;
                    if widgets::button(ui, view.t(label), Emphasis::Secondary, true).clicked() {
                        effects.push(Effect::Open(url.to_owned()));
                    }
                }
                if !any_link {
                    ui.add_space(theme::S);
                    widgets::text(
                        ui,
                        view.t(Text::LinksPending),
                        Font::Body,
                        13.0,
                        theme::TEXT_FAINT,
                    );
                }
            });
            ui.add_space(theme::L);
            widgets::text(ui, view.t(Text::Promise), Font::Strong, 15.0, theme::TEXT);
            widgets::paragraph(ui, view.t(Text::PrivacyNote), theme::TEXT_MUTED);
            ui.add_space(theme::L);
            widgets::text(
                ui,
                view.t(Text::License),
                Font::Body,
                14.0,
                theme::TEXT_MUTED,
            );
            egui::CollapsingHeader::new(view.t(Text::Fonts))
                .id_salt("fonts")
                .show(ui, |ui| {
                    for (name, license) in FONT_LICENSES {
                        widgets::text(ui, name, Font::Strong, 14.0, theme::TEXT);
                        let first = license.lines().next().unwrap_or_default();
                        widgets::text(ui, first, Font::Mono, 11.0, theme::TEXT_FAINT);
                    }
                });
            components(ui, view, effects);
            ui.add_space(theme::M);
            footer(shell, ui, view, effects);
        });
    });
}

fn components(ui: &mut Ui, view: &View<'_>, effects: &mut Vec<Effect>) {
    let count = DEPENDENCIES.lines().count();
    egui::CollapsingHeader::new(format!("{} ({count})", view.t(Text::Components)))
        .id_salt("components")
        .show(ui, |ui| {
            if widgets::button(ui, view.t(Text::LicenseTexts), Emphasis::Secondary, true).clicked()
            {
                effects.push(Effect::Open(format!(
                    "{}/blob/v{}/{LICENSE_FILE}",
                    links::REPOSITORY,
                    view.version
                )));
            }
            ui.add_space(theme::S);
            egui::ScrollArea::vertical()
                .max_height(260.0)
                .auto_shrink([false, true])
                .show_rows(ui, ROW, count, |ui, range| {
                    for line in DEPENDENCIES.lines().skip(range.start).take(range.len()) {
                        let (rect, _) =
                            ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
                        let mut fields = line.split('\t');
                        for (offset, font, color) in [
                            (0.0, Font::Body, theme::TEXT),
                            (250.0, Font::Mono, theme::TEXT_FAINT),
                            (430.0, Font::Mono, theme::TEXT_MUTED),
                        ] {
                            ui.painter().text(
                                pos2(rect.left() + offset, rect.center().y),
                                Align2::LEFT_CENTER,
                                fields.next().unwrap_or_default(),
                                font.id(12.5),
                                color,
                            );
                        }
                    }
                });
        });
}

fn footer(shell: &mut Shell, ui: &mut Ui, view: &View<'_>, effects: &mut Vec<Effect>) {
    let info = system_info(view);
    ui.horizontal(|ui| {
        widgets::text(
            ui,
            &format!("{} · {}", view.t(Text::Build), target()),
            Font::Mono,
            12.0,
            theme::TEXT_FAINT,
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let now = ui.input(|input| input.time);
            let copied = shell.copied_at.is_some_and(|at| now - at < COPIED_SECONDS);
            let label = if copied {
                Text::Copied
            } else {
                Text::CopySystemInfo
            };
            if widgets::button(ui, view.t(label), Emphasis::Secondary, true).clicked() {
                shell.copied_at = Some(now);
                effects.push(Effect::Copy(info));
            }
        });
    });
}

fn target() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn system_info(view: &View<'_>) -> String {
    format!(
        "T-GOW Launcher {}\n{}\nBuild: {}",
        view.version,
        view.platform,
        target()
    )
}
