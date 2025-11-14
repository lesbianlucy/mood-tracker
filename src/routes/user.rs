use askama::Template;
use askama_axum::IntoResponse as AskamaTemplateResponse;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Form, Router,
};
use chrono::{Local, Utc};
use serde::Deserialize;

use crate::{auth::CurrentUser, error::AppError, models::checkin::Checkin, state::AppState};

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

async fn checkins_list(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let user = current.require_user()?;
    let mut items = state.storage.load_user_checkins(&user.uuid).await?;
    items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    let summaries = items
        .into_iter()
        .map(|checkin| CheckinSummary {
            id: checkin.id,
            timestamp: format_timestamp(checkin.timestamp),
            mood: checkin.mood,
            high_level: checkin.high_level,
        })
        .collect();
    Ok(AskamaTemplateResponse::into_response(
        CheckinsListTemplate {
            checkins: summaries,
        },
    ))
}

#[derive(Template)]
#[template(path = "user/checkin_new.html")]
struct CheckinNewTemplate;

async fn checkin_new_form(current: CurrentUser) -> Result<impl IntoResponse, AppError> {
    current.require_user()?;
    Ok(AskamaTemplateResponse::into_response(CheckinNewTemplate))
}

#[derive(Deserialize)]
struct CheckinForm {
    mood: i32,
    high_level: i32,
    safety_answer: Option<String>,
    notes: Option<String>,
}

async fn checkin_new_submit(
    State(state): State<AppState>,
    current: CurrentUser,
    Form(form): Form<CheckinForm>,
) -> Result<Redirect, AppError> {
    let user = current.require_user()?;
    let mut checkin = Checkin::new(&user.uuid);
    checkin.mood = form.mood.clamp(-5, 5);
    checkin.high_level = form.high_level.clamp(0, 10);
    checkin.safety_answer = normalize_optional(form.safety_answer);
    checkin.notes = normalize_optional(form.notes);
    checkin.feels_safe = checkin
        .safety_answer
        .as_ref()
        .map(|answer| !answer.to_lowercase().contains("nein"))
        .unwrap_or(true);

    let saved = state.storage.append_checkin(&user.uuid, checkin).await?;

    Ok(Redirect::to(&format!("/me/checkins/{}", saved.id)))
}

#[derive(Template)]
#[template(path = "user/checkin_detail.html")]
struct CheckinDetailTemplate {
    mood: i32,
    high_level: i32,
    notes: String,
    raw_json: String,
}

async fn checkin_detail(
    State(state): State<AppState>,
    current: CurrentUser,
    Path(checkin_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let user = current.require_user()?;
    let items = state.storage.load_user_checkins(&user.uuid).await?;
    let checkin = items
        .into_iter()
        .find(|c| c.id == checkin_id)
        .ok_or(AppError::NotFound)?;
    let raw_json =
        serde_json::to_string_pretty(&checkin).map_err(|err| AppError::Other(err.into()))?;
    Ok(AskamaTemplateResponse::into_response(
        CheckinDetailTemplate {
            mood: checkin.mood,
            high_level: checkin.high_level,
            notes: checkin
                .notes
                .unwrap_or_else(|| "Keine Notizen hinterlegt 🌱".into()),
            raw_json,
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

fn normalize_optional(input: Option<String>) -> Option<String> {
    input.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn format_timestamp(ts: chrono::DateTime<Utc>) -> String {
    ts.with_timezone(&Local)
        .format("%d.%m.%Y %H:%M")
        .to_string()
}
