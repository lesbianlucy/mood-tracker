#![allow(dead_code)]

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use serde_json::Value;
use tokio::fs;

use crate::{error::AppError, models::checkin::Checkin};

const CHECKINS_FILE: &str = "checkins.json";

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

    pub async fn ensure_structure(&self) -> Result<(), AppError> {
        let users = self.root().join("users");
        let logs = self.root().join("logs").join("panic_events");
        fs::create_dir_all(users).await?;
        fs::create_dir_all(logs).await?;
        Ok(())
    }

    pub fn user_dir(&self, user_uuid: &str) -> PathBuf {
        self.root().join("users").join(user_uuid)
    }

    pub async fn ensure_user_dir(&self, user_uuid: &str) -> Result<PathBuf, AppError> {
        let dir = self.user_dir(user_uuid);
        fs::create_dir_all(&dir).await?;
        Ok(dir)
    }

    pub async fn load_user_checkins(&self, user_uuid: &str) -> Result<Vec<Checkin>, AppError> {
        let path = self.user_dir(user_uuid).join(CHECKINS_FILE);
        if !fs::try_exists(&path).await? {
            return Ok(Vec::new());
        }
        let raw = fs::read(&path).await?;
        if raw.is_empty() {
            return Ok(Vec::new());
        }
        let checkins: Vec<Checkin> =
            serde_json::from_slice(&raw).map_err(|err| AppError::Other(err.into()))?;
        Ok(checkins)
    }

    pub async fn save_user_checkins(
        &self,
        user_uuid: &str,
        checkins: &[Checkin],
    ) -> Result<(), AppError> {
        let dir = self.ensure_user_dir(user_uuid).await?;
        let path = dir.join(CHECKINS_FILE);
        let data =
            serde_json::to_vec_pretty(checkins).map_err(|err| AppError::Other(err.into()))?;
        fs::write(path, data).await?;
        Ok(())
    }

    pub async fn append_checkin(
        &self,
        user_uuid: &str,
        checkin: Checkin,
    ) -> Result<Checkin, AppError> {
        let mut items = self.load_user_checkins(user_uuid).await?;
        items.push(checkin.clone());
        items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        self.save_user_checkins(user_uuid, &items).await?;
        // Return the canonical record (after sorting) in case timestamps moved.
        let saved = items
            .into_iter()
            .find(|c| c.id == checkin.id)
            .unwrap_or(checkin);
        Ok(saved)
    }

    pub async fn write_user_json(
        &self,
        user_uuid: &str,
        filename: &str,
        value: &Value,
    ) -> Result<(), AppError> {
        let dir = self.ensure_user_dir(user_uuid).await?;
        let path = dir.join(filename);
        let data = serde_json::to_vec_pretty(value).map_err(|err| AppError::Other(err.into()))?;
        fs::write(path, data).await?;
        Ok(())
    }
}
