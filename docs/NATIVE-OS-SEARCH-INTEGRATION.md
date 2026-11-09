# Native OS Search Integration Contract

Status: implementation contract; no native provider is claimed to be integrated by this document.

## Product objective

UJANI Local Finder must reuse the operating system's existing search index where available, then apply UJANI's bounded local content search only to the gaps. It must not build a competing always-on indexer or silently crawl user folders.

## Provider strategy

- Windows: prefer the Windows Search index through a supported, documented Windows API/provider. Respect the user's index locations, exclusions, permissions, removable-drive availability and index freshness. Do not assume every file or file body is indexed.
- macOS: prefer Spotlight metadata search through supported metadata APIs. Respect privacy permissions, Spotlight exclusions and volume availability. Do not infer that a missing result proves a file is absent.
- Linux: detect supported desktop index providers such as Tracker or Baloo only when their documented local query interface is available. Provider availability and desktop environment vary; absence is a normal state.
- Unsupported or unavailable provider: report that native indexed search is unavailable and offer the existing user-selected-folder scan.

## Two-stage query

1. Native indexed lookup returns candidate paths and provider metadata. Treat results as candidate discovery, not authoritative content matches.
2. UJANI validates each candidate against the selected root, current filesystem identity and user permissions. It may then run bounded local extraction for supported formats.
3. Merge and deduplicate by stable file identity where available, otherwise by canonical path plus size/mtime. Preserve source attribution (OS index, filename, text extraction, PDF extraction).
4. Never present a partial, stale, permission-denied or truncated scan as proof that no match exists.

## Security and privacy invariants

- Search starts only from an explicit user action and an OS folder/volume selection or a separately consented search scope.
- No background indexing, hidden crawling, arbitrary paths supplied by the webview, or remote network paths by default.
- Do not bypass OS access controls or request broader permissions merely to improve search coverage.
- Bound result count, query length, provider response time, extraction bytes, parser time and total task duration.
- Cancellation must stop provider work where supported and always prevent later extraction/cache writes.
- Cache only bounded metadata and file identity/freshness data; never persist extracted document text or snippets unless a separately reviewed encrypted-index feature is approved.
- Provider errors, index exclusions and unsupported formats must be surfaced as coverage limitations.

## Capability and UI contract

Each provider reports: provider identifier, availability (available/unavailable/unknown), supported query scope, whether content indexing is supported, last-known freshness if available, result truncation, and a human-readable limitation. Unknown remains unknown; do not infer provider support from OS name alone.

The UI must distinguish:
- OS indexed results;
- UJANI content-inspected results;
- results not inspected because of limits or permissions.

A user can disable native-provider use and use the existing UJANI folder scan instead.

## Rollout sequence

1. Provider capability detection and read-only diagnostics.
2. One provider implementation per OS behind a common trait and feature tests.
3. Candidate validation, deduplication, cancellation and timeout tests.
4. Integration with Local Finder with explicit source labels.
5. Native-device tests for supported Windows, macOS and Linux desktop environments.
6. Only after measurement, consider incremental indexing. Persistent content indexing requires a separate threat model, encrypted-at-rest design, key lifecycle, deletion semantics and user-visible controls.

## Non-goals

This contract does not authorize a service that continuously watches folders, a central search server, cross-device indexing, remote file access, content upload, or a replacement for native OS search.
