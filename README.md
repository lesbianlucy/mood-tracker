# Kawaii Mood Webapp 🌸

Cozy Rust webapp (German UI copy) for mood tracking, drug/trip journaling, and safety tooling (panic button, Matrix notifications) with a kawaii aesthetic.

## Planned Features
- User accounts with registration/login and roles (user/admin).
- `/me` area: dashboard, mood check-ins (-5..+5), high-level scale (0..10), trip overview, panic/help, settings.
- Store all check-ins/trips/panic events as JSON under `ai/` + users table in SQLite.
- Per-user Matrix auto notifications for low mood or panic events.
- Admin panel with user management, system/git status, global templates.
- Cozy kawaii femboy UI rendered via Askama + Tailwind CSS.

## Tech Stack
- Rust 2021 with `axum` + `tokio` backend and `askama` templates.
- Auth via cookies/sessions, password hashing with `argon2`.
- SQLite + `sqlx` for relational data.
- JSON storage + auto commits via `git2`.
- `matrix-sdk` for notifications, `tracing` for logging.

## Project Structure
```
.
├── Cargo.toml
├── README.md
├── migrations/                # SQLx migrations (e.g., users table)
├── src/
│   ├── main.rs                # app bootstrap, router, state
│   ├── routes/                # public, user, admin
│   ├── services/              # storage, matrix, git
│   ├── models/                # user/checkin/trip/settings
│   └── ...
├── templates/                 # Askama HTML
├── static/                    # Tailwind inputs / assets
└── ai/                        # JSON data (gitignored)
```

## Development Setup
1. **Rust toolchain** – Stable works: `rustup default stable`.
2. **Fetch dependencies:**  
   ```bash
   cargo check
   ```
   (Downloads crates; database/Tailwind wiring comes later.)
3. **Create `.env`:** follow the sample (`DATABASE_URL=sqlite://mood.db`, `COOKIE_SECRET=...`).
4. **Run migrations:**  
   ```bash
   cargo sqlx migrate run   # or let the app run them on startup
   ```
5. **Prepare AI directory:**  
   ```bash
   mkdir -p ai/users ai/logs/panic_events
   cp ai.example/config.json ai/config.json   # once a sample exists
   ```
6. **Start the server:**  
   ```bash
   cargo run
   ```
7. **Tailwind build (upcoming):** placeholder CSS sits in `static/app.css`; Node/Tailwind CLI wiring will be added later.

## Continuous Integration
- **Rust CI** (`.github/workflows/ci.yml`): runs `cargo fmt`, `cargo clippy`, and `cargo test` on pushes/PRs with caching.
- **PR Gatekeepers** (`.github/workflows/pr-lint.yml`): enforces semantic PR titles and posts a checklist reminder.
- **PR Size Labels** (`.github/workflows/pr-size.yml`): automatically tags pull requests with size labels (XS–XXL).
- **PR Category Check** (`.github/workflows/pr-category.yml`): ensures each PR description declares a `Category:`.
- **Breaking Change Label** (`.github/workflows/breaking-change.yml`): syncs the `breaking-change` label when the PR title/body mentions `BREAKING CHANGE`.
- **First-Time Contributor Welcome** (`.github/workflows/first-time-contributor.yml`): greets new contributors with guidance.
- **New Account Label** (`.github/workflows/new-account-label.yml`): labels PRs from GitHub accounts younger than 30 days.

## Next Steps
- Implement auth flows (register/login) and real session middleware.
- Fill the JSON storage + Matrix service with full logic.
- Add a Tailwind build pipeline.
- Render templates with real data for user/admin views.

Have fun building 💖
