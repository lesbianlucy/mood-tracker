use askama::Template;
use askama_axum::IntoResponse as AskamaTemplateResponse;
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Form, Router,
};
use chrono::{DateTime, Duration, Local, TimeZone, Utc};
use serde::Deserialize;
use tracing::warn;
use uuid::Uuid;

use crate::{
    auth::CurrentUser,
    error::AppError,
    models::checkin::{AutoNotifications, Checkin, DrugEntry, PanicEvent},
    models::settings::UserConfig,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(dashboard))
        .route("/mood", get(mood_widget_page))
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
        .route("/settings/matrix-test", post(settings_matrix_test))
}

#[derive(Template)]
#[template(path = "user/dashboard.html")]
struct DashboardTemplate {
    display_name: String,
    has_last_checkin: bool,
    last_checkin: CheckinSummary,
    has_average: bool,
    average_text: String,
    widget_html: String,
    total_checkins: usize,
}

async fn dashboard(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let user = current.require_user()?;
    let checkins = state.storage.list_checkins(&user.uuid).await?;
    let user_cfg = state
        .storage
        .load_user_config(&user.uuid)
        .await
        .unwrap_or_else(|_| UserConfig::for_new_user(&user.username));

    let widget_html = build_mood_widget(&checkins)?;
    let (has_avg, avg_text) = average_mood_text(&checkins);
    let (has_last_checkin, last_checkin) = if let Some(first) = checkins.first() {
        (true, CheckinSummary::from(first))
    } else {
        (false, CheckinSummary::default())
    };

    Ok(AskamaTemplateResponse::into_response(DashboardTemplate {
        display_name: user_cfg.display_name,
        has_last_checkin,
        last_checkin,
        has_average: has_avg,
        average_text: avg_text,
        widget_html,
        total_checkins: checkins.len(),
    }))
}

#[derive(Template)]
#[template(path = "user/mood_widget.html")]
struct MoodWidgetTemplate {
    mood_text: String,
    high_text: String,
    high_percent: i32,
    has_danger: bool,
    danger_text: String,
    sparkline: Vec<MoodSparkPoint>,
    scale: Vec<MoodScaleMark>,
}

#[derive(Clone)]
struct MoodSparkPoint {
    label: String,
    x: f32,
    y: f32,
    has_prev: bool,
    prev_x: f32,
    prev_y: f32,
}

#[derive(Clone)]
struct MoodScaleMark {
    value: i32,
    active: bool,
}

fn build_mood_widget(checkins: &[Checkin]) -> Result<String, AppError> {
    let latest = checkins.first();
    let danger_text = latest.and_then(|c| danger_message(c.mood, c.high_level));
    let (has_danger, danger_text) = if let Some(text) = danger_text {
        (true, text)
    } else {
        (false, String::new())
    };
    let raw_points: Vec<_> = checkins
        .iter()
        .take(10)
        .rev()
        .map(|c| (c.timestamp, c.mood))
        .collect();
    let count = raw_points.len();
    let step = if count > 1 {
        240.0 / (count as f32 - 1.0)
    } else {
        0.0
    };
    let mut sparkline = Vec::new();
    for (idx, (ts, mood)) in raw_points.iter().enumerate() {
        let x = idx as f32 * step;
        let y = 30.0 - (*mood as f32 * 5.0);
        let (has_prev, prev_x, prev_y) = if idx > 0 {
            let prev_mood = raw_points[idx - 1].1;
            let px = (idx - 1) as f32 * step;
            let py = 30.0 - (prev_mood as f32 * 5.0);
            (true, px, py)
        } else {
            (false, 0.0, 0.0)
        };
        sparkline.push(MoodSparkPoint {
            label: ts.with_timezone(&Local).format("%d.%m.%H:%M").to_string(),
            x,
            y,
            has_prev,
            prev_x,
            prev_y,
        });
    }
    let widget = MoodWidgetTemplate {
        mood_text: latest
            .map(|c| c.mood.to_string())
            .unwrap_or_else(|| "?".into()),
        high_text: latest
            .map(|c| format!("{}/10", c.high_level))
            .unwrap_or_else(|| "0/10".into()),
        high_percent: latest.map(|c| c.high_level * 10).unwrap_or(0),
        has_danger,
        danger_text,
        sparkline,
        scale: (-5..=5)
            .map(|value| MoodScaleMark {
                value,
                active: latest.map(|c| c.mood == value).unwrap_or(false),
            })
            .collect(),
    };
    Ok(widget.render().map_err(|err| AppError::Other(err.into()))?)
}

