# Document intake safety bounds (Batch H)

Local document intelligence must reject inputs exceeding configured byte, page,
extracted-text or embedded-object limits before expensive parsing. Encrypted
documents require an explicit user action rather than silent password guessing
or bypass. Limits are configurable by the product owner but must remain finite.

These checks are preflight policy only. The parser must independently enforce
streaming limits, decompression/recursion limits, cancellation and timeouts.
Malformed files remain untrusted; the preflight metric source itself must not
fully parse a hostile document just to calculate metrics. Fuzzing and corpus
tests across supported PDF/image/video formats remain release gates.
