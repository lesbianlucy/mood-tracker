#![allow(dead_code)]

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::fs;
use tracing::{error, warn};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        checkin::{Checkin, PanicEvent},
        settings::{GlobalConfig, UserConfig},
    },
};

#[derive(Clone)]
pub struct StorageService {
    root: Arc<PathBuf>,
}

impl StorageService {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root: Arc::new(root),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn global_config_path(&self) -> PathBuf {
        self.root().join("config.json")
    }

    fn users_root(&self) -> PathBuf {
        self.root().join("users")
    }

    fn logs_root(&self) -> PathBuf {
        self.root().join("logs")
    }

    fn panic_log_dir(&self) -> PathBuf {
        self.logs_root().join("panic_events")
    }

    fn user_dir(&self, user_uuid: &str) -> PathBuf {
        self.users_root().join(user_uuid)
    }

    fn user_checkins_dir(&self, user_uuid: &str) -> PathBuf {
        self.user_dir(user_uuid).join("checkins")
    }

    fn user_trips_dir(&self, user_uuid: &str) -> PathBuf {
        self.user_dir(user_uuid).join("trips")
    }

    fn user_config_path(&self, user_uuid: &str) -> PathBuf {
        self.user_dir(user_uuid).join("config.json")
    }

    fn checkin_path(&self, user_uuid: &str, checkin_id: &str) -> PathBuf {
        self.user_checkins_dir(user_uuid)
            .join(format!("{checkin_id}.json"))
    }

    fn panic_event_path(&self, timestamp: DateTime<Utc>, id: &str) -> PathBuf {
        self.panic_log_dir()
            .join(format!("{}-{id}.json", timestamp.format("%Y%m%dT%H%M%SZ")))
    }

    pub async fn ensure_structure(&self) -> Result<(), AppError> {
        fs::create_dir_all(self.root()).await?;
        fs::create_dir_all(self.users_root()).await?;
        fs::create_dir_all(self.panic_log_dir()).await?;

        if !fs::try_exists(self.global_config_path()).await? {
            self.save_global_config(&GlobalConfig::default()).await?;
        }

        Ok(())
    }

    pub async fn ensure_user_scaffold(
        &self,
        user_uuid: &str,
        username: &str,
    ) -> Result<UserConfig, AppError> {
        fs::create_dir_all(self.user_dir(user_uuid)).await?;
        fs::create_dir_all(self.user_checkins_dir(user_uuid)).await?;
        fs::create_dir_all(self.user_trips_dir(user_uuid)).await?;

        if fs::try_exists(self.user_config_path(user_uuid)).await? {
            return self.load_user_config(user_uuid).await;
        }

        let cfg = UserConfig::for_new_user(username);
        self.save_user_config(user_uuid, &cfg).await?;
        Ok(cfg)
    }

    pub async fn load_global_config(&self) -> Result<GlobalConfig, AppError> {
        let path = self.global_config_path();
        if !fs::try_exists(&path).await? {
            self.save_global_config(&GlobalConfig::default()).await?;
        }
        let raw = fs::read(&path).await?;
        let cfg: GlobalConfig =
            serde_json::from_slice(&raw).map_err(|err| AppError::Other(err.into()))?;
        Ok(cfg)
    }

    pub async fn save_global_config(&self, cfg: &GlobalConfig) -> Result<(), AppError> {
        let path = self.global_config_path();
        self.write_json_atomic(&path, cfg).await
    }

    pub async fn load_user_config(&self, user_uuid: &str) -> Result<UserConfig, AppError> {
        let path = self.user_config_path(user_uuid);
        if !fs::try_exists(&path).await? {
            return Err(AppError::NotFound);
        }
        let raw = fs::read(&path).await?;
        let mut cfg: UserConfig =
            serde_json::from_slice(&raw).map_err(|err| AppError::Other(err.into()))?;
        if cfg.username.trim().is_empty() {
            cfg.username = user_uuid.to_string();
        }
        Ok(cfg)
    }

    pub async fn save_user_config(
        &self,
        user_uuid: &str,
        cfg: &UserConfig,
    ) -> Result<(), AppError> {
        let path = self.user_config_path(user_uuid);
        self.write_json_atomic(&path, cfg).await
    }

    pub async fn save_checkin(&self, user_uuid: &str, checkin: &Checkin) -> Result<(), AppError> {
        fs::create_dir_all(self.user_checkins_dir(user_uuid)).await?;
        let path = self.checkin_path(user_uuid, &checkin.id);
        self.write_json_atomic(&path, checkin).await
    }

    pub async fn load_checkin(
        &self,
        user_uuid: &str,
        checkin_id: &str,
    ) -> Result<Checkin, AppError> {
        let path = self.checkin_path(user_uuid, checkin_id);
        let raw = fs::read(&path).await?;
        let checkin = serde_json::from_slice(&raw).map_err(|err| AppError::Other(err.into()))?;
        Ok(checkin)
    }