#[derive(Template)]
#[template(path = "user/mood_page.html")]
struct MoodPageTemplate {
    display_name: String,
    widget_html: String,
}

async fn mood_widget_page(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let user = current.require_user()?;
    let checkins = state.storage.list_checkins(&user.uuid).await?;
    let cfg = state
        .storage
        .load_user_config(&user.uuid)
        .await
        .unwrap_or_else(|_| UserConfig::for_new_user(&user.username));
    let widget_html = build_mood_widget(&checkins)?;
    Ok(AskamaTemplateResponse::into_response(MoodPageTemplate {
        display_name: cfg.display_name,
        widget_html,
    }))
}

#[derive(Template)]
#[template(path = "user/checkins_list.html")]
struct CheckinsListTemplate {
    checkins: Vec<CheckinSummary>,
}

#[derive(Clone)]
struct CheckinSummary {
    id: String,
    timestamp: String,
    high_level: i32,
    mood_label: String,
}

impl Default for CheckinSummary {
    fn default() -> Self {
        Self {
            id: String::new(),
            timestamp: String::new(),
            high_level: 0,
            mood_label: String::new(),
        }
    }
}

impl From<&Checkin> for CheckinSummary {
    fn from(value: &Checkin) -> Self {
        Self {
            id: value.id.clone(),
            timestamp: format_timestamp(value.timestamp),
            high_level: value.high_level,
            mood_label: mood_label(value.mood),
        }
    }
}

async fn checkins_list(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let user = current.require_user()?;
    let checkins = state.storage.list_checkins(&user.uuid).await?;
    let summaries = checkins.iter().map(CheckinSummary::from).collect();
    Ok(AskamaTemplateResponse::into_response(
        CheckinsListTemplate {
            checkins: summaries,
        },
    ))
}

#[derive(Template)]
#[template(path = "user/checkin_new.html")]
struct CheckinNewTemplate {
    status_options: Vec<StatusOption>,
    route_options: Vec<&'static str>,
}

struct StatusOption {
    value: &'static str,
    label: &'static str,
    description: &'static str,
}

async fn checkin_new_form(current: CurrentUser) -> Result<impl IntoResponse, AppError> {
    current.require_user()?;
    Ok(AskamaTemplateResponse::into_response(CheckinNewTemplate {
        status_options: status_options(),
        route_options: route_options(),
    }))
}

#[derive(Deserialize)]
struct CheckinForm {
    mood: i32,
    high_level: i32,
    safety_answer: String,
    notes: Option<String>,
    #[serde(default)]
    status_tags: Vec<String>,
    #[serde(default)]
    drugs_substance: Vec<String>,
    #[serde(default)]
    drugs_dose: Vec<String>,
    #[serde(default)]
    drugs_route: Vec<String>,
    #[serde(default)]
    drugs_start_time: Vec<String>,
    #[serde(default)]
    drugs_notes: Vec<String>,
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
    checkin.status_tags = form.status_tags.clone();
    checkin.safety_answer = Some(match form.safety_answer.as_str() {
        "ok" => "Mir geht's gut 💖".into(),
        "high" => "Ich bin sehr high, aber komme klar 🌀".into(),
        _ => "Ich glaube, ich brauche Hilfe 😰".into(),
    });
    checkin.feels_safe = form.safety_answer != "panic";
    checkin.notes = form
        .notes
        .as_ref()
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());
    checkin.drugs = build_drug_entries(&form);

    let global_cfg = state.storage.load_global_config().await?;
    let user_cfg = state
        .storage
        .load_user_config(&user.uuid)
        .await
        .unwrap_or_else(|_| UserConfig::for_new_user(&user.username));

    let mut notifications = AutoNotifications::default();

    if user_cfg.auto_notify_on_low_mood && checkin.mood < user_cfg.auto_notify_threshold {
        if let Ok(list) = state
            .matrix
            .send_low_mood_notification(&user_cfg, &global_cfg, &checkin)
            .await
        {
            if !list.is_empty() {
                notifications.mood_threshold_triggered = true;
                notifications.notified_contacts.extend(list);
            }
        }
    }

    if form.safety_answer == "panic" {
        if let Ok(list) = state
            .matrix
            .send_panic_notification(&user_cfg, &global_cfg, Some(&checkin))
            .await
        {
            if !list.is_empty() {
                notifications.panic_triggered = true;
                notifications.notified_contacts.extend(list);
            }
        }
    }

    checkin.auto_notifications = notifications;

    state.storage.save_checkin(&user.uuid, &checkin).await?;

    if let Err(err) = state.git.commit_ai_changes(&format!(
        "feat: neues Mood-Checkin für {} 🌸",
        user.username
    )) {
        warn!("Git Commit für Check-in fehlgeschlagen: {err}");
    }

    Ok(Redirect::to(&format!("/me/checkins/{}", checkin.id)))
}

