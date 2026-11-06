# Parser fuzzing

UJANI Public AI includes optional cargo-fuzz harnesses for the bounded image
header inspector and the PDF text parser. These harnesses are development tools;
they are not included in the desktop runtime and do not imply that fuzzing has
already been completed.

Install a nightly Rust toolchain and cargo-fuzz:

    rustup toolchain install nightly
    cargo install cargo-fuzz

Run the image parser harness:

    cargo +nightly fuzz run image_header -- -max_len=65536

Run the PDF parser harness:

    cargo +nightly fuzz run pdf_text -- -max_len=10485760

Run each harness in an isolated development environment. PDF parsing is
intentionally exercised only for inputs carrying a PDF header and no larger
than the desktop's 10 MiB input limit. Fuzzing is not a replacement for the
dedicated child-process boundary, time budget, output cap, or OS-level resource
limits. Record corpus, duration, crashes and minimized reproductions when
reporting results. Do not describe a parser as fuzz-tested until a run has
actually completed and its corpus/results are retained.
