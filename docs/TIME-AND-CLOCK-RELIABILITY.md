# UJANI Public AI — time and clock reliability

## Responsibility boundary

UJANI reads the device clock; it does not set, repair, or silently synchronise the operating-system clock. Windows Time, systemd-timesyncd/chrony, or macOS time synchronisation remains the OS administrator's responsibility. A timezone setting changes calendar display, not the underlying UTC instant.

The local clock command returns Unix-epoch milliseconds. The UI must render that instant using the platform's local timezone; it must not treat the returned value as a timezone-free wall-clock string.

## Time-source rules

- Use the OS wall clock / UTC epoch for calendar timestamps and user-visible dates.
- Use a monotonic clock (Rust `Instant`, browser monotonic performance clock, or equivalent) for elapsed durations, deadlines, cancellation budgets, retry delays and in-process TTLs.
- Never calculate task duration by subtracting two wall-clock timestamps.
- Store history timestamps in the supported signed SQLite integer range. Reject an out-of-range value; never cast and wrap it.
- If the wall clock is unavailable or before the Unix epoch, preserve the local task's result where safe, but report that its calendar timestamp could not be recorded. Do not invent a timestamp.
- A timezone/DST change must alter display only; it must not change stored UTC instants or monotonic task durations.

## Clock-jump behaviour

While a process is running, compare wall-clock deltas with monotonic deltas to detect large forward/backward adjustments. This is diagnostic evidence, not proof that NTP is synchronised. After reboot, a fresh baseline is required.

A clock jump must not extend a running task timeout, cancellation budget or local TTL. Calendar displays may move with the OS clock. UJANI must not attempt to correct the OS clock itself.

## Application updates

Updater authenticity is established by the signed update metadata/artifact and version binding, not by trusting a local timestamp. Update installation remains user-controlled. A failed check, bad signature, unavailable network or interrupted download must leave the installed application usable. A rollback/recovery test is still required before release.

## Required verification matrix

- Unix epoch, pre-epoch and out-of-range timestamp conversion.
- Manual forward and backward clock adjustment during a running task.
- Timezone and daylight-saving transition while the app remains open.
- Sleep/resume and process restart.
- Local clock unavailable or implausible: local work remains available where safe; timestamp confidence is explicit.
- Signed update with valid/invalid signature, unavailable endpoint and interrupted download.
- Windows, Linux and macOS native builds plus physical-device upgrade/recovery tests.

## Current implementation boundary

The Public Rust local-clock command returns Unix-epoch milliseconds and the UI is responsible for local-time formatting. The encrypted-history timestamp path now uses checked conversion to SQLite's signed integer range. These checks do not prove OS clock synchronisation, continuous clock-jump detection, update rollback, or physical-device behaviour; those remain release gates.
