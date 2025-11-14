use std::{env, net::SocketAddr, path::PathBuf};

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub listen_addr: SocketAddr,
    pub ai_root: PathBuf,
    pub repo_root: PathBuf,
    pub cookie_secret: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://mood.db".to_string());
        let listen_addr: SocketAddr = env::var("APP_LISTEN_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:3000".to_string())
            .parse()
            .map_err(|err| AppError::Config(format!("invalid APP_LISTEN_ADDR: {err}")))?;

        let ai_root = env::var("AI_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("ai"));

        let repo_root = env::var("REPO_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::current_dir().expect("cwd should exist when building config")
            });

        let cookie_secret = env::var("COOKIE_SECRET")
            .unwrap_or_else(|_| "change-me-super-secret-kawaii-cookie".to_string());

        Ok(Self {
            database_url,
            listen_addr,
            ai_root,
            repo_root,
            cookie_secret,
        })
    }
}
