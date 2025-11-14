use askama::Template;
use askama_axum::IntoResponse as AskamaTemplateResponse;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect},
    routing::get,
    Form, Router,
};
use chrono::{DateTime, Local, Utc};
use serde::Deserialize;
use sqlx::Row;
use tracing::warn;

use crate::{auth::CurrentUser, error::AppError, models::settings::GlobalConfig, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(dashboard))
        .route("/users", get(users_list))
        .route("/users/:id", get(user_detail).post(update_user_role))
        .route("/system", get(system_page).post(system_commit))
        .route("/settings", get(settings_form).post(settings_submit))
}

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
struct AdminDashboardTemplate {
    user_count: i64,
    checkin_count: usize,
    panic_count: usize,
    panic_events: Vec<PanicEventSummary>,
}

#[derive(Clone)]
struct PanicEventSummary {
    when: String,
    username: String,
    mood_text: String,
}

async fn dashboard(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    current.require_admin()?;
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;
    let checkin_count = state.storage.count_all_checkins().await?;
    let mut events = state.storage.list_panic_events().await?;
    let panic_count = events.len();
    events.truncate(5);
    let summaries = events
        .into_iter()
        .map(|event| PanicEventSummary {
            when: format_timestamp(event.timestamp),
            username: event.user_uuid,
            mood_text: event
                .mood_at_panic
                .map(|m| m.to_string())
                .unwrap_or_else(|| "?".into()),
        })
        .collect();
    Ok(AskamaTemplateResponse::into_response(
        AdminDashboardTemplate {
            user_count,
            checkin_count,
            panic_count,
            panic_events: summaries,
        },
    ))
}

#[derive(Template)]
#[template(path = "admin/users_list.html")]
struct AdminUsersTemplate {
    users: Vec<AdminUserRow>,
}

#[derive(Clone)]
struct AdminUserRow {
    id: i64,
    uuid: String,
    username: String,
    email: String,
    role: String,
    created_at: String,
    last_login_at: String,
}

async fn users_list(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    current.require_admin()?;
    let rows = sqlx::query(
        r#"SELECT id, uuid, username, email, role, created_at, last_login_at FROM users ORDER BY created_at DESC"#,
    )
    .fetch_all(&state.db)
    .await?;
    let users = rows
        .into_iter()
        .map(|row| AdminUserRow {
            id: row.get("id"),
            uuid: row.get("uuid"),
            username: row.get("username"),
            email: row.get("email"),
            role: row.get("role"),
            created_at: format_datetime(row.get::<String, _>("created_at")),
            last_login_at: row
                .get::<Option<String>, _>("last_login_at")
                .map(|ts| format_datetime(ts))
                .unwrap_or_else(|| "–".into()),
        })
        .collect();
    Ok(AskamaTemplateResponse::into_response(AdminUsersTemplate {
        users,
    }))
}

#[derive(Template)]
#[template(path = "admin/user_detail.html")]
struct AdminUserDetailTemplate {
    user: AdminUserRow,
    checkin_count: usize,
    panic_events: usize,
    has_mood_avg: bool,
    mood_avg_text: String,
}

