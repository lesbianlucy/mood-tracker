#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub default_low_mood_threshold: i32,
    pub default_auto_notify_on_low_mood: bool,
    pub low_mood_message_template: String,
    pub panic_message_template: String,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            default_low_mood_threshold: 1,
            default_auto_notify_on_low_mood: true,
            low_mood_message_template: "Hey 💕, hier ist der Mood-Tracker von {username}. Stimmung: {mood}, Rausch: {high_level}/10 am {timestamp}. Nur ein kleiner Hinweis, dass ein kurzer Check-in gut tun könnte 🌸".into(),
            panic_message_template: "ALARM 💖: {username} hat in der App 'Ich brauche Hilfe' gedrückt. Stimmung: {mood} / Rausch: {high_level}/10. Vielleicht magst du kurz nach ihnen schauen 💕".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub username: String,
    pub display_name: String,
    pub homeserver_url: String,
    pub matrix_user_id: String,
    pub matrix_access_token: String,
    pub primary_contact: Option<String>,
    pub emergency_contacts: Vec<String>,
    pub auto_notify_on_low_mood: bool,
    pub auto_notify_threshold: i32,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            username: "cutie".into(),
            display_name: "Cutie".into(),
            homeserver_url: "https://matrix.org".into(),
            matrix_user_id: "@cutie:matrix.org".into(),
            matrix_access_token: "SECRET".into(),
            primary_contact: None,
            emergency_contacts: Vec::new(),
            auto_notify_on_low_mood: true,
            auto_notify_threshold: 1,
        }
    }
}
