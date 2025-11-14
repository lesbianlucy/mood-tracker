pub mod admin;
pub mod public;
pub mod user;

use axum::Router;
use tower_http::services::ServeDir;

use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .merge(public::router())
        .nest("/me", user::router())
        .nest("/admin", admin::router())
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
}
