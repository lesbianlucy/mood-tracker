#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkin {
    pub id: String,
    pub user_uuid: String,
    pub timestamp: DateTime<Utc>,
    pub mood: i32,
    pub high_level: i32,
    pub safety_answer: Option<String>,
    pub feels_safe: bool,
    pub notes: Option<String>,
    pub drugs: Vec<DrugEntry>,
    pub auto_notifications: AutoNotifications,
    #[serde(default)]
    pub status_tags: Vec<String>,
}

impl Checkin {
    pub fn new(user_uuid: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_uuid: user_uuid.into(),
            timestamp: Utc::now(),
            mood: 0,
            high_level: 0,
            safety_answer: None,
            feels_safe: true,
            notes: None,
            drugs: Vec::new(),
            auto_notifications: AutoNotifications::default(),
            status_tags: Vec::new(),
        }
    }

    pub fn safety_answer_text(&self) -> &str {
        self.safety_answer
            .as_deref()
            .unwrap_or("Kein Safety-Check angegeben 🌱")
    }

    pub fn notes_text(&self) -> Option<&str> {
        self.notes.as_deref()
    }

    pub fn has_notes(&self) -> bool {
        self.notes.is_some()
    }

    pub fn notes_display(&self) -> &str {
        self.notes.as_deref().unwrap_or("")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoNotifications {
    pub mood_threshold_triggered: bool,
    pub panic_triggered: bool,
    pub notified_contacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrugEntry {
    pub substance: String,
    pub dose: String,
    pub route: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

impl DrugEntry {
    pub fn route_text(&self) -> &str {
        self.route.as_deref().unwrap_or("n/a")
    }

    pub fn notes_text(&self) -> Option<&str> {
        self.notes.as_deref()
    }

    pub fn has_notes(&self) -> bool {
        self.notes.is_some()
    }

    pub fn notes_display(&self) -> &str {
        self.notes.as_deref().unwrap_or("")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanicEvent {
    pub id: String,
    pub user_uuid: String,
    pub timestamp: DateTime<Utc>,
    pub mood_at_panic: Option<i32>,
    pub high_level_at_panic: Option<i32>,
    pub notified_contacts: Vec<String>,
}
