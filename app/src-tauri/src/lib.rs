//! Tauri shell entry point for the kinketsu app (macOS desktop + Android).
//!
//! Holds the SQLite pool in app state and exposes the v0.1 commands the
//! SvelteKit frontend calls via `@tauri-apps/api/core::invoke`.

use kinketsu_core::currency::ExchangeRate;
use kinketsu_core::db;
use kinketsu_core::llm::{LlmClient, LlmConfig};
use kinketsu_core::models::{
    Category, DetectionEvent, DetectionSource, DetectionStatus, NewCategory, NewPaymentMethod,
    NewSubscription, PaymentMethod, Subscription,
};
use kinketsu_core::oauth::{self, OAuthCredentials, Tokens};
use kinketsu_core::parsers::{self, ParsedSubscriptionHint, extract_from_text};
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_notification::NotificationExt;
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
async fn update_subscription(
    state: State<'_, AppState>,
    sub: Subscription,
) -> Result<(), String> {
    let mut updated = sub;
    updated.updated_at = chrono::Utc::now();
    db::subscriptions::update(&state.pool, &updated)
        .await
        .map_err(|e| e.to_string())
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
async fn update_payment_method(
    state: State<'_, AppState>,
    pm: PaymentMethod,
) -> Result<(), String> {
    let mut updated = pm;
    updated.updated_at = chrono::Utc::now();
    db::payment_methods::update(&state.pool, &updated)
        .await
        .map_err(|e| e.to_string())
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
async fn update_category(state: State<'_, AppState>, cat: Category) -> Result<(), String> {
    let mut updated = cat;
    updated.updated_at = chrono::Utc::now();
    db::categories::update(&state.pool, &updated)
        .await
        .map_err(|e| e.to_string())
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

// ---- Gmail OAuth + scan ----

#[tauri::command]
async fn save_gmail_oauth_credentials(
    state: State<'_, AppState>,
    creds: OAuthCredentials,
) -> Result<(), String> {
    db::settings::set(&state.pool, db::settings::keys::GMAIL_OAUTH_CREDS, &creds)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_gmail_oauth_credentials(
    state: State<'_, AppState>,
) -> Result<Option<OAuthCredentials>, String> {
    db::settings::get::<OAuthCredentials>(&state.pool, db::settings::keys::GMAIL_OAUTH_CREDS)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn has_gmail_tokens(state: State<'_, AppState>) -> Result<bool, String> {
    let tokens: Option<Tokens> = db::settings::get(&state.pool, db::settings::keys::GMAIL_TOKENS)
        .await
        .map_err(|e| e.to_string())?;
    Ok(tokens.is_some())
}

#[tauri::command]
async fn disconnect_gmail(state: State<'_, AppState>) -> Result<(), String> {
    db::settings::delete(&state.pool, db::settings::keys::GMAIL_TOKENS)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_gmail_oauth(state: State<'_, AppState>) -> Result<(), String> {
    let creds: OAuthCredentials =
        db::settings::get(&state.pool, db::settings::keys::GMAIL_OAUTH_CREDS)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Gmail OAuth credentials not configured".to_string())?;

    let (listener, port) = oauth::bind_oauth_listener()
        .await
        .map_err(|e| e.to_string())?;
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");
    let auth_url =
        oauth::build_auth_url(&creds.client_id, &redirect_uri, &[oauth::GMAIL_READONLY_SCOPE]);

    webbrowser::open(&auth_url).map_err(|e| format!("failed to open browser: {e}"))?;

    let code = tokio::time::timeout(
        std::time::Duration::from_secs(300),
        oauth::wait_for_oauth_code(listener),
    )
    .await
    .map_err(|_| "OAuth flow timed out after 5 minutes".to_string())?
    .map_err(|e| e.to_string())?;

    let tokens = oauth::exchange_code_for_tokens(&creds, &code, &redirect_uri)
        .await
        .map_err(|e| e.to_string())?;

    db::settings::set(&state.pool, db::settings::keys::GMAIL_TOKENS, &tokens)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Debug, serde::Deserialize)]
pub struct YearMonthDto {
    pub year: i32,
    pub month: u32,
}

#[tauri::command]
async fn run_gmail_scan(
    state: State<'_, AppState>,
    range: Vec<YearMonthDto>,
) -> Result<usize, String> {
    let llm_cfg: LlmConfig = db::settings::get(&state.pool, db::settings::keys::LLM_CONFIG)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no LLM provider configured".to_string())?;

    let creds: OAuthCredentials =
        db::settings::get(&state.pool, db::settings::keys::GMAIL_OAUTH_CREDS)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Gmail OAuth credentials not configured".to_string())?;

    let mut tokens: Tokens = db::settings::get(&state.pool, db::settings::keys::GMAIL_TOKENS)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Gmail not connected — click Connect Gmail first".to_string())?;

    let access_token = oauth::ensure_access_token(&creds, &mut tokens)
        .await
        .map_err(|e| e.to_string())?;

    db::settings::set(&state.pool, db::settings::keys::GMAIL_TOKENS, &tokens)
        .await
        .map_err(|e| e.to_string())?;

    let llm = LlmClient::from_config(llm_cfg);

    let months: Vec<parsers::gmail::YearMonth> = range
        .into_iter()
        .map(|d| parsers::gmail::YearMonth {
            year: d.year,
            month: d.month,
        })
        .collect();

    let query = parsers::gmail::build_query_for_range(&months);
    let msg_ids = parsers::gmail::list_message_ids(&access_token, &query)
        .await
        .map_err(|e| e.to_string())?;

    let mut created = 0usize;
    for id in &msg_ids {
        if db::detection_events::find_by_source_ref(&state.pool, DetectionSource::Gmail, id)
            .await
            .map_err(|e| e.to_string())?
            .is_some()
        {
            continue;
        }

        let msg = match parsers::gmail::fetch_message(&access_token, id).await {
            Ok(m) => m,
            Err(_) => continue,
        };
        let body = match parsers::gmail::extract_text_body(&msg) {
            Some(b) => b,
            None => continue,
        };

        let hint = match extract_from_text(&llm, body).await {
            Ok(h) => h,
            Err(_) => continue,
        };

        let msg_ref = parsers::gmail::message_ref_from(&msg, id);
        let summary = msg_ref
            .subject
            .clone()
            .unwrap_or_else(|| "(no subject)".to_string());
        let payload = serde_json::to_value(&hint).map_err(|e| e.to_string())?;

        let ev = DetectionEvent {
            id: Uuid::now_v7(),
            source: DetectionSource::Gmail,
            source_ref: Some(id.clone()),
            raw_summary: Some(summary),
            parsed_payload: payload,
            confidence: 0.0,
            status: DetectionStatus::Pending,
            matched_subscription_id: None,
            reviewed_at: None,
            created_at: chrono::Utc::now(),
        };

        db::detection_events::insert(&state.pool, &ev)
            .await
            .map_err(|e| e.to_string())?;

        created += 1;
    }

    Ok(created)
}

// ---- PayPal OAuth ----

#[tauri::command]
async fn save_paypal_oauth_credentials(
    state: State<'_, AppState>,
    creds: OAuthCredentials,
) -> Result<(), String> {
    db::settings::set(&state.pool, db::settings::keys::PAYPAL_OAUTH_CREDS, &creds)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_paypal_oauth_credentials(
    state: State<'_, AppState>,
) -> Result<Option<OAuthCredentials>, String> {
    db::settings::get::<OAuthCredentials>(&state.pool, db::settings::keys::PAYPAL_OAUTH_CREDS)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn has_paypal_tokens(state: State<'_, AppState>) -> Result<bool, String> {
    let tokens: Option<Tokens> = db::settings::get(&state.pool, db::settings::keys::PAYPAL_TOKENS)
        .await
        .map_err(|e| e.to_string())?;
    Ok(tokens.is_some())
}

#[tauri::command]
async fn disconnect_paypal(state: State<'_, AppState>) -> Result<(), String> {
    db::settings::delete(&state.pool, db::settings::keys::PAYPAL_TOKENS)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_paypal_oauth(state: State<'_, AppState>) -> Result<(), String> {
    let creds: OAuthCredentials =
        db::settings::get(&state.pool, db::settings::keys::PAYPAL_OAUTH_CREDS)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "PayPal OAuth credentials not configured".to_string())?;

    let (listener, port) = oauth::bind_oauth_listener()
        .await
        .map_err(|e| e.to_string())?;
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");
    let auth_url = oauth::build_paypal_auth_url(
        &creds.client_id,
        &redirect_uri,
        &[oauth::PAYPAL_OPENID_SCOPE, oauth::PAYPAL_TRANSACTIONS_SCOPE],
    );

    webbrowser::open(&auth_url).map_err(|e| format!("failed to open browser: {e}"))?;

    let code = tokio::time::timeout(
        std::time::Duration::from_secs(300),
        oauth::wait_for_oauth_code(listener),
    )
    .await
    .map_err(|_| "OAuth flow timed out after 5 minutes".to_string())?
    .map_err(|e| e.to_string())?;

    let tokens = oauth::exchange_paypal_code(&creds, &code, &redirect_uri)
        .await
        .map_err(|e| e.to_string())?;

    db::settings::set(&state.pool, db::settings::keys::PAYPAL_TOKENS, &tokens)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn run_paypal_scan(state: State<'_, AppState>) -> Result<usize, String> {
    // Token refresh is exercised here so the connection is verified as live,
    // even though the Transaction Search API integration isn't wired yet.
    let creds: OAuthCredentials =
        db::settings::get(&state.pool, db::settings::keys::PAYPAL_OAUTH_CREDS)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "PayPal OAuth credentials not configured".to_string())?;
    let mut tokens: Tokens = db::settings::get(&state.pool, db::settings::keys::PAYPAL_TOKENS)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "PayPal not connected".to_string())?;
    let _access = oauth::ensure_paypal_access_token(&creds, &mut tokens)
        .await
        .map_err(|e| e.to_string())?;
    db::settings::set(&state.pool, db::settings::keys::PAYPAL_TOKENS, &tokens)
        .await
        .map_err(|e| e.to_string())?;

    Err("PayPal transaction scan is not yet wired — the Transaction Search API requires a date-range loop and per-transaction LLM extraction, both of which are scoped to a follow-up round. Token refresh succeeded, so the connection is healthy.".to_string())
}

// ---- Renewal notifications ----

async fn notify_renewals(handle: &AppHandle, pool: &SqlitePool) -> Result<usize, String> {
    let due = db::subscriptions::list_due_within(pool, 7)
        .await
        .map_err(|e| e.to_string())?;
    let mut sent = 0usize;
    for sub in &due {
        if let Some(date) = sub.next_billing_date {
            let title = format!("{} renews soon", sub.name);
            let body = format!("Next charge: {date}");
            if handle
                .notification()
                .builder()
                .title(title)
                .body(body)
                .show()
                .is_ok()
            {
                sent += 1;
            }
        }
    }
    Ok(sent)
}

#[tauri::command]
async fn check_renewals_now(state: State<'_, AppState>, app: AppHandle) -> Result<usize, String> {
    notify_renewals(&app, &state.pool).await
}

// ---- iCalendar export ----

#[tauri::command]
async fn export_subscriptions_ics(state: State<'_, AppState>) -> Result<String, String> {
    let subs = db::subscriptions::list(&state.pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(kinketsu_core::ics::export_subscriptions(&subs))
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
async fn confirm_detection_event_with_overrides(
    state: State<'_, AppState>,
    id: Uuid,
    sub: NewSubscription,
) -> Result<Subscription, String> {
    let _ev = db::detection_events::get(&state.pool, id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "detection event not found".to_string())?;

    let new_sub = sub.into_subscription();
    db::subscriptions::insert(&state.pool, &new_sub)
        .await
        .map_err(|e| e.to_string())?;

    db::detection_events::update_status(
        &state.pool,
        id,
        DetectionStatus::Confirmed,
        Some(new_sub.id),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(new_sub)
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
        .plugin(tauri_plugin_notification::init())
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

            // Daily renewal check — fires 15s after launch then every 24h.
            let scheduler_pool = pool.clone();
            let scheduler_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                loop {
                    if let Err(e) = notify_renewals(&scheduler_handle, &scheduler_pool).await {
                        log::warn!("renewal notification check failed: {e}");
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(24 * 60 * 60)).await;
                }
            });

            app.manage(AppState { pool });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_subscriptions,
            create_subscription,
            update_subscription,
            delete_subscription,
            list_payment_methods,
            create_payment_method,
            update_payment_method,
            delete_payment_method,
            list_categories,
            create_category,
            update_category,
            delete_category,
            get_llm_config,
            set_llm_config,
            extract_subscription_from_text,
            list_detection_events,
            confirm_detection_event,
            confirm_detection_event_with_overrides,
            reject_detection_event,
            refresh_exchange_rates,
            list_exchange_rates,
            export_subscriptions_ics,
            save_gmail_oauth_credentials,
            get_gmail_oauth_credentials,
            has_gmail_tokens,
            disconnect_gmail,
            start_gmail_oauth,
            run_gmail_scan,
            save_paypal_oauth_credentials,
            get_paypal_oauth_credentials,
            has_paypal_tokens,
            disconnect_paypal,
            start_paypal_oauth,
            run_paypal_scan,
            check_renewals_now,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
