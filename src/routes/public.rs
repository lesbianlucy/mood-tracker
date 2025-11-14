use askama::Template;
use askama_axum::IntoResponse as AskamaTemplateResponse;
use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Form, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

use crate::{auth, error::AppError, state::AppState};

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

#[derive(Deserialize)]
struct LoginForm {
    identifier: String,
    password: String,
}

async fn login_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> Result<(CookieJar, Redirect), AppError> {
    let user = auth::authenticate_user(&state, &form.identifier, &form.password).await?;
    let session_id = auth::create_session(&state, user.id).await?;
    Ok((
        auth::apply_session_cookie(jar, &session_id),
        Redirect::to("/me"),
    ))
}

#[derive(Template)]
#[template(path = "auth/register.html")]
pub struct RegisterTemplate;

async fn register_form() -> impl IntoResponse {
    AskamaTemplateResponse::into_response(RegisterTemplate)
}

#[derive(Deserialize)]
struct RegisterForm {
    username: String,
    email: String,
    password: String,
    password_confirm: String,
}

async fn register_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<RegisterForm>,
) -> Result<(CookieJar, Redirect), AppError> {
    if form.password != form.password_confirm {
        return Err(AppError::BadRequest(
            "Passwörter stimmen nicht überein.".into(),
        ));
    }

    let user = auth::register_user(&state, &form.username, &form.email, &form.password).await?;
    let session_id = auth::create_session(&state, user.id).await?;
    Ok((
        auth::apply_session_cookie(jar, &session_id),
        Redirect::to("/me"),
    ))
}

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), AppError> {
    if let Some(cookie) = jar.get(auth::SESSION_COOKIE) {
        auth::destroy_session(&state, cookie.value()).await?;
    }
    Ok((auth::clear_session_cookie(jar), Redirect::to("/")))
}
