use std::fs;
use std::path::Path;

use crate::error::Error;
use crate::report::Finding;

const SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];
pub(crate) const TARGET: &str = "assets/brand/tgow-launcher.ico";

pub(crate) fn write(root: &Path) -> Result<Vec<Finding>, Error> {
    let images: Vec<(u32, Vec<u8>)> = SIZES
        .iter()
        .map(|&size| (size, bitmap(size, &tgow_ui::brand::icon_rgba(size))))
        .collect();
    fs::write(root.join(TARGET), encode(&images))?;
    Ok(Vec::new())
}

fn encode(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let count = u16::try_from(images.len()).unwrap_or(u16::MAX);
    let mut file = Vec::new();
    file.extend(0_u16.to_le_bytes());
    file.extend(1_u16.to_le_bytes());
    file.extend(count.to_le_bytes());
    let mut offset = 6 + 16 * u32::from(count);
    for (size, data) in images {
        let edge = u8::try_from(*size).unwrap_or(0);
        let length = u32::try_from(data.len()).unwrap_or(u32::MAX);
        file.extend([edge, edge, 0, 0]);
        file.extend(1_u16.to_le_bytes());
        file.extend(32_u16.to_le_bytes());
        file.extend(length.to_le_bytes());
        file.extend(offset.to_le_bytes());
        offset += length;
    }
    for (_, data) in images {
        file.extend(data);
    }
    file
}

fn bitmap(size: u32, rgba: &[u8]) -> Vec<u8> {
    let width = usize::try_from(size).unwrap_or(0);
    let mask_stride = width.div_ceil(32) * 4;
    let mut data = Vec::with_capacity(40 + width * width * 4 + mask_stride * width);
    data.extend(40_u32.to_le_bytes());
    data.extend(size.to_le_bytes());
    data.extend((size * 2).to_le_bytes());
    data.extend(1_u16.to_le_bytes());
    data.extend(32_u16.to_le_bytes());
    data.extend([0_u8; 24]);
    for row in (0..width).rev() {
        for column in 0..width {
            let index = (row * width + column) * 4;
            let pixel = rgba.get(index..index + 4).unwrap_or(&[0, 0, 0, 0]);
            data.extend([pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
    }
    data.extend(std::iter::repeat_n(0_u8, mask_stride * width));
    data
}

#[cfg(test)]
mod tests {
    use super::{bitmap, encode};

    #[test]
    fn writes_a_valid_icon_directory() {
        let pixels = vec![255_u8; 16 * 16 * 4];
        let icon = encode(&[(16, bitmap(16, &pixels))]);
        assert_eq!(&icon[..6], &[0, 0, 1, 0, 1, 0]);
        assert_eq!(icon[6], 16);
        let offset = u32::from_le_bytes([icon[18], icon[19], icon[20], icon[21]]);
        assert_eq!(offset, 22);
        assert_eq!(icon.len(), 22 + 40 + 16 * 16 * 4 + 4 * 16);
    }
}
