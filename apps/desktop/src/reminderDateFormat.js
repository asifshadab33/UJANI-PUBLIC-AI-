/** Format a saved reminder without silently substituting the operator's local timezone. */
export function formatReminderDue(timestamp, timezone, locale) {
  if (!Number.isFinite(timestamp)) return "Invalid reminder timestamp";
  const date = new Date(timestamp);
  try {
    return new Intl.DateTimeFormat(locale, {
      dateStyle: "medium",
      timeStyle: "short",
      timeZone: timezone,
    }).format(date);
  } catch {
    // The stored zone may no longer be available on this platform. UTC is
    // unambiguous; displaying the host's local zone here would misstate the due time.
    return date.toISOString() + " (UTC; saved timezone unavailable: " + String(timezone) + ")";
  }
}
