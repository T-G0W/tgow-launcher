use std::sync::Arc;

use egui::{Color32, Context, FontData, FontDefinitions, FontFamily, FontId, Stroke, Visuals};

pub const BG_DEEP: Color32 = Color32::from_rgb(0x05, 0x09, 0x0E);
pub const BG: Color32 = Color32::from_rgb(0x07, 0x0F, 0x16);
pub const SURFACE_0: Color32 = Color32::from_rgb(0x0B, 0x1A, 0x25);
pub const SURFACE_1: Color32 = Color32::from_rgb(0x0F, 0x24, 0x33);
pub const SURFACE_2: Color32 = Color32::from_rgb(0x14, 0x30, 0x3F);
pub const SURFACE_3: Color32 = Color32::from_rgb(0x1A, 0x3B, 0x4F);
pub const GLASS: Color32 = Color32::from_rgba_unmultiplied_const(0x0B, 0x1A, 0x25, 184);
pub const LINE: Color32 = Color32::from_rgb(0x24, 0x48, 0x5E);
pub const LINE_STRONG: Color32 = Color32::from_rgb(0x33, 0x63, 0x7F);
pub const ACCENT: Color32 = Color32::from_rgb(0x4F, 0xB8, 0xFF);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0x8F, 0xD3, 0xFF);
pub const ACCENT_DEEP: Color32 = Color32::from_rgb(0x0B, 0x4F, 0x80);
pub const ON_ACCENT: Color32 = Color32::from_rgb(0x03, 0x12, 0x1D);
pub const TEXT: Color32 = Color32::from_rgb(0xF2, 0xF7, 0xFB);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0xB8, 0xC7, 0xD3);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x7F, 0x93, 0xA3);
pub const POSITIVE: Color32 = Color32::from_rgb(0x3D, 0xDC, 0x97);
pub const WARNING: Color32 = Color32::from_rgb(0xFF, 0xB5, 0x47);
pub const DANGER: Color32 = Color32::from_rgb(0xFF, 0x5D, 0x5D);

pub const XS: f32 = 4.0;
pub const S: f32 = 8.0;
pub const M: f32 = 16.0;
pub const L: f32 = 24.0;
pub const XL: f32 = 32.0;
pub const XXL: f32 = 48.0;

pub const TITLE_BAR: f32 = 44.0;
pub const SIDEBAR_COLLAPSED: f32 = 72.0;
pub const SIDEBAR_EXPANDED: f32 = 220.0;

pub const FAST: f32 = 0.12;
pub const NORMAL: f32 = 0.2;
pub const SLOW: f32 = 0.32;

const DISPLAY: &str = "display";
const HEADING: &str = "heading";
const STRONG: &str = "strong";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    Display,
    Heading,
    Button,
    Label,
    Body,
    Strong,
    Mono,
}

impl Font {
    #[must_use]
    pub fn id(self, size: f32) -> FontId {
        let family = match self {
            Self::Display => FontFamily::Name(DISPLAY.into()),
            Self::Heading | Self::Button => FontFamily::Name(HEADING.into()),
            Self::Label | Self::Strong => FontFamily::Name(STRONG.into()),
            Self::Body => FontFamily::Proportional,
            Self::Mono => FontFamily::Monospace,
        };
        FontId::new(size, family)
    }

    #[must_use]
    pub fn letter_spacing(self, size: f32) -> f32 {
        match self {
            Self::Display => size * 0.02,
            Self::Button => size * 0.08,
            Self::Label => size * 0.1,
            Self::Heading | Self::Body | Self::Strong | Self::Mono => 0.0,
        }
    }

    #[must_use]
    pub fn is_uppercase(self) -> bool {
        matches!(self, Self::Display | Self::Button | Self::Label)
    }
}

const FACES: [(&str, &[u8], &str); 5] = [
    (
        "bai-jamjuree",
        include_bytes!("../../../assets/fonts/BaiJamjuree-Regular.ttf"),
        "",
    ),
    (
        "bai-jamjuree-semibold",
        include_bytes!("../../../assets/fonts/BaiJamjuree-SemiBold.ttf"),
        STRONG,
    ),
    (
        "chakra-petch-semibold",
        include_bytes!("../../../assets/fonts/ChakraPetch-SemiBold.ttf"),
        HEADING,
    ),
    (
        "chakra-petch-bold",
        include_bytes!("../../../assets/fonts/ChakraPetch-Bold.ttf"),
        DISPLAY,
    ),
    (
        "jetbrains-mono",
        include_bytes!("../../../assets/fonts/JetBrainsMono-Medium.ttf"),
        "mono",
    ),
];

fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::empty();
    for (name, bytes, _) in FACES {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }
    let family = |name: &str| match name {
        "" => FontFamily::Proportional,
        "mono" => FontFamily::Monospace,
        other => FontFamily::Name(other.into()),
    };
    for (name, _, target) in FACES {
        let mut chain = vec![name.to_owned()];
        chain.extend(
            ["bai-jamjuree", "jetbrains-mono"]
                .into_iter()
                .filter(|fallback| *fallback != name)
                .map(str::to_owned),
        );
        fonts.families.insert(family(target), chain);
    }
    fonts
}

pub fn install(ctx: &Context) {
    ctx.set_fonts(fonts());
    ctx.set_visuals(visuals());
    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(S, S);
        style.spacing.button_padding = egui::vec2(M, S);
        style.spacing.scroll.floating = true;
        style.spacing.scroll.bar_width = 6.0;
        style.interaction.selectable_labels = false;
        style
            .text_styles
            .insert(egui::TextStyle::Body, Font::Body.id(15.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, Font::Button.id(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, Font::Heading.id(22.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, Font::Body.id(12.0));
        style
            .text_styles
            .insert(egui::TextStyle::Monospace, Font::Mono.id(13.0));
    });
}

fn visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(TEXT);
    visuals.panel_fill = BG;
    visuals.window_fill = SURFACE_1;
    visuals.extreme_bg_color = SURFACE_0;
    visuals.faint_bg_color = SURFACE_1;
    visuals.window_stroke = Stroke::new(1.0, LINE);
    visuals.selection.bg_fill = ACCENT_DEEP;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.hyperlink_color = ACCENT_SOFT;
    visuals.text_cursor.stroke = Stroke::new(2.0, ACCENT);
    for (widget, fill) in [
        (&mut visuals.widgets.noninteractive, SURFACE_1),
        (&mut visuals.widgets.inactive, SURFACE_1),
        (&mut visuals.widgets.hovered, SURFACE_2),
        (&mut visuals.widgets.active, SURFACE_3),
        (&mut visuals.widgets.open, SURFACE_2),
    ] {
        widget.bg_fill = fill;
        widget.weak_bg_fill = fill;
        widget.bg_stroke = Stroke::new(1.0, LINE);
        widget.fg_stroke = Stroke::new(1.0, TEXT);
        widget.corner_radius = egui::CornerRadius::same(2);
    }
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, LINE_STRONG);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals
}

pub const FONT_LICENSES: [(&str, &str); 3] = [
    (
        "Chakra Petch",
        include_str!("../../../assets/fonts/OFL-ChakraPetch.txt"),
    ),
    (
        "Bai Jamjuree",
        include_str!("../../../assets/fonts/OFL-BaiJamjuree.txt"),
    ),
    (
        "JetBrains Mono",
        include_str!("../../../assets/fonts/OFL-JetBrainsMono.txt"),
    ),
];
