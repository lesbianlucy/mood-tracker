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
        }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanicEvent {
    pub id: String,
    pub user_uuid: String,
    pub timestamp: DateTime<Utc>,
    pub mood_at_panic: Option<i32>,
    pub high_level_at_panic: Option<i32>,
    pub notified_contacts: Vec<String>,
}
