use std::io::{Read, Write};

const MAX_INPUT_BYTES: usize = 10 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const OUTPUT_TRUNCATED_MARKER: &[u8] = b"UJANI_PDF_OUTPUT_TRUNCATED_V1\n";

/// Extracts text from a validated PDF while enforcing the worker's input-size ceiling.
fn extract_pdf_text(data: &[u8]) -> Result<String, ()> {
    if data.len() > MAX_INPUT_BYTES || !data.starts_with(b"%PDF-") {
        return Err(());
    }
    pdf_extract::extract_text_from_mem(data).map_err(|_| ())
}

/// Reports whether the worker must mark its bounded output as incomplete.
fn output_was_truncated(text: &str) -> bool {
    text.len() > MAX_OUTPUT_BYTES
}

/// Truncates extracted text at the configured byte limit without splitting UTF-8 characters.
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
    let truncated = output_was_truncated(&text);
    if std::io::stdout().write_all(bounded_output(&text).as_bytes()).is_err() {
        std::process::exit(4);
    }
    // Keep stdout as plain extracted text. A fixed metadata-only stderr marker
    // lets the parent distinguish a complete result from a bounded prefix.
    if truncated && std::io::stderr().write_all(OUTPUT_TRUNCATED_MARKER).is_err() {
        std::process::exit(5);
    }
}

#[cfg(test)]
mod tests {
    use super::{bounded_output, extract_pdf_text, MAX_INPUT_BYTES, MAX_OUTPUT_BYTES};

    fn minimal_pdf_with_text() -> Vec<u8> {
        let stream = b"BT /F1 12 Tf 72 200 Td (UJANI worker test) Tj ET\n";
        let mut content = format!("<< /Length {} >>\nstream\n", stream.len()).into_bytes();
        content.extend_from_slice(stream);
        content.extend_from_slice(b"endstream");
        let objects = [
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_vec(),
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
            content,
        ];
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
            pdf.extend_from_slice(object);
            pdf.extend_from_slice(b"\nendobj\n");
        }
        let xref_offset = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
        for offset in offsets {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n", objects.len() + 1).as_bytes());
        pdf
    }

    #[test]
    fn extracts_text_from_a_valid_minimal_pdf() {
        let output = extract_pdf_text(&minimal_pdf_with_text()).unwrap();
        assert!(output.contains("UJANI worker test"));
    }

    #[test]
    fn rejects_input_over_the_worker_limit() {
        let mut input = vec![b'x'; MAX_INPUT_BYTES + 1];
        input[..5].copy_from_slice(b"%PDF-");
        assert!(extract_pdf_text(&input).is_err());
    }

    #[test]
    fn rejects_non_pdf_and_malformed_pdf_bytes() {
        assert!(extract_pdf_text(b"not a pdf").is_err());
        assert!(extract_pdf_text(b"%PDF-1.7\nnot a valid document").is_err());
    }

    #[test]
    fn output_truncation_is_signalled_only_above_the_limit() {
        assert!(!super::output_was_truncated(&"x".repeat(MAX_OUTPUT_BYTES)));
        assert!(super::output_was_truncated(&"x".repeat(MAX_OUTPUT_BYTES + 1)));
    }

    #[test]
    fn output_limit_preserves_utf8_boundaries() {
        let text = format!("{}x", "م".repeat(MAX_OUTPUT_BYTES / 2 + 10));
        let output = bounded_output(&text);
        assert_eq!(output.len(), MAX_OUTPUT_BYTES);
        assert!(text.is_char_boundary(output.len()));
        let expected = "م".repeat(MAX_OUTPUT_BYTES / "م".len());
        assert_eq!(output, expected);
    }
}
