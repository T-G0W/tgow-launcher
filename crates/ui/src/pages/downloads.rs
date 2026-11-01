use egui::Ui;

use super::library::phase_label;
use crate::icons::Icon;
use crate::shell::{Effect, Phase, View, padded};
use crate::text::Text;
use crate::theme::{self, Font};
use crate::widgets::{self, Emphasis};

pub(crate) fn show(ui: &mut Ui, view: &View<'_>, effects: &mut Vec<Effect>) {
    padded(ui, |ui| {
        widgets::heading(ui, view.t(Text::Downloads), None);
        if view.jobs.is_empty() && view.failures.is_empty() {
            widgets::empty_state(
                ui,
                Icon::Downloads,
                view.t(Text::DownloadsEmptyTitle),
                view.t(Text::DownloadsEmptyBody),
            );
            return;
        }
        for (app, phase) in view.jobs {
            let title = view.app(app).map_or(app.as_str(), |found| found.title());
            widgets::panel(ui, 12.0, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        widgets::text(ui, title, Font::Heading, 18.0, theme::TEXT);
                        widgets::text(
                            ui,
                            &phase_label(view, *phase),
                            Font::Mono,
                            12.0,
                            theme::ACCENT_SOFT,
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let cancellable =
                            matches!(phase, Phase::Waiting | Phase::Downloading { .. });
                        if cancellable
                            && widgets::button(ui, view.t(Text::Cancel), Emphasis::Secondary, true)
                                .clicked()
                        {
                            effects.push(Effect::Cancel(app.clone()));
                        }
                    });
                });
                ui.add_space(theme::S);
                widgets::progress_bar(ui, phase.fraction());
            });
            ui.add_space(theme::S);
        }
        for (app, message) in view.failures {
            let title = view.app(app).map_or(app.as_str(), |found| found.title());
            widgets::panel(ui, 12.0, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    widgets::text(ui, title, Font::Heading, 18.0, theme::TEXT);
                    widgets::chip(ui, view.t(Text::Failed), theme::DANGER);
                });
                widgets::paragraph(ui, message, theme::TEXT_MUTED);
            });
            ui.add_space(theme::S);
        }
    });
}
