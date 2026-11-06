#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() > 64 * 1024 {
        return;
    }
    let size = data.len() as u64;
    let _ = ujani_core::inspect_image("fuzz.png", data, size);
    let _ = ujani_core::inspect_image("fuzz.jpg", data, size);
    let _ = ujani_core::inspect_image("fuzz.gif", data, size);
    let _ = ujani_core::inspect_image("fuzz.webp", data, size);
    let _ = ujani_core::inspect_image("fuzz.bmp", data, size);
});
