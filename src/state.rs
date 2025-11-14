#![allow(dead_code)]

use axum_extra::extract::cookie::Key;
use sha2::{Digest, Sha512};

use crate::{
    config::AppConfig,
    db::DbPool,
    services::{git::GitService, storage::StorageService},
};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub db: DbPool,
    pub storage: StorageService,
    pub git: GitService,
    pub cookie_key: Key,
}

impl AppState {
    pub fn new(config: AppConfig, db: DbPool, storage: StorageService, git: GitService) -> Self {
        let digest = Sha512::digest(config.cookie_secret.as_bytes());
        let cookie_key = Key::from(&digest[..]);
        Self {
            config,
            db,
            storage,
            git,
            cookie_key,
        }
    }
}
