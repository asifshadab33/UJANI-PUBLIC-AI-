# UJANI local activity and search cache — design v1

## Purpose

Make repeated everyday commands faster without treating cached data as authoritative. A cache hit is a hint that can avoid repeated discovery work; it is never proof that a file exists, that a user still has access, or that an action is authorised.

Public AI and Enterprise AI may share versioned cache concepts and result metadata contracts, but must not share cache contents, identity, permissions, encryption keys, or invalidation authority.

## Public AI: local-first personal cache

### Search request identity

Build a deterministic cache key from:
- trimmed, lowercased query using the same normalization as the current local search implementation;
- selected scope ID, not a webview-supplied filesystem path;
- file-type and content-search filters;
- search provider and parser/index version;
- user-visible search preferences.

Do not include document contents, secrets, or raw credentials in cache keys. Scope IDs are opaque and are resolved only by the native backend.

### Cache lookup sequence

1. Resolve the scope ID in the native backend and confirm it is still an approved user-selected scope.
2. Look up a recent result snapshot for the full request key.
3. Validate the cached directory snapshot and every result against the filesystem. Check existence, file identity where supported, size, modification time and canonical containment under the approved scope.
4. If all validation succeeds and the snapshot is within its short freshness window, return it as a recent result.
5. If validation fails, a scope is unavailable, or the user asks to refresh, discard the stale snapshot and perform a bounded search.
6. Persist the refreshed metadata snapshot after a successful search.

A valid cache hit may accelerate returning known results. To discover newly created files, validate directory metadata for every traversed directory; if any directory changed, rescan. If the platform cannot provide reliable directory change evidence, expire the snapshot and rescan rather than claiming freshness.

### Data minimisation

Default cached data is metadata only:
- opaque scope ID;
- relative path or encrypted path metadata;
- file identity where available;
- size and modification timestamp;
- match reason and bounded result ranking;
- parser/provider version and validation time.

Do not cache raw document text, extracted PDF text, OCR output, video frames, transcripts, or embeddings by default. A future content index must be a separate explicit opt-in, encrypted at rest with a key held by the OS credential store, and clearable independently.

### Bounds and invalidation

- Default successful-result retention is **30 days**. Users may choose **7, 30, or 90 days**, **Until cleared**, or turn caching **Off**.
- "Until cleared" is still bounded by a 50-entry / 10 MiB LRU limit; it is not unlimited storage.
- Negative/no-result entries expire after **24 hours** even when the user selects "Until cleared", because absence can change quickly.
- The default cache stores metadata only, encrypted with AES-256-GCM using a key held in the OS credential vault. If the vault is unavailable, caching is skipped and the bounded search runs fresh.
- The cache database is local, permission-restricted, schema-versioned, and disposable; corruption causes cache recovery by discarding the cache rather than returning questionable results.
- Keep a bounded LRU cache with a maximum of 50 entries / 10 MiB; evict least-recently-used entries first.
- Cache lookup still traverses the approved folder to compare the complete metadata inventory. A hit avoids repeating text/PDF content inspection, not filesystem permission checks or metadata validation.
- Invalidate on scope removal, permission change, parser/provider upgrade, file deletion/move/change, explicit refresh, sign-out/profile switch, or cache clear.
- Never return a cached path without rechecking that it remains inside the approved scope.
- Do not follow symlinks or reparse points during validation; validate the opened file handle where content is read.
- Cache errors must fail open to a fresh bounded search, not fail closed to stale results.
- Cache only fully completed searches. If any eligible text/PDF file cannot be read or parsed, exceeds a supported content limit, or the worker times out, mark the result partial and do not persist it; the UI explains that some eligible files could not be inspected.
- Retain at most 20 encrypted recent query phrases, covered by the same retention policy; an empty history must not create or unlock a credential-vault key.
- Perform SQLite and OS credential-vault operations on Tauri's blocking worker pool, never on the UI thread or async executor workers.

### Remembering repeated commands

Remember low-risk preferences such as the user's last selected scope, file-type filter, or preferred search mode only after the user has selected them. Show and provide controls to clear recent searches and remembered scopes. Do not infer or retain sensitive personal interests from filenames or document contents.

## Enterprise AI: tenant-isolated cache

Enterprise cache entries must be partitioned by organisation/tenant, site, device, authenticated principal, scope, policy version and data classification as applicable.

Every cache lookup must re-authorise the current caller against the authoritative server-side identity and policy state. Never rely on cached permissions or cached enrollment status. Revocation, tenant changes, role changes, device de-enrollment and policy changes invalidate affected entries immediately.

Prefer agent-local metadata caches for device file discovery and server-side caches for fleet inventory/query results. A device's personal cache must never be exposed to an operator unless an organisation policy and the device's explicit data-sharing contract permit the exact metadata fields.

No cross-tenant cache keys, shared embeddings, shared document text, or cache warming from one tenant into another. Do not cache secrets, credentials, private keys, raw telemetry payloads, or unrestricted file contents.

## Tools and workflow cache

For repeated use of a tool, UJANI may remember:
- the user's last selected tool and safe non-sensitive options;
- a validated tool descriptor/version;
- a recent result summary with an expiry time.

Before invoking a tool, re-check its current availability, version, permissions, required consent and input constraints. Cache entries must never bypass confirmation for side effects, approval gates, or policy checks. Do not cache a successful action as proof that the action remains valid.

## Observability and user controls

- Mark results as "recent" or "validated just now" with the validation timestamp.
- Provide Refresh, encrypted recent-query history and Clear cache controls. Recent query text is encrypted with the same OS-protected key and is shown only while its selected folder still exists.
- Record cache hit/miss, validation outcome and invalidation reason without logging query contents or file names by default. Recent query history is local, encrypted and covered by the same retention setting.
- Keep telemetry opt-in and local by default in Public AI.
- In Enterprise, audit access to organisation-controlled cache entries under tenant-scoped audit policy.

## Delivery sequence

1. Keep the current native file finder read-only and bounded.
2. Introduce opaque native scope IDs and persisted user-approved scope records.
3. Add metadata-only SQLite cache with schema versioning, TTL, LRU and explicit clear controls.
4. Add directory-snapshot and file-handle validation before returning cached results.
5. Add tests for stale files, changed files, moved/deleted files, scope revocation, symlink/reparse-point replacement, parser version changes, cache corruption and cross-scope isolation.
6. Add optional content indexing only after a separate privacy/security review.
7. Implement Enterprise cache adapters only after authenticated enrollment, tenant RBAC and authoritative revocation are live.

## Non-goals

This design does not authorise whole-disk scanning, silent background indexing, document upload, automatic file modification, cross-tenant search, or bypassing user approval. It is a cache design, not a claim that these features are already implemented.


## Privacy default update

Search-result caching and encrypted recent-query history are **off until the user explicitly enables** “Remember validated search results” in the local finder. Enabling it stores bounded, encrypted metadata and recent query phrases under the OS credential-vault key. Upgrading from schema v1 disables the implicit-on setting and clears entries created under that default; it does not infer consent from the previous default.
