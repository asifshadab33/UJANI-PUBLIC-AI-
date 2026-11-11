# External assistance consent gate (Batch G)

Local-first execution remains the default. An external AI request is denied unless
the user has enabled external assistance, approved the particular task, the
payload has been minimized, and a destination is configured. Local file content
requires explicit selection; general permission to use external assistance does
not authorize uploading files.

This module is a policy primitive, not a network transport or data-loss prevention
system. The caller must apply it immediately before transmission, display the
destination and payload summary, honor cancellation, and record a privacy-safe
receipt. No silent fallback to cloud is permitted. Integration tests must verify
the actual transport path and prove that denied requests send zero bytes.
