use askama::Template;
use askama_axum::IntoResponse as AskamaTemplateResponse;
use axum::{
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Router,
};
use chrono::Utc;

use crate::{auth::CurrentUser, error::AppError, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(dashboard))
        .route("/checkins", get(checkins_list))
        .route(
            "/checkins/new",
            get(checkin_new_form).post(checkin_new_submit),
        )
        .route("/checkins/:id", get(checkin_detail))
        .route("/trips", get(trips_list))
        .route("/panic", get(panic_page))
        .route("/panic/trigger", post(panic_trigger))
        .route("/settings", get(settings_form).post(settings_submit))
}

#[derive(Template)]
#[template(path = "user/dashboard.html")]
struct DashboardTemplate {
    display_name: String,
}

async fn dashboard(current: CurrentUser) -> Result<impl IntoResponse, AppError> {
    let user = current.require_user()?;
    Ok(AskamaTemplateResponse::into_response(DashboardTemplate {
        display_name: user.username.clone(),
    }))
}

#[derive(Clone)]
struct CheckinSummary {
    id: String,
    timestamp: String,
    mood: i32,
    high_level: i32,
}

#[derive(Template)]
#[template(path = "user/checkins_list.html")]
struct CheckinsListTemplate {
    checkins: Vec<CheckinSummary>,
}

async fn checkins_list(current: CurrentUser) -> Result<impl IntoResponse, AppError> {
    current.require_user()?;
    let items = vec![CheckinSummary {
        id: "demo".into(),
        timestamp: Utc::now().to_rfc3339(),
        mood: 1,
        high_level: 2,
    }];
    Ok(AskamaTemplateResponse::into_response(
        CheckinsListTemplate { checkins: items },
    ))
}

#[derive(Template)]
#[template(path = "user/checkin_new.html")]
struct CheckinNewTemplate;

async fn checkin_new_form(current: CurrentUser) -> Result<impl IntoResponse, AppError> {
    current.require_user()?;
    Ok(AskamaTemplateResponse::into_response(CheckinNewTemplate))
}

async fn checkin_new_submit(current: CurrentUser) -> Result<Redirect, AppError> {
    current.require_user()?;
    Err(AppError::NotImplemented)
}

#[derive(Template)]
#[template(path = "user/checkin_detail.html")]
struct CheckinDetailTemplate {
    mood: i32,
    high_level: i32,
    notes: String,
    raw_json: String,
}

async fn checkin_detail(current: CurrentUser) -> Result<impl IntoResponse, AppError> {
    current.require_user()?;
    let raw = serde_json::json!({"id": "demo", "mood": 0, "high_level": 0, "notes": "Demo"});
    Ok(AskamaTemplateResponse::into_response(
        CheckinDetailTemplate {
            mood: 0,
            high_level: 0,
            notes: "Demo".into(),
            raw_json: serde_json::to_string_pretty(&raw).unwrap_or_else(|_| "{}".into()),
        },
    ))
}

#[derive(Template)]
#[template(path = "user/trips_list.html")]
struct TripsListTemplate;

async fn trips_list(current: CurrentUser) -> Result<impl IntoResponse, AppError> {
    current.require_user()?;
    Ok(AskamaTemplateResponse::into_response(TripsListTemplate))
}

#[derive(Template)]
#[template(path = "user/panic.html")]
struct PanicTemplate;

async fn panic_page(current: CurrentUser) -> Result<impl IntoResponse, AppError> {
    current.require_user()?;
    Ok(AskamaTemplateResponse::into_response(PanicTemplate))
}

async fn panic_trigger(current: CurrentUser) -> Result<Redirect, AppError> {
    current.require_user()?;
    Err(AppError::NotImplemented)
}

#[derive(Template)]
#[template(path = "user/settings.html")]
struct SettingsTemplate;

async fn settings_form(current: CurrentUser) -> Result<impl IntoResponse, AppError> {
    current.require_user()?;
    Ok(AskamaTemplateResponse::into_response(SettingsTemplate))
}

async fn settings_submit(current: CurrentUser) -> Result<Redirect, AppError> {
    current.require_user()?;
    Err(AppError::NotImplemented)
}
