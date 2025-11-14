use askama::Template;
use askama_axum::IntoResponse as AskamaTemplateResponse;
use axum::{
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Router,
};

use crate::error::AppError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(landing))
        .route("/login", get(login_form).post(login_submit))
        .route("/register", get(register_form).post(register_submit))
        .route("/logout", post(logout))
}

#[derive(Template)]
#[template(path = "landing.html")]
struct LandingTemplate;

async fn landing() -> impl IntoResponse {
    AskamaTemplateResponse::into_response(LandingTemplate)
}

#[derive(Template)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate;

async fn login_form() -> impl IntoResponse {
    AskamaTemplateResponse::into_response(LoginTemplate)
}

async fn login_submit() -> Result<Redirect, AppError> {
    Err(AppError::NotImplemented)
}

#[derive(Template)]
#[template(path = "auth/register.html")]
pub struct RegisterTemplate;

async fn register_form() -> impl IntoResponse {
    AskamaTemplateResponse::into_response(RegisterTemplate)
}

async fn register_submit() -> Result<Redirect, AppError> {
    Err(AppError::NotImplemented)
}

async fn logout() -> Result<Redirect, AppError> {
    Err(AppError::NotImplemented)
}
