use std::io::{Read, Write};

/// Isolated child-process entry point for parsing untrusted PDF bytes.
pub fn run() {
    let mut data = Vec::new();
    if std::io::stdin().take(10 * 1024 * 1024 + 1).read_to_end(&mut data).is_err()
        || data.len() > 10 * 1024 * 1024 {
        std::process::exit(2);
    }
    match pdf_extract::extract_text_from_mem(&data) {
        Ok(text) => {
            let bytes = text.as_bytes();
            let mut end = bytes.len().min(1024 * 1024);
            while !text.is_char_boundary(end) { end -= 1; }
            let _ = std::io::stdout().write_all(&bytes[..end]);
        }
        Err(_) => std::process::exit(3),
    }
}
