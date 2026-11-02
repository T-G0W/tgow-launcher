use egui::{Rect, Sense, Ui, Vec2};

use crate::brand;
use crate::shell::{Effect, View};
use crate::text::Text;
use crate::theme::{self, Font};
use crate::widgets::{self, Emphasis};

pub(crate) fn show(ui: &mut Ui, view: &View<'_>, effects: &mut Vec<Effect>) {
    ui.vertical_centered(|ui| {
        ui.add_space((ui.available_height() * 0.16).max(theme::L));
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(150.0), Sense::hover());
        brand::paint_mark(
            ui.painter(),
            Rect::from_center_size(rect.center(), Vec2::splat(150.0)),
        );
        ui.add_space(theme::L);
        widgets::text(ui, view.t(Text::Product), Font::Display, 44.0, theme::TEXT);
        ui.add_space(theme::S);
        let mut body = widgets::job(
            view.t(Text::WelcomeBody),
            Font::Body,
            16.0,
            theme::TEXT_MUTED,
        );
        body.wrap.max_width = 520.0;
        body.halign = egui::Align::Center;
        ui.label(body);
        ui.add_space(theme::XL);
        ui.horizontal(|ui| {
            let width = 470.0;
            ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
            if widgets::button(ui, view.t(Text::Install), Emphasis::Primary, true).clicked() {
                effects.push(Effect::InstallSelf);
            }
            if widgets::button(
                ui,
                view.t(Text::RunWithoutInstall),
                Emphasis::Secondary,
                true,
            )
            .clicked()
            {
                effects.push(Effect::SkipInstall);
            }
        });
        ui.add_space(theme::L);
        widgets::text(
            ui,
            view.t(Text::Promise),
            Font::Body,
            13.0,
            theme::TEXT_FAINT,
        );
    });
}
