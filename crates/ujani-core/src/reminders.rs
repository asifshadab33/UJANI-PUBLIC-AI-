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
        if self.timezone.is_empty() || self.timezone.len() > MAX_TIMEZONE_BYTES
            || self.timezone.chars().any(char::is_control) {
            return Err(ReminderValidationError::InvalidTimezone);
        }
        if self.created_at_unix_ms <= 0 || self.updated_at_unix_ms < self.created_at_unix_ms {
            return Err(ReminderValidationError::InvalidTimestamps);
        }
        if self.snoozed_until_unix_ms.is_some_and(|v| v <= 0) {
            return Err(ReminderValidationError::InvalidSnooze);
        }
        Ok(())
    }

    pub fn effective_due_at_unix_ms(&self) -> i64 {
        self.snoozed_until_unix_ms.unwrap_or(self.due_at_unix_ms)
    }

    pub fn should_notify_at(&self, now_unix_ms: i64) -> bool {
        self.state == ReminderState::Active
            && self.effective_due_at_unix_ms() <= now_unix_ms
            && self.last_notified_due_at_unix_ms != Some(self.effective_due_at_unix_ms())
    }
}

/// Deterministic bounded agenda; the caller supplies time and no clock is read here.
pub fn upcoming_reminders<'a>(reminders: &'a [LocalReminder], now_unix_ms: i64, horizon_ms: i64) -> Vec<&'a LocalReminder> {
    if now_unix_ms <= 0 || horizon_ms < 0 || horizon_ms > MAX_AGENDA_HORIZON_MS { return Vec::new(); }
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
        let items = upcoming_reminders(&[reminder("later", 300), reminder("soon", 150), reminder("outside", 999)], 100, 500);
        assert_eq!(items.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(), vec!["soon", "later"]);
        assert!(upcoming_reminders(&[reminder("soon", 150)], 100, MAX_AGENDA_HORIZON_MS + 1).is_empty());
    }
    #[test] fn notification_deduplication_uses_effective_due_instant() {
        let mut item = reminder("r1", 100); assert!(item.should_notify_at(100));
        item.last_notified_due_at_unix_ms = Some(100); assert!(!item.should_notify_at(200));
        item.snoozed_until_unix_ms = Some(300); assert!(item.should_notify_at(300));
    }
    #[test] fn snooze_and_completed_state_are_respected() {
        let mut item = reminder("r1", 100); item.snoozed_until_unix_ms = Some(500);
        assert!(!item.should_notify_at(400)); assert!(item.should_notify_at(500));
        item.state = ReminderState::Completed; assert!(!item.should_notify_at(600));
    }
}
