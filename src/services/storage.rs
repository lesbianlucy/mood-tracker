#![allow(dead_code)]

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::fs;

use crate::error::AppError;

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
}
