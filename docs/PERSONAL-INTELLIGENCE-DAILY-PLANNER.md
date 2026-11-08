# UJANI Personal Intelligence & Daily Planner

**Status:** design specification; no runtime integration is claimed by this document.  
**Tracking:** [Issue #81](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/81); related everyday-tools roadmap [#46](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/46).

## 1. Product intent

UJANI is an optional intelligent coordinator and user-facing planner. It connects only to sources the user chooses, normalises useful commitments, and presents a source-linked agenda, reminders, meeting preparation and a morning briefing.

UJANI does not replace Outlook, Zoom, a calendar, a task manager, an automation platform or an OS notification service. It should reuse those products through small, documented adapters. The feature is disabled by default and must remain useful without any external account.

UJANI cannot read every installed application. It may use documented provider APIs, supported OS APIs and explicitly selected local folders/files. Applications without a supported integration are shown as unavailable/not supported, not silently inspected.

## 2. User experience

The optional Personal Intelligence area contains:
- **Today:** fixed events, user tasks, confirmed deadlines, preparation blocks and free-time gaps.
- **Reminders:** birthdays, anniversaries, renewals, appointments and recurring user-created reminders.
- **Meeting preparation:** event metadata and a bounded set of user-authorised source references.
- **Suggestions:** proposed priorities or time blocks, clearly separated from confirmed commitments.
- **Connections:** each connected source, selected account, exact permissions, last successful sync, data retained and disconnect/delete controls.

A morning briefing is user-configurable (time, weekdays, quiet hours, notification channel). No schedule is installed until the user opts in. Missed briefings are deduplicated; provider failure is visible and does not erase local reminders.

## 3. Coordinator architecture

```text
User
  -> Personal Intelligence UI / consent manager
  -> Existing UJANI capability + integration registry
  -> Provider adapters (read-only, least privilege)
  -> Normalised commitment/evidence records
  -> Deterministic agenda and reminder engine
  -> Optional bounded AI explanation / extraction
  -> Local notification service and daily briefing
```

Reuse existing Public AI integration/capability contracts where possible. Do not add a second plugin framework, an embedded workflow engine, a permanent agent loop or a large bundled third-party product.

### Candidate source adapters

| Source | First useful scope | Integration route |
|---|---|---|
| Outlook | Calendar events; optionally selected mail queries | Microsoft Graph delegated OAuth |
| Google | Calendar; optionally Gmail search | Google OAuth APIs |
| Zoom | Meeting title/time/join metadata where authorised | Zoom OAuth/API; calendar event may be sufficient initially |
| CalDAV / OS calendar | Events and reminders | Documented CalDAV or native OS APIs |
| Notes and documents | Explicitly selected files/folders only | Native picker; bounded parsing and user-selected scope |
| Task/workflow tools | Tasks and completion state | Documented APIs, MCP or external bridge |
| Personal reminders | User-created and recurring dates | UJANI local store |

Each adapter declares supported platforms, provider, account, data categories, exact scopes, read/write capability, freshness, rate limits and data retention. Unsupported capabilities must be explicit.

## 4. External products: coordinator, not replacement

Prefer connecting to an existing provider or an independently installed planner/automation product. UJANI can recommend an integration and explain setup; the customer chooses whether to install/connect it. The bridge should exchange only typed, bounded records and must not grant broad access to UJANI.

Initial evaluation candidates:
- **[DailyOS](https://github.com/stadimeti19/DailyOS)** (MIT, early beta): a useful local-first reference. Its repository currently marks Gmail and Google Calendar integrations as implemented but warns that live-account integration is not yet fully validated; its weekday morning schedule requires an explicit foreground worker (`dailyos start`), and native OS keychain support/additional providers remain planned. Treat it as an evaluation candidate, not a dependency. Review its current code, threat model, data flows, platform support, maintenance and dependency footprint before adoption.
- **[Open Assistant](https://open-assistant.org/)** (BSL 1.1): it offers broad Outlook/email/calendar connectors, but commercial reuse or distribution needs a separate licence agreement. It may be recommended as an independently installed product only after the customer's own licence and data-handling review.
- **[n8n](https://n8n.io/)** (Sustainable Use License): it may be suitable as an independently operated customer workflow system in some configurations. Its official licence FAQ says customer-facing workflow building/configuration through an embedded UI, API, MCP or agent requires a commercial licence; do not bundle, host or expose workflow authoring from UJANI without written licence clearance.
- Prefer documented APIs and open protocols such as CalDAV and MCP where the provider implementation is secure and maintained.

A recommendation link or adapter does not imply that UJANI has installed, authenticated, validated or enabled that third-party service.

## 5. Privacy, security and user control

- Off by default. Separate consent for each provider, account and data category; e.g. Outlook Calendar is independent from Outlook Mail.
- OAuth delegated least privilege. Store tokens in the OS credential vault; never log secrets or place them in ordinary configuration.
- Read-only in MVP. No sending mail, editing/deleting meetings, contacting people, creating provider tasks or changing external systems.
- Local-first. Do not send source content to a cloud model, external planner or UJANI Enterprise without a separate explicit consent and a clear preview.
- Persist only the minimum structured commitment: provider/source reference, title or user-approved summary, start/due time, timezone, confidence, status, provenance, last-seen time and user confirmation.
- Do not persist complete email bodies, meeting transcripts or attachments by default.
- Treat email, calendar descriptions, documents and third-party tool output as untrusted evidence. They cannot alter policy, expand scopes, issue instructions to tools or authorise actions.
- AI-extracted dates/tasks remain **suggestions** until user confirmation. Display source, confidence and original wording; never guess an exact date from ambiguous text.
- Revocation stops future reads and deletes/reconciles cached provider data according to the user-visible retention policy. Local reminders the user independently created remain unless the user deletes them.
- Bounded sync windows, result counts, payload sizes, timeouts, retries, cancellation, rate limits and retention. No continuous AI inference.
- Status values distinguish connected, needs attention, stale, unavailable, revoked and unsupported. A failed provider must not make the whole planner fail.

## 6. Scheduling and intelligence

Use deterministic code for recurrence, timezone conversion, duplicate detection, conflict detection, notification timing and timeline validation. Use AI only when a user requests summarisation, meeting preparation or semantic extraction that cannot be done reliably with deterministic parsing.

The planner must:
- preserve fixed calendar commitments;
- retain original timezone and correctly handle daylight-saving transitions;
- avoid duplicate reminders across providers;
- identify uncertain or conflicting deadlines rather than silently choosing one;
- label proposed focus blocks separately from actual calendar events;
- explain why an item is prioritised and link to its source;
- allow dismiss, snooze, confirm, edit and delete of local reminders;
- never autonomously reschedule or message anyone.

## 7. Delivery sequence

1. Local reminders, recurring dates, local agenda and native desktop notifications; offline and account-free.
2. Read-only calendar sources: OS/CalDAV, Google Calendar and Outlook Calendar.
3. Zoom meeting metadata and source-linked meeting preparation.
4. User-selected mail and notes queries for candidate follow-ups/renewals, with confirm-before-save.
5. Daily planning and optional morning briefing, with source links and confidence.
6. External planner/workflow bridge after compatibility, security and licence review.

## 8. Test and release gates

Required tests include:
- disabled-by-default and independent scope consent;
- denied, expired and revoked provider tokens;
- source outage, stale data, partial results and rate limits;
- duplicate event/reminder reconciliation;
- timezone and daylight-saving boundaries;
- prompt-injection strings in email, calendar descriptions and documents;
- no provider write requests in MVP;
- cancellation, bounded memory/CPU/network and retry deduplication;
- secret/content redaction and deletion on revocation;
- keyboard navigation, screen-reader labels and responsive UI.

Provide deterministic offline fixtures so users can try the planner without credentials or network access. Do not claim Windows/macOS/Linux parity until each native integration and notification path is tested on those platforms.

## 9. Non-goals for MVP

No whole-disk scan, indiscriminate mailbox sync, screen scraping, keylogging, employee monitoring, always-on microphone, hidden background agent, auto-send, calendar mutation, purchase, contact outreach or cross-product data transfer. No promise of universal application compatibility.

## 10. Implementation status

This file is a design specification only. It does not assert that the adapters, UI, reminder database, scheduler, notification worker, OAuth flows or third-party bridges exist.