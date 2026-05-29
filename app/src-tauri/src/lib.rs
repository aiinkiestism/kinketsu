//! Tauri shell entry point for the kinketsu app (macOS desktop + Android).
//!
//! Holds the SQLite pool in app state and exposes the v0.1 commands the
//! SvelteKit frontend calls via `@tauri-apps/api/core::invoke`.

use kinketsu_core::currency::ExchangeRate;
use kinketsu_core::db;
use kinketsu_core::llm::{LlmClient, LlmConfig};
use kinketsu_core::models::{
    Category, DetectionEvent, DetectionStatus, NewCategory, NewPaymentMethod, NewSubscription,
    PaymentMethod, Subscription,
};
use kinketsu_core::parsers::{ParsedSubscriptionHint, extract_from_text};
use sqlx::SqlitePool;
use tauri::{Manager, State};
use uuid::Uuid;

pub struct AppState {
    pub pool: SqlitePool,
}

// ---- subscriptions ----

#[tauri::command]
async fn list_subscriptions(state: State<'_, AppState>) -> Result<Vec<Subscription>, String> {
    db::subscriptions::list(&state.pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_subscription(
    state: State<'_, AppState>,
    input: NewSubscription,
) -> Result<Subscription, String> {
    let sub = input.into_subscription();
    db::subscriptions::insert(&state.pool, &sub)
        .await
        .map_err(|e| e.to_string())?;
    Ok(sub)
}

#[tauri::command]
async fn delete_subscription(state: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    db::subscriptions::delete(&state.pool, id)
        .await
        .map_err(|e| e.to_string())
}

// ---- payment methods ----

#[tauri::command]
async fn list_payment_methods(state: State<'_, AppState>) -> Result<Vec<PaymentMethod>, String> {
    db::payment_methods::list(&state.pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_payment_method(
    state: State<'_, AppState>,
    input: NewPaymentMethod,
) -> Result<PaymentMethod, String> {
    let pm = input.into_payment_method();
    db::payment_methods::insert(&state.pool, &pm)
        .await
        .map_err(|e| e.to_string())?;
    Ok(pm)
}

#[tauri::command]
async fn delete_payment_method(state: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    db::payment_methods::delete(&state.pool, id)
        .await
        .map_err(|e| e.to_string())
}

// ---- categories ----

#[tauri::command]
async fn list_categories(state: State<'_, AppState>) -> Result<Vec<Category>, String> {
    db::categories::list(&state.pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_category(
    state: State<'_, AppState>,
    input: NewCategory,
) -> Result<Category, String> {
    let cat = input.into_category();
    db::categories::insert(&state.pool, &cat)
        .await
        .map_err(|e| e.to_string())?;
    Ok(cat)
}

#[tauri::command]
async fn delete_category(state: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    db::categories::delete(&state.pool, id)
        .await
        .map_err(|e| e.to_string())
}

// ---- LLM provider configuration ----

#[tauri::command]
async fn get_llm_config(state: State<'_, AppState>) -> Result<Option<LlmConfig>, String> {
    db::settings::get::<LlmConfig>(&state.pool, db::settings::keys::LLM_CONFIG)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_llm_config(state: State<'_, AppState>, config: LlmConfig) -> Result<(), String> {
    db::settings::set(&state.pool, db::settings::keys::LLM_CONFIG, &config)
        .await
        .map_err(|e| e.to_string())
}

// ---- Exchange rates ----

#[tauri::command]
async fn refresh_exchange_rates(
    state: State<'_, AppState>,
    base: String,
) -> Result<usize, String> {
    let rates = kinketsu_core::currency::refresh_rates(&base)
        .await
        .map_err(|e| e.to_string())?;
    let count = rates.len();
    for r in &rates {
        db::exchange_rates::upsert(&state.pool, r)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(count)
}

#[tauri::command]
async fn list_exchange_rates(
    state: State<'_, AppState>,
    base: String,
) -> Result<Vec<ExchangeRate>, String> {
    db::exchange_rates::list_latest_for_base(&state.pool, &base)
        .await
        .map_err(|e| e.to_string())
}

// ---- Detection events ----

#[tauri::command]
async fn list_detection_events(state: State<'_, AppState>) -> Result<Vec<DetectionEvent>, String> {
    db::detection_events::list(&state.pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn confirm_detection_event(
    state: State<'_, AppState>,
    id: Uuid,
) -> Result<Subscription, String> {
    let ev = db::detection_events::get(&state.pool, id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "detection event not found".to_string())?;

    let hint: ParsedSubscriptionHint = serde_json::from_value(ev.parsed_payload.clone())
        .map_err(|e| format!("failed to decode payload: {e}"))?;

    let name = hint.service_name.ok_or_else(|| "missing service_name".to_string())?;
    let amount = hint
        .amount_minor
        .ok_or_else(|| "missing amount_minor".to_string())?;
    let currency = hint
        .currency
        .ok_or_else(|| "missing currency".to_string())?;
    let cycle = hint
        .billing_cycle
        .ok_or_else(|| "missing billing_cycle".to_string())?;

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
        notes: hint.payment_method_hint.map(|h| format!("Payment hint: {h}")),
    };
    let sub = new_sub.into_subscription();

    db::subscriptions::insert(&state.pool, &sub)
        .await
        .map_err(|e| e.to_string())?;

    db::detection_events::update_status(
        &state.pool,
        ev.id,
        DetectionStatus::Confirmed,
        Some(sub.id),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(sub)
}

#[tauri::command]
async fn reject_detection_event(state: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    db::detection_events::update_status(&state.pool, id, DetectionStatus::Rejected, None)
        .await
        .map_err(|e| e.to_string())
}

// ---- Extraction pipeline ----

#[tauri::command]
async fn extract_subscription_from_text(
    state: State<'_, AppState>,
    text: String,
) -> Result<ParsedSubscriptionHint, String> {
    let cfg = db::settings::get::<LlmConfig>(&state.pool, db::settings::keys::LLM_CONFIG)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no LLM provider configured".to_string())?;

    let client = LlmClient::from_config(cfg);
    extract_from_text(&client, text)
        .await
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let db_path = app_data_dir.join("kinketsu.db");

            let pool = tauri::async_runtime::block_on(async {
                let pool = db::connect_file(&db_path).await?;
                db::migrate(&pool).await?;
                Ok::<SqlitePool, kinketsu_core::Error>(pool)
            })?;

            app.manage(AppState { pool });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_subscriptions,
            create_subscription,
            delete_subscription,
            list_payment_methods,
            create_payment_method,
            delete_payment_method,
            list_categories,
            create_category,
            delete_category,
            get_llm_config,
            set_llm_config,
            extract_subscription_from_text,
            list_detection_events,
            confirm_detection_event,
            reject_detection_event,
            refresh_exchange_rates,
            list_exchange_rates,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
