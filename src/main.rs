use mood::config::AppConfig;
use mood::db::init_pool;
use mood::error::AppError;
use mood::routes::create_router;
use mood::services::{git::GitService, matrix::MatrixService, storage::StorageService};
use mood::state::AppState;
use tokio::net::TcpListener;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenvy::dotenv().ok();
    init_logging();

    let config = AppConfig::from_env()?;
    let db = init_pool(&config.database_url).await?;

    if let Err(err) = sqlx::migrate!("./migrations").run(&db).await {
        error!("migration failed: {err:?}");
        return Err(AppError::Other(err.into()));
    }

    let storage = StorageService::new(config.ai_root.clone());
    storage.ensure_structure().await?;

    let git = GitService::new(config.repo_root.clone());
    git.init_repo_if_needed()?;

    let matrix = MatrixService::new();

    let state = AppState::new(
        config.clone(),
        db.clone(),
        storage.clone(),
        git.clone(),
        matrix.clone(),
    );

    let app = create_router(state.clone());

    let listener = TcpListener::bind(config.listen_addr).await?;
    info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

fn init_logging() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);
    let filter_layer = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,kawaii_mood=debug".into());

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
}
