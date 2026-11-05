//! Basic image-header inspection from user-supplied bytes; no filesystem access.
use serde::{Deserialize, Serialize};

const MAX_IMAGE_BYTES: usize = 64 * 1024;
const MAX_ORIGINAL_IMAGE_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImageInfo {
    pub file_name: String,
    pub format: String,
    pub size_bytes: u64,
    pub dimensions: Option<(u32, u32)>,
    pub description: String,
}

pub fn inspect_image(file_name: &str, data: &[u8], original_size_bytes: u64) -> Result<ImageInfo, String> {
    if file_name.trim().is_empty() || file_name.len() > 255 { return Err("A valid image file name is required.".into()); }
    if data.is_empty() || data.len() > MAX_IMAGE_BYTES || original_size_bytes == 0 || original_size_bytes > MAX_ORIGINAL_IMAGE_BYTES || data.len() as u64 > original_size_bytes { return Err("Image must be 1 byte–20 MiB; only its first 64 KiB header is processed.".into()); }
    let ext = file_name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let (format, dimensions) = match ext.as_str() {
        "png" => {
            if data.len() < 24 || &data[..8] != b"\x89PNG\r\n\x1a\n" { return Err("Invalid PNG header.".into()); }
            let w = u32::from_be_bytes(data[16..20].try_into().map_err(|_| "Invalid PNG width.")?);
            let h = u32::from_be_bytes(data[20..24].try_into().map_err(|_| "Invalid PNG height.")?);
            if w == 0 || h == 0 { return Err("Invalid PNG dimensions.".into()); }
            ("PNG", Some((w, h)))
        }
        "jpg" | "jpeg" => {
            if data.len() < 3 || data[..2] != [0xff, 0xd8] { return Err("Invalid JPEG header.".into()); }
            ("JPEG", None)
        }
        "gif" => {
            if data.len() < 10 || !(&data[..6] == b"GIF87a" || &data[..6] == b"GIF89a") { return Err("Invalid GIF header.".into()); }
            let w = u16::from_le_bytes([data[6], data[7]]) as u32;
            let h = u16::from_le_bytes([data[8], data[9]]) as u32;
            ("GIF", Some((w, h)))
        }
        "webp" => {
            if data.len() < 12 || &data[..4] != b"RIFF" || &data[8..12] != b"WEBP" { return Err("Invalid WebP header.".into()); }
            ("WebP", None)
        }
        "bmp" => {
            if data.len() < 26 || &data[..2] != b"BM" { return Err("Invalid BMP header.".into()); }
            let w = i32::from_le_bytes(data[18..22].try_into().map_err(|_| "Invalid BMP width.")?).unsigned_abs();
            let h = i32::from_le_bytes(data[22..26].try_into().map_err(|_| "Invalid BMP height.")?).unsigned_abs();
            ("BMP", Some((w, h)))
        }
        _ => return Err("Unsupported image type. Supported: PNG, JPEG, GIF, WebP and BMP.".into()),
    };
    let dimension_text = dimensions.map(|(w,h)| format!(", {w} × {h} pixels")).unwrap_or_else(|| ", dimensions unavailable from basic header inspection".into());
    Ok(ImageInfo { file_name: file_name.into(), format: format.into(), size_bytes: original_size_bytes, dimensions, description: format!("{format} image, {original_size_bytes} bytes{dimension_text}") })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn reads_png_dimensions_from_header() {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend_from_slice(&[0,0,0,13,b'I',b'H',b'D',b'R']);
        bytes.extend_from_slice(&640u32.to_be_bytes());
        bytes.extend_from_slice(&480u32.to_be_bytes());
        let info = inspect_image("sample.png", &bytes, bytes.len() as u64).unwrap();
        assert_eq!(info.dimensions, Some((640,480)));
    }
    #[test] fn rejects_mismatched_image_headers() {
        assert!(inspect_image("sample.png", b"not png", 7).is_err());
    }
}
