use std::io::{Read, Write};

const MAX_INPUT_BYTES: usize = 10 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;

fn extract_pdf_text(data: &[u8]) -> Result<String, ()> {
    if data.len() > MAX_INPUT_BYTES || !data.starts_with(b"%PDF-") {
        return Err(());
    }
    pdf_extract::extract_text_from_mem(data).map_err(|_| ())
}

fn bounded_output(text: &str) -> &str {
    let mut end = text.len().min(MAX_OUTPUT_BYTES);
    while !text.is_char_boundary(end) { end -= 1; }
    &text[..end]
}

/// Dedicated sidecar entry point. It reads one bounded PDF from stdin and writes
/// only bounded UTF-8 text to stdout; diagnostics never contain document content.
pub fn run() {
    let mut data = Vec::new();
    if std::io::stdin().take((MAX_INPUT_BYTES + 1) as u64).read_to_end(&mut data).is_err()
        || data.len() > MAX_INPUT_BYTES {
        std::process::exit(2);
    }
    let text = match extract_pdf_text(&data) {
        Ok(text) => text,
        Err(()) => std::process::exit(3),
    };
    if std::io::stdout().write_all(bounded_output(&text).as_bytes()).is_err() {
        std::process::exit(4);
    }
}

#[cfg(test)]
mod tests {
    use super::{bounded_output, extract_pdf_text, MAX_INPUT_BYTES, MAX_OUTPUT_BYTES};

    #[test]
    fn rejects_input_over_the_worker_limit() {
        let mut input = vec![b'x'; MAX_INPUT_BYTES + 1];
        input[..5].copy_from_slice(b"%PDF-");
        assert!(extract_pdf_text(&input).is_err());
    }

    #[test]
    fn rejects_non_pdf_and_malformed_pdf_bytes() {
        assert!(extract_pdf_text(b"not a pdf").is_err());
        assert!(extract_pdf_text(b"%PDF-1.7\\nnot a valid document").is_err());
    }

    #[test]
    fn output_limit_preserves_utf8_boundaries() {
        let text = format!("{}x", "م".repeat(MAX_OUTPUT_BYTES / 2 + 10));
        let output = bounded_output(&text);
        assert!(output.len() <= MAX_OUTPUT_BYTES);
        assert!(text.is_char_boundary(output.len()));
    }
}
