# UJANI Public AI — Security Centre UX and Connection Wizard Specification

**Status:** Proposed implementation specification  
**Product:** UJANI Public AI  
**Default posture:** Local-only, opt-in, read-only  
**Purpose:** Give an individual a simple view of security products already installed on their device. UJANI coordinates and explains; it does not replace or claim to be an antivirus, firewall, EDR, anti-spyware or ransomware engine.

## 1. Product principles

1. **Customer choice:** no provider is enabled until the user selects it and confirms its data scope.
2. **Local first:** provider discovery, status collection, normalisation and summaries stay on-device by default.
3. **Read-only first:** this release reads security posture and provider-reported alerts. It does not change firewall rules, quarantine files, kill processes, remove threats or run remote commands.
4. **Lightweight core:** adapters are optional platform modules, not mandatory dependencies. No bundled security engines, kernel drivers, permanent background AI loop or large database.
5. **Honest status:** distinguish protected, at-risk, alert-reported, stale, unavailable, unsupported and unknown. Never convert missing telemetry into a green state.
6. **Attribution:** every observation identifies the provider and its timestamp. UJANI must say “Reported by [provider]”, not imply it independently verified the finding.
7. **Graceful absence:** the application remains fully usable when no security product is installed or a provider is unsupported.

## 2. Information architecture

Add a **Security** item to the main navigation. It opens these views:

- **Overview:** concise posture summary, active alerts and coverage gaps.
- **Protection:** antivirus/endpoint protection, firewall, ransomware protection and update/scan status.
- **Alerts:** provider-reported malware, spyware, suspicious activity and ransomware alerts.
- **Connected providers:** discovered, enabled, paused and unsupported providers.
- **Privacy & permissions:** collection scopes, local-only setting, retention, export and revocation.

Do not add a separate always-running security dashboard process. Reuse the existing application shell, navigation, design system, settings, notifications and local event/cache facilities.

## 3. Overview screen

### Header
- Title: “Security Centre”
- Subtitle: “See what your security products report. You control what UJANI can access.”
- Overall state derived from evidence, with explanatory text and a timestamp.
- Primary action: “Connect a security product”.

### Summary cards
1. Antivirus & endpoint protection
2. Firewall
3. Threat alerts (malware / spyware)
4. Ransomware protection
5. Updates & last scans
6. Connected providers

Each card displays:
- Provider/product name(s)
- State: Protected / Attention needed / Alert reported / Stale / Not connected / Unsupported / Unknown
- Last checked time and evidence age
- Number of active provider-reported alerts, if available
- A short explanation and “View details”

A card may aggregate multiple providers only when their evidence is compatible. Conflicting provider states must be shown as mixed, never silently averaged.

### Empty state
When no providers are available:
- Explain that UJANI does not install security products.
- Offer “Discover installed products”.
- Offer “Continue without connecting”.
- Do not imply the device is unprotected solely because discovery is unavailable.

## 4. Connection wizard

Use a six-step, resumable wizard. Do not request all permissions at once.

### Step 1 — Introduction
Explain:
- UJANI coordinates existing security software; it is not a replacement.
- Discovery is local and read-only.
- Nothing is shared with UJANI cloud services by default.
Buttons: “Discover on this device” and “Not now”.

### Step 2 — Discover providers
Show discovered entries grouped by category and platform:
- Antivirus / endpoint protection
- Firewall
- Threat / malware / spyware reporting
- Ransomware protection
- Network monitoring (only if an installed supported provider is found)

Each entry includes product name, publisher/vendor if known, detection source, supported read-only capabilities and discovery confidence. Discovery itself must not start scans or change settings.

If discovery requires OS permission, explain the exact permission before prompting. If no supported discovery API exists, show “Manual connection may be available” rather than probing undocumented/private interfaces.

### Step 3 — Choose providers
User explicitly selects providers. Default selection is none. For each provider, show:
- Data fields requested
- Whether access is local-only
- Polling/event mechanism and expected resource use
- Required OS permissions
- Provider-specific limitations

Do not select all automatically.

### Step 4 — Preview information
Show representative field names and sample-shaped values without exposing secrets:
- Provider name and version
- Health/protection state
- Signature/intelligence update timestamp
- Last scan timestamp and result summary
- Alert ID/reference, category, severity and event time
- Firewall profile state (not full rules by default)

Do not collect or display file contents, packet payloads, passwords, API tokens, full command lines, browsing history or unrelated personal information.

### Step 5 — Confirm and connect
Show a plain-language summary:
- Selected providers
- Requested permissions
- Data retained locally
- Polling frequency / event subscription
- Any unsupported fields
- “Connect selected providers” and “Go back”

Require an explicit confirmation. If an OS permission prompt appears, explain that it is controlled by the OS.

### Step 6 — Connection result
For each provider show Connected, Partially supported, Permission needed, Unavailable or Failed. Include a reason and retry path. A failed provider must not block others.

## 5. Provider detail and management

Provider detail screen:
- Product name, vendor and version when available
- Connection state and last successful check
- Capabilities and fields currently enabled
- Evidence freshness
- Link/button to open the provider's own UI where the OS/vendor supports it
- “Pause”, “Disconnect” and “Review permissions”