async fn user_detail(
    State(state): State<AppState>,
    current: CurrentUser,
    Path(user_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    current.require_admin()?;
    let row = sqlx::query( "SELECT id, uuid, username, email, role, created_at, last_login_at FROM users WHERE id = ?1" )
        .bind(user_id)
        .fetch_optional(&state.db)
        .await?;
    let Some(row) = row else {
        return Err(AppError::NotFound);
    };
    let user_row = AdminUserRow {
        id: row.get("id"),
        uuid: row.get("uuid"),
        username: row.get("username"),
        email: row.get("email"),
        role: row.get("role"),
        created_at: format_datetime(row.get::<String, _>("created_at")),
        last_login_at: row
            .get::<Option<String>, _>("last_login_at")
            .map(|ts| format_datetime(ts))
            .unwrap_or_else(|| "–".into()),
    };
    let checkins = state.storage.list_checkins(&user_row.uuid).await?;
    let panic_events = state
        .storage
        .count_user_panic_events(&user_row.uuid)
        .await?;
    let (has_mood_avg, mood_avg_text) = if checkins.is_empty() {
        (false, String::new())
    } else {
        let avg = checkins.iter().map(|c| c.mood).sum::<i32>() as f32 / checkins.len() as f32;
        (true, format!("{:.1}", avg))
    };
    Ok(AskamaTemplateResponse::into_response(
        AdminUserDetailTemplate {
            user: user_row,
            checkin_count: checkins.len(),
            panic_events,
            has_mood_avg,
            mood_avg_text,
        },
    ))
}

#[derive(Deserialize)]
struct RoleForm {
    role: String,
}

async fn update_user_role(
    State(state): State<AppState>,
    current: CurrentUser,
    Path(user_id): Path<i64>,
    Form(form): Form<RoleForm>,
) -> Result<Redirect, AppError> {
    current.require_admin()?;
    if !matches!(form.role.as_str(), "user" | "admin") {
        return Err(AppError::BadRequest("Ungültige Rolle".into()));
    }
    sqlx::query("UPDATE users SET role = ?1 WHERE id = ?2")
        .bind(&form.role)
        .bind(user_id)
        .execute(&state.db)
        .await?;
    Ok(Redirect::to(&format!("/admin/users/{user_id}")))
}

#[derive(Template)]
#[template(path = "admin/system.html")]
struct AdminSystemTemplate {
    branch: String,
    pending_ai: bool,
    has_commit: bool,
    commit: GitCommit,
}

#[derive(Clone, Default)]
struct GitCommit {
    hash: String,
    message: String,
    timestamp: String,
}

async fn system_page(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    current.require_admin()?;
    let status = state.git.status()?;
    let (has_commit, commit) = if let Some(info) = status.head {
        (
            true,
            GitCommit {
                hash: info.hash,
                message: info.message,
                timestamp: info.timestamp,
            },
        )
    } else {
        (false, GitCommit::default())
    };
    Ok(AskamaTemplateResponse::into_response(AdminSystemTemplate {
        branch: status.branch,
        pending_ai: status.pending_ai_changes,
        has_commit,
        commit,
    }))
}

async fn system_commit(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<Redirect, AppError> {
    current.require_admin()?;
    if let Err(err) = state
        .git
        .commit_ai_changes("chore: manueller Commit aus Admin-Panel 💾")
    {
        warn!("Git Commit im Admin Panel fehlgeschlagen: {err}");
    }
    Ok(Redirect::to("/admin/system"))
}

#[derive(Template)]
#[template(path = "admin/settings.html")]
struct AdminSettingsTemplate {
    config: GlobalConfig,
}

#[derive(Deserialize)]
struct GlobalSettingsForm {
    default_low_mood_threshold: i32,
    default_auto_notify_on_low_mood: Option<String>,
    low_mood_message_template: String,
    panic_message_template: String,
}

async fn settings_form(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    current.require_admin()?;
    let config = state.storage.load_global_config().await?;
    Ok(AskamaTemplateResponse::into_response(
        AdminSettingsTemplate { config },
    ))
}

async fn settings_submit(
    State(state): State<AppState>,
    current: CurrentUser,
    Form(form): Form<GlobalSettingsForm>,
) -> Result<Redirect, AppError> {
    current.require_admin()?;
    let mut config = state.storage.load_global_config().await?;
    config.default_low_mood_threshold = form.default_low_mood_threshold;
    config.default_auto_notify_on_low_mood = form.default_auto_notify_on_low_mood.is_some();
    config.low_mood_message_template = form.low_mood_message_template;
    config.panic_message_template = form.panic_message_template;
    state.storage.save_global_config(&config).await?;
    if let Err(err) = state
        .git
        .commit_ai_changes("chore: globale Einstellungen aktualisiert ✨")
    {
        warn!("Git Commit für globale Einstellungen fehlgeschlagen: {err}");
    }
    Ok(Redirect::to("/admin/settings"))
}

fn format_timestamp(ts: DateTime<Utc>) -> String {
    ts.with_timezone(&Local)
        .format("%d.%m.%Y %H:%M")
        .to_string()
}

fn format_datetime(raw: String) -> String {
    DateTime::parse_from_rfc3339(&raw)
        .map(|dt| {
            dt.with_timezone(&Local)
                .format("%d.%m.%Y %H:%M")
                .to_string()
        })
        .unwrap_or(raw)
}
