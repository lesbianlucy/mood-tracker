use askama::Template;
use askama_axum::IntoResponse as AskamaTemplateResponse;
use axum::{
    response::{IntoResponse, Redirect},
    routing::get,
    Router,
};

use crate::{auth::CurrentUser, error::AppError, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(dashboard))
        .route("/users", get(users_list))
        .route("/users/:id", get(user_detail))
        .route("/system", get(system_page).post(system_commit))
        .route("/settings", get(settings_form).post(settings_submit))
}

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
struct AdminDashboardTemplate;

async fn dashboard(current: CurrentUser) -> Result<impl IntoResponse, AppError> {
    current.require_admin()?;
    Ok(AskamaTemplateResponse::into_response(
        AdminDashboardTemplate,
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
    username: String,
    email: String,
    role: String,
    uuid: String,
}

async fn users_list(current: CurrentUser) -> Result<impl IntoResponse, AppError> {
    current.require_admin()?;
    Ok(AskamaTemplateResponse::into_response(AdminUsersTemplate {
        users: vec![AdminUserRow {
            id: 1,
            username: "cutie".into(),
            email: "cutie@example.com".into(),
            role: "admin".into(),
            uuid: "demo".into(),
        }],
    }))
}

#[derive(Template)]
#[template(path = "admin/user_detail.html")]
struct AdminUserDetailTemplate {
    user: AdminUserRow,
}

async fn user_detail(current: CurrentUser) -> Result<impl IntoResponse, AppError> {
    current.require_admin()?;
    Ok(AskamaTemplateResponse::into_response(
        AdminUserDetailTemplate {
            user: AdminUserRow {
                id: 1,
                username: "cutie".into(),
                email: "cutie@example.com".into(),
                role: "admin".into(),
                uuid: "demo".into(),
            },
        },
    ))
}

#[derive(Template)]
#[template(path = "admin/system.html")]
struct AdminSystemTemplate;

async fn system_page(current: CurrentUser) -> Result<impl IntoResponse, AppError> {
    current.require_admin()?;
    Ok(AskamaTemplateResponse::into_response(AdminSystemTemplate))
}

async fn system_commit(current: CurrentUser) -> Result<Redirect, AppError> {
    current.require_admin()?;
    Err(AppError::NotImplemented)
}

#[derive(Template)]
#[template(path = "admin/settings.html")]
struct AdminSettingsTemplate;

async fn settings_form(current: CurrentUser) -> Result<impl IntoResponse, AppError> {
    current.require_admin()?;
    Ok(AskamaTemplateResponse::into_response(AdminSettingsTemplate))
}

async fn settings_submit(current: CurrentUser) -> Result<Redirect, AppError> {
    current.require_admin()?;
    Err(AppError::NotImplemented)
}
