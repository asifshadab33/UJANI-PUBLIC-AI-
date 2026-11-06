#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() > 10 * 1024 * 1024 || !data.starts_with(b"%PDF-") {
        return;
    }
    let _ = pdf_extract::extract_text_from_mem(data);
});
