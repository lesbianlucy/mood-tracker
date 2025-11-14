use askama::Template;
use askama_axum::IntoResponse as AskamaTemplateResponse;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

use crate::{
    auth::{self, CurrentUser},
    error::AppError,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(landing))
        .route("/login", get(login_form).post(login_submit))
        .route("/register", get(register_form).post(register_submit))
        .route("/logout", post(logout))
}

#[derive(Template)]
#[template(path = "landing.html")]
struct LandingTemplate {
    logged_in: bool,
}

async fn landing(current: CurrentUser) -> impl IntoResponse {
    AskamaTemplateResponse::into_response(LandingTemplate {
        logged_in: current.0.is_some(),
    })
}

#[derive(Template)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate {
    show_error: bool,
    error_message: String,
    identifier: String,
}

async fn login_form() -> impl IntoResponse {
    AskamaTemplateResponse::into_response(LoginTemplate {
        show_error: false,
        error_message: String::new(),
        identifier: String::new(),
    })
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
) -> Result<Response, AppError> {
    match auth::authenticate_user(&state, &form.identifier, &form.password).await {
        Ok(user) => {
            let session_id = auth::create_session(&state, user.id).await?;
            Ok((
                auth::apply_session_cookie(jar, &session_id),
                Redirect::to("/me"),
            )
                .into_response())
        }
        Err(AppError::Unauthorized) => Ok(render_login_error(
            form.identifier,
            "Login fehlgeschlagen 😿 – bitte prüfe deine Daten.".into(),
        )),
        Err(AppError::BadRequest(msg)) => Ok(render_login_error(form.identifier, msg)),
        Err(err) => Err(err),
    }
}

fn render_login_error(identifier: String, message: String) -> Response {
    (
        StatusCode::BAD_REQUEST,
        AskamaTemplateResponse::into_response(LoginTemplate {
            show_error: true,
            error_message: message,
            identifier,
        }),
    )
        .into_response()
}

#[derive(Template)]
#[template(path = "auth/register.html")]
pub struct RegisterTemplate {
    show_error: bool,
    error_message: String,
    username: String,
    email: String,
}

async fn register_form() -> impl IntoResponse {
    AskamaTemplateResponse::into_response(RegisterTemplate {
        show_error: false,
        error_message: String::new(),
        username: String::new(),
        email: String::new(),
    })
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
) -> Result<Response, AppError> {
    if form.password != form.password_confirm {
        return Ok(render_register_error(
            form.username,
            form.email,
            "Die Passwörter stimmen nicht überein 💔".into(),
        ));
    }

    match auth::register_user(&state, &form.username, &form.email, &form.password).await {
        Ok(user) => {
            let session_id = auth::create_session(&state, user.id).await?;
            Ok((
                auth::apply_session_cookie(jar, &session_id),
                Redirect::to("/me"),
            )
                .into_response())
        }
        Err(AppError::BadRequest(msg)) => Ok(render_register_error(form.username, form.email, msg)),
        Err(err) => Err(err),
    }
}

fn render_register_error(username: String, email: String, message: String) -> Response {
    (
        StatusCode::BAD_REQUEST,
        AskamaTemplateResponse::into_response(RegisterTemplate {
            show_error: true,
            error_message: message,
            username,
            email,
        }),
    )
        .into_response()
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
