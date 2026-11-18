# Existing security-provider discovery and posture

## Product boundary

UJANI Public AI is a local-first security coordinator, not an antivirus, EDR, firewall, quarantine engine, or device-isolation product. It must reuse the security capabilities already supplied by the operating system or a product selected and installed by the user/organisation.

UJANI does not silently install, upgrade, configure, disable, or replace security products. If protection is absent or cannot be verified, it explains the gap and points the user to the operating system's security settings or an administrator. It may present a shortlist of reputable free and paid products only as optional information; the user or administrator makes and performs the installation decision.

## Discovery and evidence rules

1. Prefer documented, read-only OS security APIs for aggregate health and product registration.
2. Use a vendor integration only when the user/administrator explicitly configures that provider and grants the required permissions.
3. Distinguish:
   - provider reported healthy;
   - provider needs attention or reports an alert;
   - OS reports that no provider is monitoring the category;
   - provider/API unavailable, unsupported, stale, or unknown.
4. Do not infer a named product, real-time protection, signature freshness, scan result, quarantine, or isolation capability from an aggregate "healthy" category.
5. Bind every observation to its source, category, timestamp, and (when available) product identity. Stale, malformed, conflicting, or missing evidence is unverified, never healthy.
6. Keep status local by default. Enterprise sharing is limited to administrator-configured, tenant-scoped posture summaries and must not include file contents, credentials, raw command lines, or unrelated user activity.

## Capability model

The shared domain contract distinguishes malware protection, firewall, EDR, quarantine, device isolation, and security updates. A provider may expose only a subset. UI labels must say "reported by provider" and display the source and last observation time.

A provider's existence does not prove that every capability is enabled. For example, detecting an antivirus category does not establish that on-demand scanning, real-time protection, quarantine, current signatures, or ransomware protection is active.

## User experience

- If protection is reported healthy: show the provider/source and retain it; do not recommend replacing it.
- If attention/alert is reported: show the provider's own message and guide the user to its UI.
- If no provider is reported: warn that UJANI could not find a monitored provider for that category; direct the user to OS security settings or their administrator.
- If unsupported or unavailable: say "could not verify", not "unprotected".
- Offer free and paid product information only as optional user/admin guidance. Never auto-download, auto-install, elevate privileges, or accept affiliate-driven ranking.

## Integration phases

- P1: normalized evidence contract and conservative posture assessment (this change).
- P2: Windows product identity and Defender status only through supported, documented read-only interfaces; retain Security Center aggregate result as separate evidence.
- P3: Linux and macOS provider discovery using documented platform interfaces and distribution-specific support checks; report unsupported where no reliable API exists.
- P4: optional administrator/user-configured vendor connectors for scan status, threat events, quarantine and isolation capability. Each connector is read-only first, least-privilege, bounded, cancellable, and independently tested.
- P5: user-approved deep links or vendor-documented actions where available. UJANI does not itself quarantine or isolate; it asks the existing provider to do so only after explicit user/admin approval and a verified provider capability.

No phase may treat a provider's advertised capability as proof that the action succeeded. Action outcomes require provider receipts and a refreshed posture observation.