Disconnect immediately stops collection and removes future access. Retention choices for already-collected local metadata are presented separately; disconnect must not silently erase evidence the user has chosen to retain.

## 6. Alert experience

Alert rows show:
- Severity as reported by source
- Provider and product
- Category (malware, spyware, ransomware, suspicious behaviour, firewall/network)
- Event time and received time
- Provider's status / action taken, if supplied
- Stable reference to open in the source product

UJANI may explain an alert in plain language when the user asks. AI summaries must link to the underlying evidence and clearly separate provider facts from UJANI interpretation. Do not invent a detection, confidence score or remediation result.

## 7. Status and freshness model

Use explicit machine states:
- `healthy`: provider reports protection active and evidence is fresh.
- `attention`: provider reports disabled, degraded or a required update/scan condition.
- `alert`: provider reports one or more relevant alerts.
- `stale`: last evidence exceeds the provider-specific freshness threshold.
- `unavailable`: provider was configured but cannot currently be queried.
- `unsupported`: platform/provider does not expose this capability.
- `not_connected`: user has not enabled this provider.
- `unknown`: evidence is absent, ambiguous or conflicting.

Do not define one universal freshness interval for every product. Each adapter declares its expected update/check interval; the UI shows the actual age and the threshold used.

## 8. Adapter architecture and initial provider scope

Reuse the existing versioned capability registry and read-only connector/evidence contract. Public AI should have a small local adapter host with a narrow typed interface. Each adapter declares:
- Provider ID, name, version and platform support
- Read-only capabilities and permission requirements
- Exact fields collected and data classification
- Discovery method and evidence provenance
- Timeout, output-size, polling and resource budgets
- Health/freshness semantics and supported deep links
- Licence, upstream documentation and compatibility range

Initial order:
1. Windows Security Center provider discovery (where supported).
2. Microsoft Defender status/threat summary and Windows Firewall profile status using supported local interfaces.
3. Linux optional status adapters for installed ClamAV, firewalld, nftables and UFW.
4. macOS supported native posture and vendor-documented interfaces.
5. Additional open-source/commercial providers only through documented, maintained APIs.

Do not assume Microsoft Defender is the active antivirus when Windows reports a third-party provider. Do not parse undocumented output if a supported interface exists. Do not add a provider until its permissions, data handling, licensing and test strategy are reviewed.

## 9. Privacy, security and resource requirements

- All providers disabled until explicit user enablement.
- Local-only is the default; no telemetry upload or cloud account requirement.
- No credentials in URLs, logs, crash reports or UI telemetry.
- Store secrets only in OS-provided secure credential storage if a future connector genuinely needs them.
- Minimise fields; bound response bytes, event count, string length and query duration.
- Use low-frequency polling or provider events where available; exponential backoff on failure.
- Cancel requests on disconnect, application shutdown and user cancellation.
- Do not run AI inference continuously. Deterministic aggregation first; invoke local AI only on user request.
- No auto-remediation, firewall writes, quarantine, process termination, privilege escalation or remote execution in this scope.
- Make retention visible and configurable; provide clear local-data deletion controls.

## 10. Accessibility and usability

- Keyboard navigable, screen-reader labelled controls and visible focus.
- Status must use text and icon, not colour alone.
- Plain-language explanations with expandable technical evidence.
- Responsive layout suitable for narrow windows and mobile-sized screens.
- No modal-only workflow; wizard state can be resumed or cancelled.
- All destructive or privacy-impacting controls require clear confirmation.

## 11. Acceptance criteria

1. App builds and works with no security providers installed.
2. No heavyweight mandatory runtime dependency or always-on AI process is added.
3. Discovery is read-only and does not start scans or change system settings.
4. No provider is enabled without explicit user selection and confirmation.
5. Users can pause, disconnect and revoke each provider independently.
6. Provider failures, stale data, unsupported fields and conflicting evidence are represented accurately.
7. Every alert and status item includes source provenance and timestamp.
8. Tests cover malformed/oversized data, timeout, cancellation, stale/unavailable states, duplicate events, permission denial and provider isolation.
9. Windows provider discovery is covered by mocked CI tests and manual supported-Windows smoke tests; Linux/macOS unsupported cases fail gracefully.
10. Privacy review confirms no provider data leaves the device in local-only mode.

## 12. Out of scope

- Building antivirus, spyware detection, ransomware detection or firewall engines.
- Installing or silently enabling third-party products.
- Changing firewall rules, quarantining/removing files or killing processes.
- Remote device control, autonomous containment or central fleet administration.
- Claims of complete protection or guaranteed detection/removal.

## 13. Delivery sequence

**Phase A:** navigation, overview/empty states, provider data model, wizard shell and mock adapter.  
**Phase B:** Windows Security Center, Defender and Windows Firewall read-only adapters.  
**Phase C:** Linux/macOS optional adapters and provider management.  
**Phase D:** alert explanations, accessibility hardening, retention controls and cross-platform acceptance testing.
