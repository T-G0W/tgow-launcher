use egui::{Sense, Stroke, Ui, vec2};

use super::accent_of;
use crate::icons::Icon;
use crate::shape;
use crate::shell::{Page, Shell, View, padded};
use crate::text::Text;
use crate::theme::{self, Font};
use crate::widgets::{self, Emphasis};

const PREVIEW_LINES: usize = 6;

pub(crate) fn show(shell: &mut Shell, ui: &mut Ui, view: &View<'_>) {
    padded(ui, |ui| {
        widgets::heading(ui, view.t(Text::News), None);
        let mut releases: Vec<_> = view
            .apps
            .iter()
            .filter_map(|app| app.manifest().map(|manifest| (app, manifest)))
            .collect();
        if releases.is_empty() {
            widgets::empty_state(
                ui,
                Icon::News,
                view.t(Text::NewsEmptyTitle),
                view.t(Text::NewsEmptyBody),
            );
            return;
        }
        releases.sort_by(|(_, a), (_, b)| {
            b.published
                .cmp(&a.published)
                .then_with(|| a.name.cmp(&b.name))
        });
        for (app, manifest) in releases {
            let accent = accent_of(app);
            ui.horizontal_top(|ui| {
                let (rail, _) = ui.allocate_exact_size(vec2(24.0, 120.0), Sense::hover());
                ui.painter().vline(
                    rail.center().x,
                    rail.y_range(),
                    Stroke::new(1.0, theme::LINE),
                );
                shape::fill(
                    ui.painter(),
                    egui::Rect::from_center_size(
                        egui::pos2(rail.center().x, rail.top() + 22.0),
                        vec2(12.0, 12.0),
                    ),
                    3.0,
                    accent,
                    Stroke::NONE,
                );
                ui.vertical(|ui| {
                    widgets::panel(ui, 12.0, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            widgets::text(ui, &manifest.name, Font::Heading, 19.0, theme::TEXT);
                            widgets::chip(ui, &format!("v{}", manifest.version), accent);
                            if let Some(published) = &manifest.published {
                                widgets::text(
                                    ui,
                                    published.get(..10).unwrap_or(published),
                                    Font::Mono,
                                    12.0,
                                    theme::TEXT_FAINT,
                                );
                            }
                        });
                        let notes = manifest.notes.get(view.language).unwrap_or_default();
                        let preview: Vec<&str> = notes
                            .lines()
                            .filter(|line| !line.trim().is_empty())
                            .take(PREVIEW_LINES)
                            .collect();
                        for line in preview {
                            let clean = line
                                .trim()
                                .trim_start_matches(['-', '*', '#', ' '])
                                .replace("**", "");
                            widgets::paragraph(ui, &clean, theme::TEXT_MUTED);
                        }
                        ui.add_space(theme::XS);
                        if widgets::button(
                            ui,
                            view.t(Text::ReleaseNotes),
                            Emphasis::Secondary,
                            true,
                        )
                        .clicked()
                        {
                            shell.detail.tab = super::app::Tab::Notes;
                            shell.page = Page::App(app.id.clone());
                        }
                    });
                    ui.add_space(theme::M);
                });
            });
        }
    });
}
