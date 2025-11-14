#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum UserRole {
    #[default]
    #[serde(rename = "user")]
    User,
    #[serde(rename = "admin")]
    Admin,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::User => "user",
            UserRole::Admin => "admin",
        }
    }
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    pub uuid: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}
