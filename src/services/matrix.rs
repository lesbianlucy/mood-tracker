#![allow(dead_code)]

use chrono::{DateTime, Utc};
use matrix_sdk::{
    matrix_auth::{MatrixSession, MatrixSessionTokens},
    ruma::{events::room::message::RoomMessageEventContent, OwnedDeviceId, OwnedUserId, UserId},
    Client, SessionMeta,
};
use tracing::{info, warn};
use url::Url;

use crate::{
    error::AppError,
    models::{
        checkin::Checkin,
        settings::{GlobalConfig, UserConfig},
    },
};

#[derive(Clone, Default)]
pub struct MatrixService;

impl MatrixService {
    pub fn new() -> Self {
        Self
    }

    pub async fn send_low_mood_notification(
        &self,
        user_cfg: &UserConfig,
        global_cfg: &GlobalConfig,
        checkin: &Checkin,
    ) -> Result<Vec<String>, AppError> {
        if !self.is_enabled(user_cfg) {
            return Ok(Vec::new());
        }
        let Some(client) = self.prepare_client(user_cfg).await? else {
            return Ok(Vec::new());
        };
        let message = self.render_template(
            &global_cfg.low_mood_message_template,
            user_cfg,
            Some(checkin),
            checkin.timestamp,
        );
        let contacts = self.collect_contacts(user_cfg);
        if contacts.is_empty() {
            warn!("Keine Matrix-Kontakte für automatische Benachrichtigung hinterlegt");
            return Ok(Vec::new());
        }
        self.notify_contacts(&client, &contacts, &message).await
    }

    pub async fn send_panic_notification(
        &self,
        user_cfg: &UserConfig,
        global_cfg: &GlobalConfig,
        checkin: Option<&Checkin>,
    ) -> Result<Vec<String>, AppError> {
        if !self.is_enabled(user_cfg) {
            return Ok(Vec::new());
        }
        let Some(client) = self.prepare_client(user_cfg).await? else {
            return Ok(Vec::new());
        };
        let message = self.render_template(
            &global_cfg.panic_message_template,
            user_cfg,
            checkin,
            Utc::now(),
        );
        let contacts = self.collect_contacts(user_cfg);
        if contacts.is_empty() {
            warn!("Keine Matrix-Kontakte für Panic-Alarm hinterlegt");
            return Ok(Vec::new());
        }
        self.notify_contacts(&client, &contacts, &message).await
    }

    pub async fn send_test_message(&self, user_cfg: &UserConfig) -> Result<Vec<String>, AppError> {
        if !self.is_enabled(user_cfg) {
            return Ok(Vec::new());
        }
        let Some(client) = self.prepare_client(user_cfg).await? else {
            return Ok(Vec::new());
        };
        let message = "Hi 💖, das ist eine Testnachricht aus deinem Mood-Tracker. Alles funktioniert super kawaii!".to_string();
        let contacts = vec![user_cfg.matrix_user_id.clone()];
        self.notify_contacts(&client, &contacts, &message).await
    }

    fn is_enabled(&self, user_cfg: &UserConfig) -> bool {
        !user_cfg.matrix_access_token.trim().is_empty()
    }

    fn collect_contacts(&self, user_cfg: &UserConfig) -> Vec<String> {
        let mut contacts = Vec::new();
        if let Some(primary) = user_cfg.primary_contact.as_deref() {
            let trimmed = primary.trim();
            if !trimmed.is_empty() {
                contacts.push(trimmed.to_string());
            }
        }
        contacts.extend(user_cfg.emergency_contacts.iter().filter_map(|entry| {
            let trimmed = entry.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }));
        contacts
    }

    async fn prepare_client(&self, user_cfg: &UserConfig) -> Result<Option<Client>, AppError> {
        let token = user_cfg.matrix_access_token.trim();
        if token.is_empty() {
            return Ok(None);
        }
        let Some(device_id) = user_cfg
            .matrix_device_id
            .as_deref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        else {
            warn!("Matrix Access Token gesetzt, aber keine Device ID angegeben.");
            return Ok(None);
        };

        let homeserver = Url::parse(&user_cfg.homeserver_url)
            .map_err(|err| AppError::BadRequest(format!("Ungültige Homeserver URL: {err}")))?;

        let client = Client::builder()
            .homeserver_url(homeserver)
            .build()
            .await
            .map_err(|err| AppError::Other(err.into()))?;

        let user_id = UserId::parse(&user_cfg.matrix_user_id)
            .map_err(|_| AppError::BadRequest("Matrix User ID ist ungültig.".into()))?;
        let device_id = OwnedDeviceId::try_from(device_id.to_string())
            .map_err(|_| AppError::BadRequest("Matrix Device ID ist ungültig.".into()))?;

        let session = MatrixSession {
            meta: SessionMeta {
                user_id: user_id.to_owned(),
                device_id,
            },
            tokens: MatrixSessionTokens {
                access_token: token.to_string(),
                refresh_token: None,
            },
        };

        client
            .restore_session(session)
            .await
            .map_err(|err| AppError::Other(err.into()))?;

        Ok(Some(client))
    }

    async fn notify_contacts(
        &self,
        client: &Client,
        contacts: &[String],
        message: &str,
    ) -> Result<Vec<String>, AppError> {
        let mut notified = Vec::new();
        for contact in contacts {
            let trimmed = contact.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(user_id) = OwnedUserId::try_from(trimmed.to_string()) else {
                warn!(contact = %trimmed, "Matrix Kontakt konnte nicht geparst werden");
                continue;
            };
            let room = client
                .create_dm(user_id.as_ref())
                .await
                .map_err(|err| AppError::Other(err.into()))?;
            room.send(RoomMessageEventContent::text_plain(message))
                .await
                .map_err(|err| AppError::Other(err.into()))?;
            notified.push(trimmed.to_string());
        }
        if notified.is_empty() {
            warn!("Matrix Benachrichtigung konnte niemanden erreichen.");
        } else {
            info!(targets = ?notified, "Matrix Nachrichten versendet");
        }
        Ok(notified)
    }

    fn render_template(
        &self,
        template: &str,
        user_cfg: &UserConfig,
        checkin: Option<&Checkin>,
        timestamp: DateTime<Utc>,
    ) -> String {
        let mut message = template.to_string();
        message = message.replace("{username}", &user_cfg.display_name);
        message = message.replace(
            "{mood}",
            &checkin
                .map(|c| c.mood.to_string())
                .unwrap_or_else(|| "unbekannt".into()),
        );
        message = message.replace(
            "{high_level}",
            &checkin
                .map(|c| c.high_level.to_string())
                .unwrap_or_else(|| "0".into()),
        );
        message = message.replace(
            "{timestamp}",
            &timestamp.format("%d.%m.%Y %H:%M").to_string(),
        );
        message
    }
}
