use std::{env, fs, path::PathBuf};

/// Prepares platform assets and skips application context generation for the isolated worker build.
fn main() {
    println!("cargo:rerun-if-env-changed=UJANI_PDF_WORKER_BUILD");
    println!("cargo:rerun-if-changed=build.rs");
    // The isolated worker shares the package manifest but does not need Tauri's
    // application context, icon processing, or bundle validation.
    if env::var_os("UJANI_PDF_WORKER_BUILD").is_some() {
        return;
    }
    // Tauri's context macro expects a source icon even during a development
    // compile. Keep a tiny valid fallback for clean checkouts; a real icon
    // supplied by the project always takes precedence.
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let icon_dir = manifest_dir.join("icons");
    let icon_path = icon_dir.join("icon.png");
    if !icon_path.exists() {
        fs::create_dir_all(&icon_dir).expect("create Tauri icon directory");
        let encoded = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGMQ05b5DwACFQFdXb3nEgAAAABJRU5ErkJggg==";
        use base64::Engine as _;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .expect("decode fallback PNG");
        fs::write(&icon_path, bytes).expect("write fallback Tauri icon");
    }
    println!("cargo:rerun-if-changed=build.rs");
    tauri_build::build()
}
