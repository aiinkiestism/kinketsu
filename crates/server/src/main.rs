//! kinketsu HTTP server — single-user self-host target.
//!
//! All mutating routes (and the read routes that touch private state) sit
//! behind a Bearer token. The token is the value of `KINKETSU_SECRET`; pick a
//! long random string and pass it as `Authorization: Bearer <secret>`.

use axum::Json;
use axum::Router;
use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use serde_json::json;
use sqlx::SqlitePool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use kinketsu_core::currency::ExchangeRate;
use kinketsu_core::db;
use kinketsu_core::ics;
use kinketsu_core::llm::{LlmClient, LlmConfig};
use kinketsu_core::models::{
    Category, DetectionEvent, DetectionStatus, NewCategory, NewPaymentMethod, NewSubscription,
    PaymentMethod, Subscription,
};
use kinketsu_core::parsers::{
    self, ParsedSubscriptionHint, extract_from_text, extract_many_from_text,
};

#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
    secret: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let db_path = std::env::var("KINKETSU_DB").unwrap_or_else(|_| "./kinketsu.db".to_string());
    let secret = std::env::var("KINKETSU_SECRET")
        .map_err(|_| anyhow::anyhow!("KINKETSU_SECRET env var required"))?;
    if secret.len() < 16 {
        return Err(anyhow::anyhow!(
            "KINKETSU_SECRET is too short — use at least 16 characters of entropy"
        ));
    }

    let pool = db::connect(&format!("sqlite://{db_path}?mode=rwc")).await?;
    db::migrate(&pool).await?;

    let state = AppState { pool, secret };

    let protected = Router::new()
        .route(
            "/subscriptions",
            get(list_subscriptions).post(create_subscription),
        )
        .route(
            "/subscriptions/:id",
            put(update_subscription).delete(delete_subscription),
        )
        .route(
            "/payment-methods",
            get(list_payment_methods).post(create_payment_method),
        )
        .route(
            "/payment-methods/:id",
            put(update_payment_method).delete(delete_payment_method),
        )
        .route("/categories", get(list_categories).post(create_category))
        .route(
            "/categories/:id",
            put(update_category).delete(delete_category),
        )
        .route("/detection-events", get(list_detection_events))
        .route(
            "/detection-events/:id/confirm",
            post(confirm_detection_event),
        )
        .route("/detection-events/:id/reject", post(reject_detection_event))
        .route("/settings/llm", get(get_llm_config).put(set_llm_config))
        .route(
            "/exchange-rates",
            get(list_exchange_rates).post(refresh_exchange_rates),
        )
        .route("/scan/text", post(scan_text))
        .route("/scan/csv", post(scan_csv))
        .route("/ics", get(export_ics))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_secret,
        ));

    let app = Router::new()
        .route("/health", get(health))
        .merge(protected)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = std::env::var("KINKETSU_BIND").unwrap_or_else(|_| "0.0.0.0:3000".into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("kinketsu-server listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn require_secret(State(s): State<AppState>, req: Request, next: Next) -> Response {
    let auth = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());
    let expected = format!("Bearer {}", s.secret);
    if auth == Some(expected.as_str()) {
        next.run(req).await
    } else {
        (StatusCode::UNAUTHORIZED, "missing or invalid bearer token").into_response()
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "service": "kinketsu-server" }))
}

type ApiError = (StatusCode, String);

