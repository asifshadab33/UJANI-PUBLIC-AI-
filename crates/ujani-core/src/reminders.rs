//! Pure, bounded reminder contracts for the optional local-first planner.
//!
//! This module deliberately has no clock, filesystem, database, OS notification,
//! network, or AI dependencies. The desktop layer owns persistence and delivery.

use serde::{Deserialize, Serialize};

pub const MAX_REMINDER_ID_BYTES: usize = 96;
pub const MAX_REMINDER_TITLE_BYTES: usize = 160;
pub const MAX_REMINDER_NOTES_BYTES: usize = 2_000;
pub const MAX_TIMEZONE_BYTES: usize = 64;
pub const MAX_AGENDA_HORIZON_MS: i64 = 31 * 24 * 60 * 60 * 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderState { Active, Completed }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalReminder {
    pub id: String,
    pub title: String,
    pub notes: Option<String>,
    /// Absolute UTC instant in Unix milliseconds; timezone is retained for display.
    pub due_at_unix_ms: i64,
    pub timezone: String,
    pub state: ReminderState,
    pub snoozed_until_unix_ms: Option<i64>,
    /// Deduplication marker for the last notification delivered for this due instant.
    pub last_notified_due_at_unix_ms: Option<i64>,
    pub created_at_unix_ms: i64,
    pub updated_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReminderValidationError {
    InvalidId, InvalidTitle, NotesTooLong, InvalidDueTime,
    InvalidTimezone, InvalidTimestamps, InvalidSnooze,
}

/// Accept a bounded IANA-style zone identifier, not an arbitrary path or offset.
/// This is syntax validation only; the platform adapter must resolve the zone
/// against its timezone database before rendering local wall-clock times.
fn valid_timezone_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_TIMEZONE_BYTES
        && !value.starts_with('/')
        && !value.ends_with('/')
        && value.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'+' | b'.')
                })
        })
}

impl LocalReminder {
    pub fn validate(&self) -> Result<(), ReminderValidationError> {
        if self.id.is_empty() || self.id.len() > MAX_REMINDER_ID_BYTES
            || !self.id.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)) {
            return Err(ReminderValidationError::InvalidId);
        }
        if self.title.trim().is_empty() || self.title.len() > MAX_REMINDER_TITLE_BYTES
            || self.title.chars().any(char::is_control) {
            return Err(ReminderValidationError::InvalidTitle);
        }
        if self.notes.as_ref().is_some_and(|v| v.len() > MAX_REMINDER_NOTES_BYTES || v.chars().any(|c| c == '\0')) {
            return Err(ReminderValidationError::NotesTooLong);
        }
        if self.due_at_unix_ms <= 0 { return Err(ReminderValidationError::InvalidDueTime); }
        if !valid_timezone_identifier(&self.timezone) {
            return Err(ReminderValidationError::InvalidTimezone);
        }
        if self.created_at_unix_ms <= 0 || self.updated_at_unix_ms < self.created_at_unix_ms {
            return Err(ReminderValidationError::InvalidTimestamps);
        }
        // A snooze must move the effective due instant forward. An earlier
        // timestamp could otherwise make a future reminder notify prematurely.
        if self
            .snoozed_until_unix_ms
            .is_some_and(|v| v <= self.due_at_unix_ms)
        {
            return Err(ReminderValidationError::InvalidSnooze);
        }
        Ok(())
    }

    pub fn effective_due_at_unix_ms(&self) -> i64 {
        self.snoozed_until_unix_ms.unwrap_or(self.due_at_unix_ms)
    }

    pub fn should_notify_at(&self, now_unix_ms: i64) -> bool {
        // Invalid persisted/deserialised records must never become notification work.
        self.validate().is_ok()
            && self.state == ReminderState::Active
            && self.effective_due_at_unix_ms() <= now_unix_ms
            && self.last_notified_due_at_unix_ms != Some(self.effective_due_at_unix_ms())
    }
}

