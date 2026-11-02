use egui::{Align2, Rect, Sense, Stroke, Ui, Vec2, ViewportCommand, pos2, vec2};
use tgow_core::settings::Language;

use crate::icons::Icon;
use crate::starfield::Starfield;
use crate::text::Text;
use crate::theme::{self, Font};
use crate::widgets;
use crate::{brand, shape};

pub const SIZE: [f32; 2] = [560.0, 340.0];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Starting,
    Checking,
    Downloading(String),
    Installing(String),
    Restarting,
    UpToDate,
    Offline,
}

#[derive(Debug, Clone)]
pub struct Splash<'a> {
    pub language: Language,
    pub version: &'a str,
    pub step: Step,
    pub progress: Option<f32>,
}

#[derive(Default)]
pub struct Screen {
    starfield: Starfield,
}

impl Screen {
    pub fn show(&mut self, ui: &mut Ui, splash: &Splash<'_>) -> bool {
        let rect = ui.max_rect();
        let time = ui.input(|input| input.time);
        let ambient = shape::seconds(time);
        let animated = !widgets::reduced_motion(ui.ctx());
        let drag = ui.interact(
            rect,
            ui.scope_id().with("splash-drag"),
            Sense::click_and_drag(),
        );
        if drag.drag_started() {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, theme::BG);
        self.starfield
            .paint(painter, rect, if animated { ambient } else { 0.0 });
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, theme::LINE),
            egui::StrokeKind::Inside,
        );
        let pulse = if animated {
            0.5 + 0.5 * (ambient * 2.2).sin()
        } else {
            0.5
        };
        let center = pos2(rect.center().x, rect.top() + rect.height() * 0.36);
        let mark = Rect::from_center_size(center, Vec2::splat(96.0 + 4.0 * pulse));
        shape::glow(
            painter,
            mark.shrink(8.0),
            theme::RADIUS_L,
            theme::ACCENT.gamma_multiply(0.18 + 0.14 * pulse),
            40,
        );
        brand::paint_mark(painter, mark);
        painter.text(
            pos2(center.x, mark.bottom() + 22.0),
            Align2::CENTER_TOP,
            text(splash, Text::Product).to_uppercase(),
            Font::Display.id(24.0),
            theme::TEXT,
        );
        let status_y = mark.bottom() + 66.0;
        painter.text(
            pos2(center.x, status_y),
            Align2::CENTER_TOP,
            status(splash),
            Font::Body.id(14.0),
            theme::TEXT_MUTED,
        );
        let bar = Rect::from_center_size(pos2(center.x, status_y + 36.0), vec2(280.0, 4.0));
        painter.rect_filled(bar, 2.0, theme::SURFACE_1);
        let filled = if let Some(fraction) = splash.progress {
            Rect::from_min_size(
                bar.min,
                vec2(bar.width() * fraction.clamp(0.0, 1.0), bar.height()),
            )
        } else {
            let span = bar.width() * 0.3;
            let travel = if animated {
                (ambient * 0.8).fract()
            } else {
                0.35
            };
            let left = bar.left() - span + (bar.width() + span) * travel;
            Rect::from_min_max(
                pos2(left.max(bar.left()), bar.top()),
                pos2((left + span).min(bar.right()), bar.bottom()),
            )
        };
        if filled.width() > 0.0 {
            painter.rect_filled(filled, 2.0, theme::ACCENT);
        }
        painter.text(
            pos2(rect.left() + 16.0, rect.bottom() - 14.0),
            Align2::LEFT_BOTTOM,
            format!("v{}", splash.version),
            Font::Mono.id(11.0),
            theme::TEXT_FAINT,
        );
        painter.text(
            pos2(rect.right() - 16.0, rect.bottom() - 14.0),
            Align2::RIGHT_BOTTOM,
            "T-GOW",
            Font::Label.id(11.0),
            theme::TEXT_FAINT,
        );
        let close = Rect::from_min_size(pos2(rect.right() - 46.0, rect.top()), vec2(46.0, 40.0));
        let quit = ui
            .scope_builder(egui::UiBuilder::new().max_rect(close), |ui| {
                widgets::window_button(ui, Icon::Close, text(splash, Text::Close), true).clicked()
            })
            .inner;
        if animated {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(16));
        }
        quit
    }
}

fn text(splash: &Splash<'_>, key: Text) -> &'static str {
    key.get(splash.language)
}

fn status(splash: &Splash<'_>) -> String {
    match &splash.step {
        Step::Starting => text(splash, Text::SplashStarting).to_owned(),
        Step::Checking => text(splash, Text::SplashChecking).to_owned(),
        Step::Downloading(version) => {
            format!("{} · v{version}", text(splash, Text::SplashDownloading))
        }
        Step::Installing(version) => {
            format!("{} · v{version}", text(splash, Text::SplashInstalling))
        }
        Step::Restarting => text(splash, Text::SplashRestarting).to_owned(),
        Step::UpToDate => text(splash, Text::SplashUpToDate).to_owned(),
        Step::Offline => text(splash, Text::SplashOffline).to_owned(),
    }
}