fn err(e: impl std::fmt::Display) -> ApiError {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

fn bad_request(msg: impl Into<String>) -> ApiError {
    (StatusCode::BAD_REQUEST, msg.into())
}

fn not_found(msg: impl Into<String>) -> ApiError {
    (StatusCode::NOT_FOUND, msg.into())
}

// ---- subscriptions ----

async fn list_subscriptions(
    State(s): State<AppState>,
) -> Result<Json<Vec<Subscription>>, ApiError> {
    Ok(Json(db::subscriptions::list(&s.pool).await.map_err(err)?))
}

async fn create_subscription(
    State(s): State<AppState>,
    Json(input): Json<NewSubscription>,
) -> Result<Json<Subscription>, ApiError> {
    let sub = input.into_subscription();
    db::subscriptions::insert(&s.pool, &sub)
        .await
        .map_err(err)?;
    Ok(Json(sub))
}

async fn update_subscription(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut sub): Json<Subscription>,
) -> Result<StatusCode, ApiError> {
    if sub.id != id {
        return Err(bad_request("path id does not match body id"));
    }
    sub.updated_at = chrono::Utc::now();
    db::subscriptions::update(&s.pool, &sub)
        .await
        .map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_subscription(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    db::subscriptions::delete(&s.pool, id).await.map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- payment methods ----

async fn list_payment_methods(
    State(s): State<AppState>,
) -> Result<Json<Vec<PaymentMethod>>, ApiError> {
    Ok(Json(db::payment_methods::list(&s.pool).await.map_err(err)?))
}

async fn create_payment_method(
    State(s): State<AppState>,
    Json(input): Json<NewPaymentMethod>,
) -> Result<Json<PaymentMethod>, ApiError> {
    let pm = input.into_payment_method();
    db::payment_methods::insert(&s.pool, &pm)
        .await
        .map_err(err)?;
    Ok(Json(pm))
}

async fn update_payment_method(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut pm): Json<PaymentMethod>,
) -> Result<StatusCode, ApiError> {
    if pm.id != id {
        return Err(bad_request("path id does not match body id"));
    }
    pm.updated_at = chrono::Utc::now();
    db::payment_methods::update(&s.pool, &pm)
        .await
        .map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_payment_method(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    db::payment_methods::delete(&s.pool, id)
        .await
        .map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- categories ----

async fn list_categories(State(s): State<AppState>) -> Result<Json<Vec<Category>>, ApiError> {
    Ok(Json(db::categories::list(&s.pool).await.map_err(err)?))
}

async fn create_category(
    State(s): State<AppState>,
    Json(input): Json<NewCategory>,
) -> Result<Json<Category>, ApiError> {
    let cat = input.into_category();
    db::categories::insert(&s.pool, &cat).await.map_err(err)?;
    Ok(Json(cat))
}

async fn update_category(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut cat): Json<Category>,
) -> Result<StatusCode, ApiError> {
    if cat.id != id {
        return Err(bad_request("path id does not match body id"));
    }
    cat.updated_at = chrono::Utc::now();
    db::categories::update(&s.pool, &cat).await.map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_category(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    db::categories::delete(&s.pool, id).await.map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- detection events ----

async fn list_detection_events(
    State(s): State<AppState>,
) -> Result<Json<Vec<DetectionEvent>>, ApiError> {
    Ok(Json(
        db::detection_events::list(&s.pool).await.map_err(err)?,
    ))
}

async fn confirm_detection_event(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Subscription>, ApiError> {
    let ev = db::detection_events::get(&s.pool, id)
        .await
        .map_err(err)?
        .ok_or_else(|| not_found("detection event"))?;

    let hint: ParsedSubscriptionHint =
        serde_json::from_value(ev.parsed_payload.clone()).map_err(err)?;
    let name = hint
        .service_name
        .ok_or_else(|| bad_request("missing service_name"))?;
    let amount = hint
        .amount_minor
        .ok_or_else(|| bad_request("missing amount_minor"))?;
    let currency = hint
        .currency
        .ok_or_else(|| bad_request("missing currency"))?;
    let cycle = hint
        .billing_cycle
        .ok_or_else(|| bad_request("missing billing_cycle"))?;

    let new_sub = NewSubscription {
        name,
        service_icon: None,
        plan: None,
        amount_minor: amount,
        currency,
        billing_cycle: cycle,
        next_billing_date: None,
        started_at: hint.charged_at,
        payment_method_id: None,
        category_id: None,
        status: None,
        notes: hint
            .payment_method_hint
            .map(|h| format!("Payment hint: {h}")),
    };
    let sub = new_sub.into_subscription();
    db::subscriptions::insert(&s.pool, &sub)
        .await
        .map_err(err)?;
    db::detection_events::update_status(&s.pool, ev.id, DetectionStatus::Confirmed, Some(sub.id))
        .await
        .map_err(err)?;
    Ok(Json(sub))
}

async fn reject_detection_event(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    db::detection_events::update_status(&s.pool, id, DetectionStatus::Rejected, None)
        .await
        .map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- settings ----

async fn get_llm_config(State(s): State<AppState>) -> Result<Json<Option<LlmConfig>>, ApiError> {
    let cfg = db::settings::get::<LlmConfig>(&s.pool, db::settings::keys::LLM_CONFIG)
        .await
        .map_err(err)?;
    Ok(Json(cfg))
}

async fn set_llm_config(
    State(s): State<AppState>,
    Json(cfg): Json<LlmConfig>,
) -> Result<StatusCode, ApiError> {
    db::settings::set(&s.pool, db::settings::keys::LLM_CONFIG, &cfg)
        .await
        .map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- exchange rates ----

#[derive(serde::Deserialize)]
struct BaseQuery {
    base: Option<String>,
}

async fn list_exchange_rates(
    State(s): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<BaseQuery>,
) -> Result<Json<Vec<ExchangeRate>>, ApiError> {
    let base = q.base.as_deref().unwrap_or("JPY");
    Ok(Json(
        db::exchange_rates::list_latest_for_base(&s.pool, base)
            .await
            .map_err(err)?,
    ))
}

async fn refresh_exchange_rates(
    State(s): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<BaseQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let base = q.base.as_deref().unwrap_or("JPY");
    let rates = kinketsu_core::currency::refresh_rates(base)
        .await
        .map_err(err)?;
    for r in &rates {
        db::exchange_rates::upsert(&s.pool, r).await.map_err(err)?;
    }
    Ok(Json(json!({ "fetched": rates.len() })))
}

// ---- scan ----

#[derive(serde::Deserialize)]
struct ScanTextRequest {
    text: String,
}

async fn scan_text(
    State(s): State<AppState>,
    Json(req): Json<ScanTextRequest>,
) -> Result<Json<ParsedSubscriptionHint>, ApiError> {
    let cfg = db::settings::get::<LlmConfig>(&s.pool, db::settings::keys::LLM_CONFIG)
        .await
        .map_err(err)?
        .ok_or_else(|| bad_request("no LLM provider configured"))?;
    let client = LlmClient::from_config(cfg);
    match extract_from_text(&client, req.text).await.map_err(err)? {
        Some(hint) => Ok(Json(hint)),
        None => Err(bad_request(
            "LLM classified this text as not a recurring subscription",
        )),
    }
}

async fn scan_csv(
    State(s): State<AppState>,
    Json(req): Json<ScanTextRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let cfg = db::settings::get::<LlmConfig>(&s.pool, db::settings::keys::LLM_CONFIG)
        .await
        .map_err(err)?
        .ok_or_else(|| bad_request("no LLM provider configured"))?;
    let client = LlmClient::from_config(cfg);
    let hints = extract_many_from_text(&client, req.text)
        .await
        .map_err(err)?;
    let mut created = 0usize;
    for hint in hints {
        let payload = serde_json::to_value(&hint).map_err(err)?;
        let summary = hint
            .service_name
            .clone()
            .unwrap_or_else(|| "(unnamed)".to_string());
        let ev = DetectionEvent {
            id: Uuid::now_v7(),
            source: kinketsu_core::models::DetectionSource::CsvImport,
            source_ref: None,
            raw_summary: Some(summary),
            parsed_payload: payload,
            confidence: 0.0,
            status: DetectionStatus::Pending,
            matched_subscription_id: None,
            reviewed_at: None,
            created_at: chrono::Utc::now(),
        };
        db::detection_events::insert(&s.pool, &ev)
            .await
            .map_err(err)?;
        created += 1;
    }
    Ok(Json(json!({ "created": created })))
}

// ---- iCalendar export ----

async fn export_ics(State(s): State<AppState>) -> Result<Response, ApiError> {
    let subs = db::subscriptions::list(&s.pool).await.map_err(err)?;
    let body = ics::export_subscriptions(&subs);
    let resp = Response::builder()
        .header("content-type", "text/calendar; charset=utf-8")
        .header(
            "content-disposition",
            "attachment; filename=\"kinketsu.ics\"",
        )
        .body(axum::body::Body::from(body))
        .map_err(err)?;
    Ok(resp)
}

// Avoid clippy::unused on parsers re-export
#[allow(dead_code)]
fn _force_parsers_link() {
    let _ = parsers::extract_from_text;
}
