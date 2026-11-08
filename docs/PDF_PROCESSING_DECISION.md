# UJANI document processing decision

## Product principle

UJANI should not recreate an operating-system capability or build a document engine when a maintained local component can do the job. AI is used for understanding and fulfilling the user's request; deterministic document tooling is used to decode and extract source material.

## PDF pipeline

1. The existing PDF parser extracts selectable text locally. It is not an AI model and does not send documents to a service.
2. A small dedicated worker isolates parsing from the desktop process. The worker accepts one bounded PDF over standard input, has no application IPC surface, and returns bounded UTF-8 text over standard output.
3. The existing local Ollama model performs document analysis, summarization and optional translation. The PDF parser does not make those decisions.
4. Scanned-page OCR is a separate capability and must be reported as unsupported until an approved local OCR engine is integrated and tested.

## Why retain a worker

PDFs are complex, user-supplied binary inputs. Keeping parsing out of the main UI process reduces the impact of parser crashes, malformed files and resource exhaustion. The worker is a reliability/security boundary, not a reason to grow a bespoke PDF subsystem.

## Parser selection gate

Do not switch parser libraries based on one benchmark or marketing claim. Evaluate candidate maintained local parsers (including PDFOxide, PDF.js and PDFium) against a representative, privacy-safe corpus of UJANI test documents. Compare extraction correctness, malformed-file handling, tables/reading order, memory/CPU, binary/install size, supported architectures, license obligations and maintenance maturity. Keep the smallest reliable local parser that passes the corpus. A model may summarize extracted text; it must not be asked to guess missing or unparsed PDF content.

## Non-goals

- No cloud upload as a PDF parsing fallback.
- No new AI model solely to extract selectable PDF text.
- No OCR claim until a local OCR engine is installed, permissioned and validated.
- No parser-specific business logic duplicated in the desktop UI.
