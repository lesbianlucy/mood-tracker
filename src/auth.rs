#![allow(dead_code)]

use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts};

use crate::{error::AppError, models::user::UserRole};

pub const SESSION_COOKIE: &str = "kawaii_session";

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: i64,
    pub uuid: String,
    pub username: String,
    pub role: UserRole,
}

#[derive(Debug, Clone, Default)]
pub struct CurrentUser(pub Option<AuthenticatedUser>);

#[async_trait]
impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Sobald echte Sessions implementiert sind, lesen wir hier Cookies/DB.
        if let Some(user) = parts.extensions.get::<AuthenticatedUser>() {
            return Ok(Self(Some(user.clone())));
        }

        Ok(Self(None))
    }
}

impl CurrentUser {
    pub fn require_user(&self) -> Result<&AuthenticatedUser, AppError> {
        self.0.as_ref().ok_or(AppError::Unauthorized)
    }

    pub fn require_admin(&self) -> Result<&AuthenticatedUser, AppError> {
        let user = self.require_user()?;
        if user.role == UserRole::Admin {
            Ok(user)
        } else {
            Err(AppError::Forbidden)
        }
    }
}

pub fn unauthorized() -> AppError {
    AppError::Unauthorized
}

pub fn forbidden() -> AppError {
    AppError::Forbidden
}