fn build_drug_entries(form: &CheckinForm) -> Vec<DrugEntry> {
    let mut entries = Vec::new();
    for idx in 0..form.drugs_substance.len() {
        let substance = form.drugs_substance[idx].trim();
        if substance.is_empty() {
            continue;
        }
        let dose = form
            .drugs_dose
            .get(idx)
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| "unbekannt".into());
        let route = form
            .drugs_route
            .get(idx)
            .map(|r| r.trim().to_string())
            .filter(|r| !r.is_empty());
        let start_time = form
            .drugs_start_time
            .get(idx)
            .and_then(|raw| parse_datetime(raw.trim()));
        let notes = form
            .drugs_notes
            .get(idx)
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty());
        entries.push(DrugEntry {
            substance: substance.to_string(),
            dose,
            route,
            start_time,
            notes,
        });
    }
    entries
}

fn parse_datetime(value: &str) -> Option<DateTime<Utc>> {
    if value.is_empty() {
        return None;
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.with_timezone(&Utc));
    }
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M")
        .ok()
        .and_then(|naive| Local.from_local_datetime(&naive).single())
        .map(|dt| dt.with_timezone(&Utc))
}

#[derive(Template)]
#[template(path = "user/checkin_detail.html")]
struct CheckinDetailTemplate {
    checkin: CheckinDetailData,
}

#[derive(Clone, Default)]
struct CheckinDetailData {
    timestamp: String,
    mood: i32,
    high_level: i32,
    status_tags: Vec<String>,
    has_status_tags: bool,
    safety_answer: String,
    has_notes: bool,
    notes: String,
    drugs: Vec<DrugView>,
    has_drugs: bool,
    notifications: Vec<String>,
    has_notifications: bool,
}

#[derive(Clone, Default)]
struct DrugView {
    substance: String,
    dose: String,
    route: String,
    has_notes: bool,
    notes: String,
}

