use std::io::Cursor;

use egui::ColorImage;
use image::{ImageReader, Limits};

const MAX_EDGE: u32 = 4096;
const MAX_ALLOCATION: u64 = 96 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Slot {
    Icon,
    Hero,
}

#[must_use]
pub(crate) fn decode(bytes: &[u8]) -> Option<ColorImage> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_EDGE);
    limits.max_image_height = Some(MAX_EDGE);
    limits.max_alloc = Some(MAX_ALLOCATION);
    reader.limits(limits);
    let rgba = reader.decode().ok()?.to_rgba8();
    let size = [
        usize::try_from(rgba.width()).ok()?,
        usize::try_from(rgba.height()).ok()?,
    ];
    Some(ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn rejects_garbage_and_decodes_a_tiny_png() {
        assert!(decode(b"not an image").is_none());
        let png: [u8; 67] = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        let image = decode(&png).expect("valid png");
        assert_eq!(image.size, [1, 1]);
    }
}
