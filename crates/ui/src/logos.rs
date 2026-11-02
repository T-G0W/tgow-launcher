use egui::{Color32, ColorImage, Context, Id, Painter, Rect, TextureHandle, TextureOptions, pos2};

use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Logo {
    Discord,
    GitHub,
    Website,
}

impl Logo {
    fn name(self) -> &'static str {
        match self {
            Self::Discord => "tgow.logo.discord",
            Self::GitHub => "tgow.logo.github",
            Self::Website => "tgow.logo.website",
        }
    }

    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Discord => include_bytes!("../../../assets/icons/discord.png"),
            Self::GitHub => include_bytes!("../../../assets/icons/github.png"),
            Self::Website => include_bytes!("../../../assets/icons/website.png"),
        }
    }

    #[must_use]
    pub fn color(self) -> Color32 {
        match self {
            Self::Discord => Color32::from_rgb(0x58, 0x65, 0xF2),
            Self::GitHub => theme::TEXT,
            Self::Website => theme::ACCENT,
        }
    }
}

pub fn paint(painter: &Painter, rect: Rect, logo: Logo, tint: Color32) {
    if let Some(texture) = texture(painter.ctx(), logo) {
        painter.image(
            texture.id(),
            rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            tint,
        );
    }
}

fn texture(ctx: &Context, logo: Logo) -> Option<TextureHandle> {
    let id = Id::unique(logo.name());
    if let Some(texture) = ctx.data(|data| data.get_temp::<TextureHandle>(id)) {
        return Some(texture);
    }
    let decoded = image::load_from_memory_with_format(logo.bytes(), image::ImageFormat::Png)
        .ok()?
        .to_rgba8();
    let size = [
        usize::try_from(decoded.width()).ok()?,
        usize::try_from(decoded.height()).ok()?,
    ];
    let texture = ctx.load_texture(
        logo.name(),
        ColorImage::from_rgba_unmultiplied(size, decoded.as_raw()),
        TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear)),
    );
    ctx.data_mut(|data| data.insert_temp(id, texture.clone()));
    Some(texture)
}

#[cfg(test)]
mod tests {
    use super::Logo;

    #[test]
    fn every_logo_decodes_to_a_square_mask() {
        for logo in [Logo::Discord, Logo::GitHub, Logo::Website] {
            let decoded = image::load_from_memory(logo.bytes())
                .expect("png")
                .to_rgba8();
            assert_eq!(decoded.width(), decoded.height());
            assert!(decoded.pixels().any(|pixel| pixel.0[3] > 200));
            assert!(
                decoded
                    .pixels()
                    .all(|pixel| pixel.0[..3] == [255, 255, 255])
            );
        }
    }
}
