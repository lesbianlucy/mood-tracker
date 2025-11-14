#![allow(dead_code)]

use std::{fmt, fs::File, net::SocketAddr};

use anyhow::Context;
use cucumber::{given, then, when, World as _};
use mood::{
    auth::{self, AuthenticatedUser},
    config::AppConfig,
    db::init_pool,
    models::checkin::Checkin,
    services::{git::GitService, storage::StorageService},
    state::AppState,
};
use tempfile::TempDir;

#[derive(Debug, cucumber::World, Default)]
struct AppWorld {
    state: Option<TestState>,
    registered_user: Option<AuthenticatedUser>,
}

impl AppWorld {
    fn app_state(&self) -> &AppState {
        self.state
            .as_ref()
            .expect("state must be initialised first")
            .app()
    }
}

struct TestState {
    app: AppState,
    _root: TempDir,
}

impl fmt::Debug for TestState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestState").finish()
    }
}

impl TestState {
    async fn new() -> anyhow::Result<Self> {
        let root = TempDir::new().context("create temp dir for bdd world")?;
        let ai_root = root.path().join("ai");
        let repo_root = root.path().join("repo");
        std::fs::create_dir_all(&ai_root)?;
        std::fs::create_dir_all(&repo_root)?;

        let db_path = root.path().join("bdd.sqlite");
        File::create(&db_path)?;
        let database_url = format!("sqlite://{}", db_path.to_string_lossy());

        let config = AppConfig {
            database_url: database_url.clone(),
            listen_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
            ai_root: ai_root.clone(),
            repo_root: repo_root.clone(),
            cookie_secret: "bdd-cookie-secret".into(),
        };

        let db = init_pool(&config.database_url).await?;
        sqlx::migrate!("./migrations").run(&db).await?;

        let storage = StorageService::new(config.ai_root.clone());
        storage.ensure_structure().await?;

        let git = GitService::new(config.repo_root.clone());
        git.init_repo_if_needed()?;

        let app = AppState::new(config, db, storage, git);
        Ok(Self { app, _root: root })
    }

    fn app(&self) -> &AppState {
        &self.app
    }
}

#[given("a fresh application state")]
async fn given_fresh_state(world: &mut AppWorld) {
    world.state = Some(TestState::new().await.expect("state"));
    world.registered_user = None;
}

#[given(
    regex = r#"^a registered user \"([^\"]+)\" with email \"([^\"]+)\" and password \"([^\"]+)\"$"#
)]
async fn given_registered_user(
    world: &mut AppWorld,
    username: String,
    email: String,
    password: String,
) {
    register_user(world, username, email, password).await;
}

#[when(
    regex = r#"^I register a user \"([^\"]+)\" with email \"([^\"]+)\" and password \"([^\"]+)\"$"#
)]
async fn when_register_user(
    world: &mut AppWorld,
    username: String,
    email: String,
    password: String,
) {
    register_user(world, username, email, password).await;
}

#[then(regex = r#"^I can authenticate as \"([^\"]+)\" using password \"([^\"]+)\"$"#)]
async fn then_can_authenticate(world: &mut AppWorld, identifier: String, password: String) {
    let authed = auth::authenticate_user(world.app_state(), &identifier, &password)
        .await
        .expect("authentication");
    assert_eq!(authed.username, identifier);
}

#[when(regex = r#"^I submit a check-in with mood (-?\d+) and high (\d+) and notes \"([^\"]*)\"$"#)]
async fn when_submit_checkin(world: &mut AppWorld, mood: i32, high: i32, notes: String) {
    let user = world
        .registered_user
        .as_ref()
        .expect("user must exist before creating checkins");
    let mut checkin = Checkin::new(&user.uuid);
    checkin.mood = mood.clamp(-5, 5);
    checkin.high_level = high.clamp(0, 10);
    if !notes.trim().is_empty() {
        checkin.notes = Some(notes.clone());
    }
    world
        .app_state()
        .storage
        .append_checkin(&user.uuid, checkin)
        .await
        .expect("append checkin");
}

#[then(regex = r"^the user has (\d+) stored check-ins$")]
async fn then_user_has_checkins(world: &mut AppWorld, expected: usize) {
    let user = world
        .registered_user
        .as_ref()
        .expect("user must exist before assertions");
    let checkins = world
        .app_state()
        .storage
        .load_user_checkins(&user.uuid)
        .await
        .expect("load checkins");
    assert_eq!(checkins.len(), expected);
}

#[then(regex = r"^the latest stored check-in has mood (-?\d+) and high (\d+)$")]
async fn then_latest_has_values(world: &mut AppWorld, mood: i32, high: i32) {
    let user = world
        .registered_user
        .as_ref()
        .expect("user must exist before assertions");
    let mut checkins = world
        .app_state()
        .storage
        .load_user_checkins(&user.uuid)
        .await
        .expect("load checkins");
    checkins.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    let latest = checkins.first().expect("at least one checkin expected");
    assert_eq!(latest.mood, mood);
    assert_eq!(latest.high_level, high);
}

async fn register_user(world: &mut AppWorld, username: String, email: String, password: String) {
    let created = auth::register_user(world.app_state(), &username, &email, &password)
        .await
        .expect("register user");
    world.registered_user = Some(created);
}

#[tokio::main]
async fn main() {
    AppWorld::cucumber()
        .fail_on_skipped()
        .with_default_cli()
        .run("tests/features")
        .await;
}
