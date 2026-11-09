# Everyday Assistant and Open Application Interoperability

**Status:** approved product direction and implementation contract; this document does not claim every listed task or integration is implemented.

## Product identity

UJANI Public AI is an everyday computer assistant, not a replacement for the user's operating system, office suite, email client, browser or preferred AI provider. It should make existing software and device resources more useful while keeping local processing the default.

The core promise is: **local when possible, connected when useful, open to the applications the user already chooses.**

## Task-first priorities

Prioritise high-frequency user outcomes over file-format-specific subsystems.

### Everyday writing
- Correct grammar, spelling, punctuation and clarity.
- Rewrite for professional, friendly, concise or formal tone.
- Compose an email from a short instruction and user-provided facts.
- Draft replies and subject lines from text the user explicitly supplies.
- Translate text when a suitable offline language capability is installed.
- Explain, simplify or improve a selected paragraph.

### Reading and understanding
- Summarise pasted or explicitly selected text.
- Explain difficult paragraphs and terminology.
- Extract key points, action items and dates.
- Compare two user-selected texts or documents.
- Create meeting notes and structured outlines.

### Practical computer assistance
- Find files using native OS indexes where available.
- Find duplicates with local metadata and hashing.
- Identify large files using metadata only.
- Open files in the user's default application.
- Use native file pickers, clipboard, notifications and scheduling facilities where appropriate and permissioned.

PDF remains a supported document format, but it is not the product's centre of gravity. Keep essential parser safety, size limits, cancellation and regression coverage. Do not add advanced PDF-only subsystems unless user evidence justifies their maintenance and performance cost. Prefer a small number of reusable text/document adapters over format-specific product silos.

## Execution selection

The task router should select the lightest suitable mechanism:

1. OS capability or existing local service for file discovery, opening, clipboard, clock, notifications and scheduling.
2. Deterministic local code for hashing, arithmetic, validation, formatting and exact transformations.
3. An already-installed local model for language tasks such as writing, explanation and summarisation.
4. An explicitly configured connected provider when local capability is missing or insufficient and the user approves the exact transfer.

Do not start, download, switch or run models automatically. Do not run multiple large models in parallel by default. Discovery and recommendations are user-initiated and resource-bounded. If the local machine cannot support a useful model, explain the limitation and offer an optional connected route.

## Interoperability tiers

### Tier 1 — open and launch
The first directory increment contains official provider website links only. Browser-preview links open in a new tab; native desktop hand-off to the system browser is not yet validated and must not be claimed until a restricted Tauri opener adapter is added and tested. Installed desktop-application launching requires a separate, allow-listed OS opener adapter and platform validation. Links must be clearly labelled as external. No account is connected and no UJANI content is transferred by merely opening a link.

### Tier 2 — user-directed hand-off
Where safe and supported, prepare content in UJANI and let the user copy it, open a native share/file picker, or create a draft using a documented URI/API. Show exactly what will be handed off. Never silently read another application's private content.

### Tier 3 — explicit provider integration
Use official APIs, OAuth and documented protocols only. Treat provider subscriptions and API billing as separate unless the provider explicitly documents otherwise. Never scrape private application interfaces, extract credentials, automate login or bypass subscription boundaries.

A static link directory is deliberately small and has no background process, polling, credentials or new network dependency. Deeper adapters are separate optional modules and must not increase startup cost when unused.

## Privacy and permissions

- No prompt, file, email or selected text leaves the device without a clear, task-specific approval.
- Opening a provider website is not consent to transmit content.
- Show provider, destination, data scope and relevant billing/privacy caveats before external requests.
- Never store API keys in logs, history or plaintext settings.
- Respect OS access controls and user-selected scopes.
- Do not collect content for product analytics. Any optional usage counters must be local, content-free and user-controlled.

## Cross-platform compatibility

Use a capability adapter, not an OS-name assumption. Report Available, Unavailable or Unknown with provider provenance. Native search providers can be missing, disabled, stale, permission-limited or configured to exclude locations. A missing search result is not proof that a file does not exist.

- Windows: prefer documented Windows Search interfaces and native file/application handlers where available.
- macOS: prefer Spotlight metadata APIs and Launch Services/default handlers where available.
- Linux: detect supported desktop search services and desktop URI/application handlers; availability differs between distributions and desktop environments.
- All platforms: provide a bounded, explicit user-selected-folder fallback; never silently crawl the whole disk.

## Performance and dependency budgets

- No new always-on daemon, file watcher, persistent indexer or integration polling for Tier 1.
- Lazy-load optional UI and adapters.
- Do not add a dependency for a capability already supplied by the OS or existing Tauri APIs.
- Every new dependency requires a licence, maintenance, security, binary-size and platform-support review.
- Measure cold startup, idle memory, installer size, foreground responsiveness and task CPU/RAM before and after integration.
- Bound provider timeouts, result counts, payload sizes and cancellation.
- If a feature's compatibility or maintenance cost outweighs its value, ship an official link or manual hand-off instead.

## Acceptance criteria

- Core chat and local tasks still work with network disabled.
- The integration directory opens official destinations and does not transfer UJANI content.
- A missing external app or provider never prevents UJANI startup.
- OS search reports provider availability and coverage limitations honestly.
- Local writing actions route only to an installed, suitable model or explain why unavailable.
- User cancellation stops work and prevents subsequent external transfer.
- No new background process or continuous indexing is introduced.
- Windows, macOS and supported Linux builds receive platform-specific tests; real-device checks are recorded separately from CI.