async fn checkin_detail(
    State(state): State<AppState>,
    current: CurrentUser,
    Path(checkin_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let user = current.require_user()?;
    let checkin = state.storage.load_checkin(&user.uuid, &checkin_id).await?;
    let status_tags = checkin.status_tags.clone();
    let notes_text = checkin
        .notes_text()
        .map(|s| s.to_string())
        .unwrap_or_default();
    let drugs: Vec<DrugView> = checkin
        .drugs
        .iter()
        .map(|drug| {
            let note = drug.notes_text().map(|s| s.to_string()).unwrap_or_default();
            DrugView {
                substance: drug.substance.clone(),
                dose: drug.dose.clone(),
                route: drug.route_text().to_string(),
                has_notes: !note.is_empty(),
                notes: note,
            }
        })
        .collect();
    let notifications = checkin.auto_notifications.notified_contacts.clone();
    let data = CheckinDetailData {
        timestamp: format_timestamp(checkin.timestamp),
        mood: checkin.mood,
        high_level: checkin.high_level,
        status_tags: status_tags.clone(),
        has_status_tags: !status_tags.is_empty(),
        safety_answer: checkin.safety_answer_text().to_string(),
        has_notes: !notes_text.is_empty(),
        notes: notes_text,
        has_drugs: !drugs.is_empty(),
        drugs,
        has_notifications: !notifications.is_empty(),
        notifications,
    };
    Ok(AskamaTemplateResponse::into_response(
        CheckinDetailTemplate { checkin: data },
    ))
}

#[derive(Template)]
#[template(path = "user/trips_list.html")]
struct TripsListTemplate {
    trips: Vec<TripSummary>,
}

struct TripSummary {
    title: String,
    main_substance: String,
    mood_span: String,
    checkin_count: usize,
}

async fn trips_list(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let user = current.require_user()?;
    let checkins = state.storage.list_checkins(&user.uuid).await?;
    let trips = checkins
        .into_iter()
        .filter(|c| !c.drugs.is_empty())
        .map(|c| TripSummary {
            title: c
                .timestamp
                .with_timezone(&Local)
                .format("%d.%m.%Y")
                .to_string(),
            main_substance: c
                .drugs
                .first()
                .map(|d| d.substance.clone())
                .unwrap_or_else(|| "Unbekannt".into()),
            mood_span: mood_label(c.mood),
            checkin_count: 1,
        })
        .collect();
    Ok(AskamaTemplateResponse::into_response(TripsListTemplate {
        trips,
    }))
}

#[derive(Template)]
#[template(path = "user/panic.html")]
struct PanicTemplate {
    display_name: String,
    alert_sent: bool,
}

#[derive(Deserialize)]
struct PanicQuery {
    status: Option<String>,
}

async fn panic_page(
    State(state): State<AppState>,
    current: CurrentUser,
    Query(query): Query<PanicQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user = current.require_user()?;
    let cfg = state
        .storage
        .load_user_config(&user.uuid)
        .await
        .unwrap_or_else(|_| UserConfig::for_new_user(&user.username));
    Ok(AskamaTemplateResponse::into_response(PanicTemplate {
        display_name: cfg.display_name,
        alert_sent: query.status.as_deref() == Some("sent"),
    }))
}

async fn panic_trigger(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<Redirect, AppError> {
    let user = current.require_user()?;
    let cfg = state
        .storage
        .load_user_config(&user.uuid)
        .await
        .unwrap_or_else(|_| UserConfig::for_new_user(&user.username));
    let global_cfg = state.storage.load_global_config().await?;
    let last_checkin = state.storage.latest_checkin(&user.uuid).await?;

    let contacts = state
        .matrix
        .send_panic_notification(&cfg, &global_cfg, last_checkin.as_ref())
        .await
        .unwrap_or_default();

    let event = PanicEvent {
        id: Uuid::new_v4().to_string(),
        user_uuid: user.uuid.clone(),
        timestamp: Utc::now(),
        mood_at_panic: last_checkin.as_ref().map(|c| c.mood),
        high_level_at_panic: last_checkin.as_ref().map(|c| c.high_level),
        notified_contacts: contacts.clone(),
    };
    state.storage.save_panic_event(&event).await?;

    if let Err(err) = state
        .git
        .commit_ai_changes(&format!("feat: Panic-Event für {} 😰", user.username))
    {
        warn!("Git Commit für Panic Event fehlgeschlagen: {err}");
    }

    Ok(Redirect::to("/me/panic?status=sent"))
}

#[derive(Template)]
#[template(path = "user/settings.html")]
struct SettingsTemplate {
    config: UserConfig,
    status_saved: bool,
    matrix_ok: bool,
    matrix_error: bool,
    matrix_device_id_value: String,
    primary_contact_value: String,
    emergency_contacts_text: String,
}

#[derive(Deserialize)]
struct SettingsQuery {
    status: Option<String>,
    matrix: Option<String>,
}

async fn settings_form(
    State(state): State<AppState>,
    current: CurrentUser,
    Query(query): Query<SettingsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user = current.require_user()?;
    let config = state
        .storage
        .load_user_config(&user.uuid)
        .await
        .unwrap_or_else(|_| UserConfig::for_new_user(&user.username));
    let matrix_device_id_value = config.matrix_device_id.clone().unwrap_or_default();
    let primary_contact_value = config.primary_contact.clone().unwrap_or_default();
    let emergency_contacts_text = config.emergency_contacts.join("\n");

    Ok(AskamaTemplateResponse::into_response(SettingsTemplate {
        config,
        status_saved: query.status.as_deref() == Some("gespeichert"),
        matrix_ok: query.matrix.as_deref() == Some("ok"),
        matrix_error: query.matrix.as_deref() == Some("error"),
        matrix_device_id_value,
        primary_contact_value,
        emergency_contacts_text,
    }))
}

#[derive(Deserialize)]
struct SettingsForm {
    display_name: String,
    homeserver_url: String,
    matrix_user_id: String,
    matrix_access_token: String,
    matrix_device_id: Option<String>,
    primary_contact: Option<String>,
    emergency_contacts: Option<String>,
    auto_notify_on_low_mood: Option<String>,
    auto_notify_threshold: i32,
}

async fn settings_submit(
    State(state): State<AppState>,
    current: CurrentUser,
    Form(form): Form<SettingsForm>,
) -> Result<Redirect, AppError> {
    let user = current.require_user()?;
    let mut config = state
        .storage
        .load_user_config(&user.uuid)
        .await
        .unwrap_or_else(|_| UserConfig::for_new_user(&user.username));

    config.display_name = form.display_name.trim().to_string();
    config.homeserver_url = form.homeserver_url.trim().to_string();
    config.matrix_user_id = form.matrix_user_id.trim().to_string();
    config.matrix_access_token = form.matrix_access_token.trim().to_string();
    config.matrix_device_id = form
        .matrix_device_id
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    config.primary_contact = form
        .primary_contact
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    config.emergency_contacts = form
        .emergency_contacts
        .unwrap_or_default()
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();
    config.auto_notify_on_low_mood = form.auto_notify_on_low_mood.is_some();
    config.auto_notify_threshold = form.auto_notify_threshold;

    state.storage.save_user_config(&user.uuid, &config).await?;

    if let Err(err) = state.git.commit_ai_changes(&format!(
        "chore: Matrix-Konfiguration von {} aktualisiert 💕",
        user.username
    )) {
        warn!("Git Commit für Settings fehlgeschlagen: {err}");
    }

    Ok(Redirect::to("/me/settings?status=gespeichert"))
}

async fn settings_matrix_test(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<Redirect, AppError> {
    let user = current.require_user()?;
    let config = state
        .storage
        .load_user_config(&user.uuid)
        .await
        .unwrap_or_else(|_| UserConfig::for_new_user(&user.username));
    match state.matrix.send_test_message(&config).await {
        Ok(_) => Ok(Redirect::to("/me/settings?matrix=ok")),
        Err(err) => {
            warn!("Matrix Test fehlgeschlagen: {err}");
            Ok(Redirect::to("/me/settings?matrix=error"))
        }
    }
}

fn status_options() -> Vec<StatusOption> {
    vec![
        StatusOption {
            value: "calm",
            label: "stabil 😊",
            description: "ruhig und geerdet",
        },
        StatusOption {
            value: "emotional",
            label: "emotional 🥺",
            description: "Gefühle sind sehr präsent",
        },
        StatusOption {
            value: "overwhelmed",
            label: "sozial überfordert 😵‍💫",
            description: "brauche Ruhe",
        },
        StatusOption {
            value: "restless",
            label: "körperlich unruhig 🫀",
            description: "viel Energie im Körper",
        },
    ]
}

fn route_options() -> Vec<&'static str> {
    vec![
        "oral",
        "nasal",
        "sublingual",
        "inhalation",
        "intravenös",
        "rektal",
        "transdermal",
    ]
}

fn format_timestamp(ts: DateTime<Utc>) -> String {
    ts.with_timezone(&Local)
        .format("%d.%m.%Y %H:%M")
        .to_string()
}

fn mood_label(value: i32) -> String {
    match value {
        -5 => "Katastrophe 😭".into(),
        -4 => "Sehr schwer 😢".into(),
        -3 => "Ziemlich mies 😢".into(),
        -2 => "Traurig 😔".into(),
        -1 => "Down 😔".into(),
        0 => "Neutral 😶".into(),
        1 => "Vorsichtig ok 🙂".into(),
        2 => "Ganz gut 🙂".into(),
        3 => "Leicht euphorisch ✨".into(),
        4 => "Strahlend 💖".into(),
        _ => "Übertrieben gut ✨".into(),
    }
}

fn danger_message(mood: i32, high: i32) -> Option<String> {
    if mood < 0 && high >= 7 {
        Some("Achtung: du bist gerade ziemlich down und stark berauscht 😰 – vielleicht wäre es gut, mit jemandem zu reden 💕".into())
    } else if mood >= 0 && high <= 3 {
        Some("Du scheinst gerade recht stabil zu sein 💖".into())
    } else {
        None
    }
}

fn average_mood_text(checkins: &[Checkin]) -> (bool, String) {
    let since = Utc::now() - Duration::days(7);
    let values: Vec<i32> = checkins
        .iter()
        .filter(|c| c.timestamp >= since)
        .map(|c| c.mood)
        .collect();
    if values.is_empty() {
        (false, String::new())
    } else {
        let avg = values.iter().sum::<i32>() as f32 / values.len() as f32;
        (true, format!("Ø letzte 7 Tage: {:.1}", avg))
    }
}