/// Deterministic bounded agenda; the caller supplies time and no clock is read here.
pub fn upcoming_reminders(reminders: &[LocalReminder], now_unix_ms: i64, horizon_ms: i64) -> Vec<&LocalReminder> {
    if now_unix_ms <= 0 || !(0..=MAX_AGENDA_HORIZON_MS).contains(&horizon_ms) { return Vec::new(); }
    let end = now_unix_ms.saturating_add(horizon_ms);
    let mut items: Vec<&LocalReminder> = reminders.iter()
        .filter(|item| item.validate().is_ok())
        .filter(|item| item.state == ReminderState::Active)
        .filter(|item| { let due = item.effective_due_at_unix_ms(); due >= now_unix_ms && due <= end })
        .collect();
    items.sort_by(|a, b| a.effective_due_at_unix_ms().cmp(&b.effective_due_at_unix_ms()).then_with(|| a.id.cmp(&b.id)));
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reminder(id: &str, due: i64) -> LocalReminder {
        LocalReminder { id: id.into(), title: "Renew policy".into(), notes: None, due_at_unix_ms: due,
            timezone: "Asia/Qatar".into(), state: ReminderState::Active, snoozed_until_unix_ms: None,
            last_notified_due_at_unix_ms: None, created_at_unix_ms: 10, updated_at_unix_ms: 10 }
    }
    #[test]
    fn accepts_iana_style_timezone_identifiers() {
        for timezone in ["UTC", "Asia/Kolkata", "Etc/GMT+5"] {
            let mut item = reminder("timezone-valid", 100);
            item.timezone = timezone.into();
            assert!(item.validate().is_ok(), "{timezone}");
        }
    }

    #[test]
    fn rejects_unsafe_or_malformed_timezone_identifiers() {
        for timezone in [
            "",
            "/etc/passwd",
            "../UTC",
            "Asia/../UTC",
            "Asia//Kolkata",
            "Asia/Kolkata ",
            "UTC+05:30",
        ] {
            let mut item = reminder("timezone-invalid", 100);
            item.timezone = timezone.into();
            assert_eq!(item.validate(), Err(ReminderValidationError::InvalidTimezone), "{timezone}");
        }
    }

    #[test] fn accepts_bounded_local_reminder() { assert!(reminder("reminder-1", 100).validate().is_ok()); }
    #[test] fn rejects_invalid_ids_titles_and_oversized_notes() {
        let mut item = reminder("../bad", 100); assert_eq!(item.validate(), Err(ReminderValidationError::InvalidId));
        item = reminder("good", 100); item.title = "  ".into(); assert_eq!(item.validate(), Err(ReminderValidationError::InvalidTitle));
        item = reminder("good", 100); item.notes = Some("x".repeat(MAX_REMINDER_NOTES_BYTES + 1));
        assert_eq!(item.validate(), Err(ReminderValidationError::NotesTooLong));
    }
    #[test] fn completed_and_invalid_records_never_enter_agenda() {
        let mut done = reminder("done", 200); done.state = ReminderState::Completed;
        let mut invalid = reminder("invalid", 200); invalid.timezone.clear();
        assert!(upcoming_reminders(&[done, invalid], 100, 500).is_empty());
    }
    #[test] fn agenda_is_sorted_and_horizon_is_bounded() {
        let records = [reminder("later", 300), reminder("soon", 150), reminder("outside", 999)];
        let items = upcoming_reminders(&records, 100, 500);
        assert_eq!(items.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(), vec!["soon", "later"]);
        assert!(upcoming_reminders(&[reminder("soon", 150)], 100, MAX_AGENDA_HORIZON_MS + 1).is_empty());
    }
    #[test] fn notification_deduplication_uses_effective_due_instant() {
        let mut item = reminder("r1", 100); assert!(item.should_notify_at(100));
        item.last_notified_due_at_unix_ms = Some(100); assert!(!item.should_notify_at(200));
        item.snoozed_until_unix_ms = Some(300); assert!(item.should_notify_at(300));
    }
    #[test]
    fn snooze_must_move_due_time_forward_and_completed_state_is_respected() {
        let mut item = reminder("r1", 100);
        item.snoozed_until_unix_ms = Some(50);
        assert_eq!(item.validate(), Err(ReminderValidationError::InvalidSnooze));
        assert!(!item.should_notify_at(50));

        item.snoozed_until_unix_ms = Some(100);
        assert_eq!(item.validate(), Err(ReminderValidationError::InvalidSnooze));

        item.snoozed_until_unix_ms = Some(500);
        assert!(item.validate().is_ok());
        assert!(!item.should_notify_at(400));
        assert!(item.should_notify_at(500));
        item.state = ReminderState::Completed;
        assert!(!item.should_notify_at(600));
    }

    #[test]
    fn agenda_includes_exact_boundaries_and_snoozed_effective_due_time() {
        let mut at_now = reminder("at-now", 100);
        let mut snoozed = reminder("snoozed-original", 80);
        snoozed.snoozed_until_unix_ms = Some(120);
        let at_end = reminder("at-end", 200);
        let before = reminder("before", 99);
        let after = reminder("after", 201);
        let records = [after, at_end, snoozed, before, at_now.clone()];
        let agenda = upcoming_reminders(&records, 100, 100);
        assert_eq!(
            agenda.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
            vec!["at-now", "snoozed-original", "at-end"]
        );
        assert_eq!(agenda[1].effective_due_at_unix_ms(), 120);
        at_now.snoozed_until_unix_ms = Some(101);
        assert_eq!(upcoming_reminders(&[at_now], 101, 0).len(), 1);
    }

    #[test]
    fn agenda_tie_breaking_is_stable_and_horizon_addition_saturates() {
        let records = [
            reminder("z", i64::MAX),
            reminder("b", 150),
            reminder("a", 150),
        ];
        let sorted = upcoming_reminders(&records, 100, 100);
        assert_eq!(sorted.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(), vec!["a", "b"]);
        let max_due_records = [reminder("max", i64::MAX)];
        let overflow_window = upcoming_reminders(
            &max_due_records,
            i64::MAX - 1,
            100,
        );
        assert_eq!(overflow_window.len(), 1);
        assert_eq!(overflow_window[0].due_at_unix_ms, i64::MAX);
    }
}