    pub async fn list_checkins(&self, user_uuid: &str) -> Result<Vec<Checkin>, AppError> {
        let dir = self.user_checkins_dir(user_uuid);
        if !fs::try_exists(&dir).await? {
            return Ok(Vec::new());
        }

        let mut entries = fs::read_dir(&dir).await?;
        let mut items: Vec<Checkin> = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let Ok(meta) = entry.metadata().await else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            if entry
                .file_name()
                .to_string_lossy()
                .to_lowercase()
                .ends_with(".json")
            {
                match fs::read(entry.path()).await {
                    Ok(raw) if !raw.is_empty() => match serde_json::from_slice(&raw) {
                        Ok(checkin) => items.push(checkin),
                        Err(err) => warn!(
                            path = %entry.path().display(),
                            "konnte Check-in JSON nicht lesen: {err}"
                        ),
                    },
                    Ok(_) => continue,
                    Err(err) => warn!(
                        path = %entry.path().display(),
                        "konnte Check-in nicht lesen: {err}"
                    ),
                }
            }
        }

        items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(items)
    }

    pub async fn latest_checkin(&self, user_uuid: &str) -> Result<Option<Checkin>, AppError> {
        let items = self.list_checkins(user_uuid).await?;
        Ok(items.into_iter().next())
    }

    pub async fn save_panic_event(&self, event: &PanicEvent) -> Result<(), AppError> {
        fs::create_dir_all(self.panic_log_dir()).await?;
        let path = self.panic_event_path(event.timestamp, &event.id);
        self.write_json_atomic(&path, event).await
    }

    pub async fn list_panic_events(&self) -> Result<Vec<PanicEvent>, AppError> {
        let dir = self.panic_log_dir();
        if !fs::try_exists(&dir).await? {
            return Ok(Vec::new());
        }
        let mut entries = fs::read_dir(&dir).await?;
        let mut items: Vec<PanicEvent> = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let Ok(meta) = entry.metadata().await else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            match fs::read(entry.path()).await {
                Ok(raw) if !raw.is_empty() => match serde_json::from_slice(&raw) {
                    Ok(event) => items.push(event),
                    Err(err) => warn!(
                        path = %entry.path().display(),
                        "konnte Panic-Event nicht lesen: {err}"
                    ),
                },
                Ok(_) => continue,
                Err(err) => warn!(
                    path = %entry.path().display(),
                    "konnte Panic-Event-Datei nicht lesen: {err}"
                ),
            }
        }
        items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(items)
    }

    pub async fn count_user_checkins(&self, user_uuid: &str) -> Result<usize, AppError> {
        let dir = self.user_checkins_dir(user_uuid);
        if !fs::try_exists(&dir).await? {
            return Ok(0);
        }
        let mut entries = fs::read_dir(&dir).await?;
        let mut count = 0usize;
        while let Some(entry) = entries.next_entry().await? {
            let Ok(meta) = entry.metadata().await else {
                continue;
            };
            if meta.is_file()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .ends_with(".json")
            {
                count += 1;
            }
        }
        Ok(count)
    }

    pub async fn count_user_panic_events(&self, user_uuid: &str) -> Result<usize, AppError> {
        let events = self.list_panic_events().await?;
        Ok(events
            .into_iter()
            .filter(|e| e.user_uuid == user_uuid)
            .count())
    }

    pub async fn count_all_checkins(&self) -> Result<usize, AppError> {
        let mut total = 0usize;
        let users_root = self.users_root();
        if !fs::try_exists(&users_root).await? {
            return Ok(0);
        }
        let mut entries = fs::read_dir(&users_root).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.metadata().await?.is_dir() {
                let uuid = entry.file_name().to_string_lossy().to_string();
                total += self.count_user_checkins(&uuid).await?;
            }
        }
        Ok(total)
    }

    pub async fn list_user_uuids(&self) -> Result<Vec<String>, AppError> {
        let mut uuids = Vec::new();
        let root = self.users_root();
        if !fs::try_exists(&root).await? {
            return Ok(uuids);
        }
        let mut entries = fs::read_dir(&root).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.metadata().await?.is_dir() {
                uuids.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        Ok(uuids)
    }

    async fn write_json_atomic<T: Serialize>(
        &self,
        path: &Path,
        value: &T,
    ) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let tmp = path.with_extension(format!(
            "tmp-{}",
            Uuid::new_v4().to_string().replace('-', "")
        ));
        let data = serde_json::to_vec_pretty(value).map_err(|err| AppError::Other(err.into()))?;
        fs::write(&tmp, &data).await?;
        if let Err(err) = fs::rename(&tmp, path).await {
            error!(
                tmp = %tmp.display(),
                dest = %path.display(),
                "konnte Datei nicht verschieben: {err}"
            );
            return Err(err.into());
        }
        Ok(())
    }

    pub async fn merge_contacts(&self, user_cfg: &UserConfig) -> Result<Vec<String>, AppError> {
        let mut contacts = Vec::new();
        if let Some(primary) = user_cfg.primary_contact.as_deref() {
            if !primary.trim().is_empty() {
                contacts.push(primary.trim().to_string());
            }
        }
        contacts.extend(
            user_cfg
                .emergency_contacts
                .iter()
                .filter_map(|c| {
                    let trimmed = c.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .collect::<Vec<_>>(),
        );
        let mut seen = HashSet::new();
        contacts.retain(|c| seen.insert(c.clone()));
        Ok(contacts)
    }
}
