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
            if data.len() < 24 || &data[..8] != b"\x89PNG\r\n\x1a\n" || &data[12..16] != b"IHDR" { return Err("Invalid PNG header or IHDR chunk.".into()); }
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
            if w == 0 || h == 0 { return Err("Invalid GIF dimensions.".into()); }
            ("GIF", Some((w, h)))
        }
        "webp" => {
            if data.len() < 12 || &data[..4] != "RIFF".as_bytes() || &data[8..12] != "WEBP".as_bytes() { return Err("Invalid WebP header.".into()); }
            ("WebP", None)
        }
        "bmp" => {
            // Only parse Windows BITMAPINFOHEADER-compatible DIB headers; OS/2 12-byte headers use different offsets.
            let dib_header_size = u32::from_le_bytes(data[14..18].try_into().map_err(|_| "Invalid BMP DIB header size.")?);
            if dib_header_size < 40 || dib_header_size as usize > data.len().saturating_sub(14) {
                return Err("Unsupported or incomplete BMP DIB header.".into());
            }
            if data.len() < 26 || &data[..2] != b"BM" { return Err("Invalid BMP header.".into()); }
            let w = i32::from_le_bytes(data[18..22].try_into().map_err(|_| "Invalid BMP width.")?).unsigned_abs();
            let h = i32::from_le_bytes(data[22..26].try_into().map_err(|_| "Invalid BMP height.")?).unsigned_abs();
            if w == 0 || h == 0 { return Err("Invalid BMP dimensions.".into()); }
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

    #[test]
    fn reads_png_dimensions_from_header() {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend_from_slice(&[0,0,0,13,b'I',b'H',b'D',b'R']);
        bytes.extend_from_slice(&640u32.to_be_bytes());
        bytes.extend_from_slice(&480u32.to_be_bytes());
        let info = inspect_image("sample.png", &bytes, bytes.len() as u64).unwrap();
        assert_eq!(info.dimensions, Some((640,480)));
    }

    #[test]
    fn rejects_mismatched_image_headers() {
        assert!(inspect_image("sample.png", b"not png", 7).is_err());
    }

    #[test]
    fn rejects_zero_dimensions_and_size_mismatches() {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend_from_slice(&[0,0,0,13,b'I',b'H',b'D',b'R']);
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(&10_u32.to_be_bytes());
        assert!(inspect_image("sample.png", &bytes, bytes.len() as u64).is_err());
        assert!(inspect_image("sample.png", &bytes, 1).is_err());
    }

    #[test]
    fn rejects_unsupported_and_oversized_headers() {
        assert!(inspect_image("sample.svg", b"<svg/>", 6).is_err());
        assert!(inspect_image("sample.png", &vec![0; MAX_IMAGE_BYTES + 1], (MAX_IMAGE_BYTES + 1) as u64).is_err());
    }
}


#[cfg(test)]
mod additional_format_tests {
    use super::*;

    #[test]
    fn inspects_jpeg_gif_webp_and_bmp_headers() {
        let jpeg = inspect_image("photo.jpg", &[0xff, 0xd8, 0xff], 3).unwrap();
        assert_eq!(jpeg.format, "JPEG");
        assert_eq!(jpeg.dimensions, None);

        let gif = inspect_image("animation.gif", b"GIF89a\x20\x00\x10\x00", 10).unwrap();
        assert_eq!(gif.format, "GIF");
        assert_eq!(gif.dimensions, Some((32, 16)));

        let webp = inspect_image("picture.webp", b"RIFF\x04\x00\x00\x00WEBP", 12).unwrap();
        assert_eq!(webp.format, "WebP");

        let mut bmp = vec![0_u8; 26];
        bmp[..2].copy_from_slice(b"BM");
        bmp[18..22].copy_from_slice(&32_i32.to_le_bytes());
        bmp[22..26].copy_from_slice(&16_i32.to_le_bytes());
        let bmp_info = inspect_image("bitmap.bmp", &bmp, 26).unwrap();
        assert_eq!(bmp_info.dimensions, Some((32, 16)));
    }

    #[test]
    fn rejects_png_with_wrong_first_chunk_and_os2_bmp_header() {
        let mut png = b"\\x89PNG\\r\\n\\x1a\\n".to_vec();
        png.extend_from_slice(&[0, 0, 0, 13, b'B', b'A', b'D', b'!']);
        png.extend_from_slice(&1_u32.to_be_bytes());
        png.extend_from_slice(&1_u32.to_be_bytes());
        assert!(inspect_image("bad.png", &png, png.len() as u64).is_err());

        let mut bmp = vec![0_u8; 26];
        bmp[..2].copy_from_slice(b"BM");
        bmp[14..18].copy_from_slice(&12_u32.to_le_bytes());
        bmp[18..22].copy_from_slice(&32_i32.to_le_bytes());
        bmp[22..26].copy_from_slice(&16_i32.to_le_bytes());
        assert!(inspect_image("os2.bmp", &bmp, bmp.len() as u64).is_err());
    }

    #[test]
    fn rejects_extension_and_header_mismatch_for_other_formats() {
        assert!(inspect_image("photo.jpg", b"GIF89a\x01\x00\x01\x00", 10).is_err());
        assert!(inspect_image("picture.webp", b"RIFF\x00\x00\x00\x00NOPE", 12).is_err());
        assert!(inspect_image("bitmap.bmp", b"not a bitmap header at all", 26).is_err());
    }
}
