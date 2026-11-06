fn main() {
    if std::env::args().nth(1).as_deref() == Some("--pdf-extract-worker") {
        ujani_desktop_lib::pdf_worker::run();
        return;
    }
    ujani_desktop_lib::run();
}