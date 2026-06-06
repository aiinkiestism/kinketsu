//! Tauri shell entry point for the kinketsu app (macOS desktop + Android).
//!
//! Holds the SQLite pool in app state and exposes the v0.1 commands the
//! SvelteKit frontend calls via `@tauri-apps/api/core::invoke`.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::oneshot;

use kinketsu_core::currency::ExchangeRate;
use kinketsu_core::db;
use kinketsu_core::llm::{ExtractionRequest, LlmClient, LlmConfig};
use kinketsu_core::models::{
    Category, DetectionEvent, DetectionSource, DetectionStatus, LearnedDecision, LearnedSender,
    NewCategory, NewPaymentMethod, NewSubscription, PaymentMethod, Subscription,
};
use kinketsu_core::oauth::{self, OAuthCredentials, Tokens};
use kinketsu_core::parsers::{
    self, ParsedSubscriptionHint, extract_from_text, extract_many_from_text,
};
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;
use uuid::Uuid;

pub struct AppState {
    pub pool: SqlitePool,
    /// Cancellation signal for an in-flight OAuth flow (Gmail or PayPal).
    /// Starting a new flow first cancels any previous one, so the UI's
    /// Connect button is always responsive even if the previous attempt
    /// silently failed (e.g. user closed the browser tab).
    pub oauth_cancel: StdMutex<Option<oneshot::Sender<()>>>,
    /// Cooperative cancellation flag for an in-flight scan loop. The scan
    /// checks this at every iteration so the UI's Cancel button can bail
    /// out without waiting for the next LLM round-trip to finish.
    pub scan_cancel: Arc<AtomicBool>,
    /// Survivors from the most recent scan preview, cached so a follow-up scan
    /// with identical parameters skips re-fetching every message body.
    pub last_preview: StdMutex<Option<PreviewCache>>,
}

fn arm_oauth_cancel(state: &AppState) -> oneshot::Receiver<()> {
    let (tx, rx) = oneshot::channel::<()>();
    let mut guard = state
        .oauth_cancel
        .lock()
        .expect("oauth_cancel mutex poisoned");
    if let Some(prev) = guard.take() {
        // Best-effort: receiver may already be dropped if the previous flow
        // finished; that's fine and surfaces as `Err(())` we ignore.
        let _ = prev.send(());
    }
    *guard = Some(tx);
    rx
}

// ---- subscriptions ----

#[tauri::command]
#[specta::specta]
async fn list_subscriptions(state: State<'_, AppState>) -> Result<Vec<Subscription>, String> {
    db::subscriptions::list(&state.pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
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
#[specta::specta]
async fn update_subscription(state: State<'_, AppState>, sub: Subscription) -> Result<(), String> {
    let mut updated = sub;
    updated.updated_at = chrono::Utc::now();
    db::subscriptions::update(&state.pool, &updated)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn delete_subscription(state: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    db::subscriptions::delete(&state.pool, id)
        .await
        .map_err(|e| e.to_string())
}

// ---- payment methods ----

#[tauri::command]
#[specta::specta]
async fn list_payment_methods(state: State<'_, AppState>) -> Result<Vec<PaymentMethod>, String> {
    db::payment_methods::list(&state.pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
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
#[specta::specta]
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
#[specta::specta]
async fn delete_payment_method(state: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    db::payment_methods::delete(&state.pool, id)
        .await
        .map_err(|e| e.to_string())
}

// ---- categories ----

#[tauri::command]
#[specta::specta]
async fn list_categories(state: State<'_, AppState>) -> Result<Vec<Category>, String> {
    db::categories::list(&state.pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
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
#[specta::specta]
async fn update_category(state: State<'_, AppState>, cat: Category) -> Result<(), String> {
    let mut updated = cat;
    updated.updated_at = chrono::Utc::now();
    db::categories::update(&state.pool, &updated)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn delete_category(state: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    db::categories::delete(&state.pool, id)
        .await
        .map_err(|e| e.to_string())
}

// ---- LLM provider configuration ----

#[tauri::command]
#[specta::specta]
async fn get_llm_config(state: State<'_, AppState>) -> Result<Option<LlmConfig>, String> {
    db::settings::get::<LlmConfig>(&state.pool, db::settings::keys::LLM_CONFIG)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn set_llm_config(state: State<'_, AppState>, config: LlmConfig) -> Result<(), String> {
    db::settings::set(&state.pool, db::settings::keys::LLM_CONFIG, &config)
        .await
        .map_err(|e| e.to_string())
}

// ---- Gmail OAuth + scan ----

#[tauri::command]
#[specta::specta]
async fn save_gmail_oauth_credentials(
    state: State<'_, AppState>,
    creds: OAuthCredentials,
) -> Result<(), String> {
    db::settings::set(&state.pool, db::settings::keys::GMAIL_OAUTH_CREDS, &creds)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn get_gmail_oauth_credentials(
    state: State<'_, AppState>,
) -> Result<Option<OAuthCredentials>, String> {
    db::settings::get::<OAuthCredentials>(&state.pool, db::settings::keys::GMAIL_OAUTH_CREDS)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn has_gmail_tokens(state: State<'_, AppState>) -> Result<bool, String> {
    let tokens: Option<Tokens> = db::settings::get(&state.pool, db::settings::keys::GMAIL_TOKENS)
        .await
        .map_err(|e| e.to_string())?;
    Ok(tokens.is_some())
}

#[tauri::command]
#[specta::specta]
async fn disconnect_gmail(state: State<'_, AppState>) -> Result<(), String> {
    db::settings::delete(&state.pool, db::settings::keys::GMAIL_TOKENS)
        .await
        .map_err(|e| e.to_string())
}

/// Abort any in-flight OAuth flow (Gmail or PayPal) so the UI can reset its
/// connecting state without waiting for the 2-minute timeout to fire.
#[tauri::command]
#[specta::specta]
async fn cancel_oauth(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state
        .oauth_cancel
        .lock()
        .map_err(|_| "oauth_cancel mutex poisoned".to_string())?;
    if let Some(tx) = guard.take() {
        let _ = tx.send(());
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn start_gmail_oauth(state: State<'_, AppState>) -> Result<(), String> {
    let creds: OAuthCredentials =
        db::settings::get(&state.pool, db::settings::keys::GMAIL_OAUTH_CREDS)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Gmail OAuth credentials not configured".to_string())?;

    let cancel_rx = arm_oauth_cancel(&state);

    let (listener, port) = oauth::bind_oauth_listener()
        .await
        .map_err(|e| e.to_string())?;
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");
    let auth_url = oauth::build_auth_url(
        &creds.client_id,
        &redirect_uri,
        &[oauth::GMAIL_READONLY_SCOPE],
    );

    webbrowser::open(&auth_url).map_err(|e| format!("failed to open browser: {e}"))?;

    let code = tokio::select! {
        biased;
        _ = cancel_rx => return Err("oauth cancelled".into()),
        res = oauth::wait_for_oauth_code(listener) => res.map_err(|e| e.to_string())?,
        () = tokio::time::sleep(std::time::Duration::from_secs(120)) => {
            return Err("OAuth flow timed out after 2 minutes — try Connect Gmail again.".into());
        }
    };

    let tokens = oauth::exchange_code_for_tokens(&creds, &code, &redirect_uri)
        .await
        .map_err(|e| e.to_string())?;

    db::settings::set(&state.pool, db::settings::keys::GMAIL_TOKENS, &tokens)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Debug, serde::Deserialize, specta::Type)]
pub struct YearMonthDto {
    pub year: i32,
    pub month: u32,
}

/// Per-scan tuning the UI sends alongside the month range. `None` caps fall
/// back to the pipeline defaults.
#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
pub struct ScanOptsDto {
    /// Max messages to fetch + screen (replaces the old hard 500 cap).
    pub max_fetch: Option<u32>,
    /// Max survivors sent to the LLM — the cost-bearing knob.
    pub max_llm: Option<u32>,
    /// Restrict the Gmail query to `category:purchases`.
    pub use_purchases: bool,
}

/// A preview's screened survivors, cached so a follow-up scan with the same
/// parameters skips re-fetching every message body.
pub struct PreviewCache {
    signature: String,
    screen: parsers::scan::ScreenResult,
}

fn build_scan_opts(deep: bool, dto: &ScanOptsDto) -> parsers::scan::ScanOptions {
    let mode = if deep {
        parsers::gmail::ScanMode::Deep
    } else {
        parsers::gmail::ScanMode::Fast
    };
    let max_fetch = dto
        .max_fetch
        .map_or(parsers::scan::DEFAULT_MAX_FETCH, |v| v as usize);
    let max_llm = dto
        .max_llm
        .map_or(parsers::scan::DEFAULT_MAX_LLM, |v| v as usize);
    parsers::scan::ScanOptions::new(mode, max_fetch, max_llm, dto.use_purchases)
}

/// Stable key identifying a scan's inputs, so a cached preview is only reused
/// for an identical follow-up scan.
fn scan_signature(range: &[YearMonthDto], opts: &parsers::scan::ScanOptions) -> String {
    let mut s = String::new();
    for ym in range {
        s.push_str(&format!("{}-{};", ym.year, ym.month));
    }
    s.push_str(&format!(
        "mode={:?};fetch={};llm={};pur={}",
        opts.mode, opts.max_fetch, opts.max_llm, opts.use_purchases
    ));
    s
}

fn to_months(range: Vec<YearMonthDto>) -> Vec<parsers::gmail::YearMonth> {
    range
        .into_iter()
        .map(|d| parsers::gmail::YearMonth {
            year: d.year,
            month: d.month,
        })
        .collect()
}

/// Refresh + persist the Gmail access token, returning a usable bearer token.
async fn ensure_gmail_token(state: &AppState) -> Result<String, String> {
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
    Ok(access_token)
}

async fn load_llm_config(state: &AppState) -> Result<LlmConfig, String> {
    db::settings::get(&state.pool, db::settings::keys::LLM_CONFIG)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no LLM provider configured".to_string())
}

fn llm_provider_model(cfg: &LlmConfig) -> (String, String) {
    match cfg {
        LlmConfig::Claude { model, .. } => ("claude".into(), model.clone()),
        LlmConfig::OpenAi { model, .. } => ("openai".into(), model.clone()),
        LlmConfig::Gemini { model, .. } => ("gemini".into(), model.clone()),
        LlmConfig::Ollama { model, .. } => ("ollama".into(), model.clone()),
        LlmConfig::LmStudio { model, .. } => ("lmstudio".into(), model.clone()),
    }
}

/// A `scan-progress` emitter for one pass. Returns a fresh `FnMut` each call so
/// the screen and extract passes don't share a moved closure.
fn make_scan_emit(app: &AppHandle) -> impl FnMut(&parsers::scan::Progress) {
    let app = app.clone();
    move |p: &parsers::scan::Progress| {
        let _ = app.emit(
            "scan-progress",
            serde_json::json!({
                "processed": p.processed,
                "total": p.total,
                "created": p.created,
                "skippedClassified": p.skipped_classified,
                "skippedSeen": p.skipped_seen,
                "skippedBlocked": p.skipped_blocked,
                "phase": p.phase,
            }),
        );
    }
}

/// Take a cached preview if it matches `signature`, consuming it so a repeat
/// scan re-fetches fresh next time.
fn take_cached_screen(state: &AppState, signature: &str) -> Option<parsers::scan::ScreenResult> {
    let mut guard = state.last_preview.lock().ok()?;
    if guard.as_ref().is_some_and(|c| c.signature == signature) {
        return guard.take().map(|c| c.screen);
    }
    None
}

/// Shared driver for the fast and deep Gmail scans: screen (or reuse a matching
/// preview), extract via the LLM, persist a summary, return the created count.
async fn do_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    range: Vec<YearMonthDto>,
    deep: bool,
    opts: ScanOptsDto,
) -> Result<usize, String> {
    let scan_opts = build_scan_opts(deep, &opts);
    let llm_cfg = load_llm_config(&state).await?;
    let access_token = ensure_gmail_token(&state).await?;
    let llm = LlmClient::from_config(llm_cfg);

    state.scan_cancel.store(false, Ordering::Relaxed);

    let range_months = range.len();
    let signature = scan_signature(&range, &scan_opts);
    let screen = match take_cached_screen(&state, &signature) {
        Some(cached) => cached,
        None => {
            let months = to_months(range);
            let query = parsers::gmail::build_query_for_range(
                &months,
                scan_opts.mode,
                scan_opts.use_purchases,
            );
            parsers::scan::screen(
                &state.pool,
                &access_token,
                &query,
                &scan_opts,
                &state.scan_cancel,
                make_scan_emit(&app),
            )
            .await
            .map_err(|e| e.to_string())?
        }
    };

    let counts = parsers::scan::extract(
        &state.pool,
        &llm,
        &screen,
        range_months,
        scan_opts.concurrency,
        &state.scan_cancel,
        make_scan_emit(&app),
    )
    .await
    .map_err(|e| e.to_string())?;

    let summary = parsers::scan::ScanSummary {
        ran_at: chrono::Utc::now(),
        mode: if deep { "deep" } else { "fast" }.to_string(),
        matched_estimate: screen.matched_estimate,
        listed: screen.listed as u32,
        llm_calls: screen.llm_targets() as u32,
        created: counts.created as u32,
        updated: counts.updated as u32,
        skipped_seen: screen.skipped_seen as u32,
        skipped_blocked: screen.skipped_blocked as u32,
        skipped_no_amount: screen.skipped_no_amount as u32,
        skipped_classified: counts.skipped_classified as u32,
        skipped_recurrence: screen.skipped_recurrence as u32,
    };
    db::settings::set(&state.pool, db::settings::keys::LAST_SCAN_SUMMARY, &summary)
        .await
        .map_err(|e| e.to_string())?;

    log::info!(
        "gmail scan done ({}): matched~{} listed={} llm={} created={} updated={} classified={} no_amount={} seen={} blocked={} recurrence={}",
        summary.mode,
        summary.matched_estimate,
        summary.listed,
        summary.llm_calls,
        summary.created,
        summary.updated,
        summary.skipped_classified,
        summary.skipped_no_amount,
        summary.skipped_seen,
        summary.skipped_blocked,
        summary.skipped_recurrence,
    );

    Ok(counts.created)
}

/// Stage 1 — tight, precision-favored Gmail scan.
#[tauri::command]
#[specta::specta]
async fn run_gmail_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    range: Vec<YearMonthDto>,
    opts: ScanOptsDto,
) -> Result<usize, String> {
    do_scan(app, state, range, false, opts).await
}

/// Stage 2 — broader Gmail keywords plus a multi-month recurrence filter.
/// Only senders appearing in 2+ distinct months proceed to the LLM, so it
/// rewards selecting several months.
#[tauri::command]
#[specta::specta]
async fn run_gmail_deep_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    range: Vec<YearMonthDto>,
    opts: ScanOptsDto,
) -> Result<usize, String> {
    do_scan(app, state, range, true, opts).await
}

/// Dry run: list + screen only (no LLM), returning how many emails matched,
/// how many would reach the LLM, and an estimated token/cost figure. Caches
/// the screened survivors so a follow-up scan with the same parameters skips
/// re-fetching.
#[tauri::command]
#[specta::specta]
async fn preview_gmail_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    range: Vec<YearMonthDto>,
    deep: bool,
    opts: ScanOptsDto,
) -> Result<parsers::scan::ScanEstimate, String> {
    let scan_opts = build_scan_opts(deep, &opts);
    let llm_cfg = load_llm_config(&state).await?;
    let (provider, model) = llm_provider_model(&llm_cfg);
    let access_token = ensure_gmail_token(&state).await?;

    state.scan_cancel.store(false, Ordering::Relaxed);

    let signature = scan_signature(&range, &scan_opts);
    let months = to_months(range);
    let query =
        parsers::gmail::build_query_for_range(&months, scan_opts.mode, scan_opts.use_purchases);
    let screen = parsers::scan::screen(
        &state.pool,
        &access_token,
        &query,
        &scan_opts,
        &state.scan_cancel,
        make_scan_emit(&app),
    )
    .await
    .map_err(|e| e.to_string())?;

    let estimate = parsers::scan::estimate(&screen, &provider, &model);

    if let Ok(mut guard) = state.last_preview.lock() {
        *guard = Some(PreviewCache { signature, screen });
    }

    Ok(estimate)
}

/// Most recent scan summary for the dashboard card.
#[tauri::command]
#[specta::specta]
async fn get_last_scan_summary(
    state: State<'_, AppState>,
) -> Result<Option<parsers::scan::ScanSummary>, String> {
    db::settings::get(&state.pool, db::settings::keys::LAST_SCAN_SUMMARY)
        .await
        .map_err(|e| e.to_string())
}

/// Cooperatively cancel an in-flight `run_gmail_scan` loop. The loop checks
/// the flag between messages and bails out with `scan cancelled`.
#[tauri::command]
#[specta::specta]
async fn cancel_scan(state: State<'_, AppState>) -> Result<(), String> {
    state.scan_cancel.store(true, Ordering::Relaxed);
    Ok(())
}

/// Open the URL in the user's default browser (used by the error-dialog
/// action buttons that link to Google Cloud Console etc.).
#[tauri::command]
#[specta::specta]
fn open_url(url: String) -> Result<(), String> {
    webbrowser::open(&url).map_err(|e| format!("failed to open url: {e}"))
}

// ---- PayPal OAuth ----

#[tauri::command]
#[specta::specta]
async fn save_paypal_oauth_credentials(
    state: State<'_, AppState>,
    creds: OAuthCredentials,
) -> Result<(), String> {
    db::settings::set(&state.pool, db::settings::keys::PAYPAL_OAUTH_CREDS, &creds)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn get_paypal_oauth_credentials(
    state: State<'_, AppState>,
) -> Result<Option<OAuthCredentials>, String> {
    db::settings::get::<OAuthCredentials>(&state.pool, db::settings::keys::PAYPAL_OAUTH_CREDS)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn has_paypal_tokens(state: State<'_, AppState>) -> Result<bool, String> {
    let tokens: Option<Tokens> = db::settings::get(&state.pool, db::settings::keys::PAYPAL_TOKENS)
        .await
        .map_err(|e| e.to_string())?;
    Ok(tokens.is_some())
}

#[tauri::command]
#[specta::specta]
async fn disconnect_paypal(state: State<'_, AppState>) -> Result<(), String> {
    db::settings::delete(&state.pool, db::settings::keys::PAYPAL_TOKENS)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn start_paypal_oauth(state: State<'_, AppState>) -> Result<(), String> {
    let creds: OAuthCredentials =
        db::settings::get(&state.pool, db::settings::keys::PAYPAL_OAUTH_CREDS)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "PayPal OAuth credentials not configured".to_string())?;

    let cancel_rx = arm_oauth_cancel(&state);

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

    let code = tokio::select! {
        biased;
        _ = cancel_rx => return Err("oauth cancelled".into()),
        res = oauth::wait_for_oauth_code(listener) => res.map_err(|e| e.to_string())?,
        () = tokio::time::sleep(std::time::Duration::from_secs(120)) => {
            return Err("OAuth flow timed out after 2 minutes — try Connect PayPal again.".into());
        }
    };

    let tokens = oauth::exchange_paypal_code(&creds, &code, &redirect_uri)
        .await
        .map_err(|e| e.to_string())?;

    db::settings::set(&state.pool, db::settings::keys::PAYPAL_TOKENS, &tokens)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

// PayPal has no scan command: the Transaction Search API is business-tier only,
// so personal accounts (the target user) can never call it. PayPal subscriptions
// are discovered via Gmail receipt parsing and PayPal Activity CSV import. The
// OAuth connect flow is retained purely as "Log In with PayPal" identity
// verification, not as a data source.

// ---- Renewal notifications ----

const NOTIF_TITLE_TEMPLATE: &str = "{name} renews soon";
const NOTIF_BODY_TEMPLATE: &str = "Next charge: {date}";

async fn notification_templates(pool: &SqlitePool) -> (String, String) {
    let locale = db::settings::get::<String>(pool, db::settings::keys::USER_LOCALE)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "en".to_string());
    if locale == "en" {
        return (
            NOTIF_TITLE_TEMPLATE.to_string(),
            NOTIF_BODY_TEMPLATE.to_string(),
        );
    }
    let cache = db::settings::get::<HashMap<String, String>>(pool, &translations_key(&locale))
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let title = cache
        .get(NOTIF_TITLE_TEMPLATE)
        .cloned()
        .unwrap_or_else(|| NOTIF_TITLE_TEMPLATE.to_string());
    let body = cache
        .get(NOTIF_BODY_TEMPLATE)
        .cloned()
        .unwrap_or_else(|| NOTIF_BODY_TEMPLATE.to_string());
    (title, body)
}

async fn notify_renewals(handle: &AppHandle, pool: &SqlitePool) -> Result<usize, String> {
    let due = db::subscriptions::list_due_within(pool, 7)
        .await
        .map_err(|e| e.to_string())?;
    let (title_template, body_template) = notification_templates(pool).await;
    let mut sent = 0usize;
    for sub in &due {
        if let Some(date) = sub.next_billing_date {
            let title = title_template.replace("{name}", &sub.name);
            let body = body_template.replace("{date}", &date.to_string());
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
#[specta::specta]
async fn check_renewals_now(state: State<'_, AppState>, app: AppHandle) -> Result<usize, String> {
    notify_renewals(&app, &state.pool).await
}

// ---- iCalendar export ----

#[tauri::command]
#[specta::specta]
async fn export_subscriptions_ics(state: State<'_, AppState>) -> Result<String, String> {
    let subs = db::subscriptions::list(&state.pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(kinketsu_core::ics::export_subscriptions(&subs))
}

// ---- User preferences ----

#[tauri::command]
#[specta::specta]
async fn get_default_currency(state: State<'_, AppState>) -> Result<Option<String>, String> {
    db::settings::get::<String>(&state.pool, db::settings::keys::DEFAULT_CURRENCY)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn set_default_currency(state: State<'_, AppState>, currency: String) -> Result<(), String> {
    db::settings::set(&state.pool, db::settings::keys::DEFAULT_CURRENCY, &currency)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn get_user_locale(state: State<'_, AppState>) -> Result<Option<String>, String> {
    db::settings::get::<String>(&state.pool, db::settings::keys::USER_LOCALE)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn set_user_locale(state: State<'_, AppState>, locale: String) -> Result<(), String> {
    db::settings::set(&state.pool, db::settings::keys::USER_LOCALE, &locale)
        .await
        .map_err(|e| e.to_string())
}

// ---- Translation (LLM-driven, cached in settings) ----

fn translations_key(locale: &str) -> String {
    format!("{}{}", db::settings::keys::TRANSLATIONS_PREFIX, locale)
}

#[tauri::command]
#[specta::specta]
async fn get_translations(
    state: State<'_, AppState>,
    locale: String,
) -> Result<Option<HashMap<String, String>>, String> {
    db::settings::get::<HashMap<String, String>>(&state.pool, &translations_key(&locale))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn save_translations(
    state: State<'_, AppState>,
    locale: String,
    translations: HashMap<String, String>,
) -> Result<(), String> {
    db::settings::set(&state.pool, &translations_key(&locale), &translations)
        .await
        .map_err(|e| e.to_string())
}

/// Translate a batch of English strings to `target_locale` using the user's
/// configured LLM provider. Returns a map of same keys → translated values.
/// Placeholder tokens (`{name}`, `{count}`, etc.) must be preserved by the LLM.
#[tauri::command]
#[specta::specta]
async fn translate_strings(
    state: State<'_, AppState>,
    target_locale: String,
    strings: HashMap<String, String>,
) -> Result<HashMap<String, String>, String> {
    if strings.is_empty() {
        return Ok(HashMap::new());
    }

    let cfg: LlmConfig = db::settings::get(&state.pool, db::settings::keys::LLM_CONFIG)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no LLM provider configured".to_string())?;
    let llm = LlmClient::from_config(cfg);

    let user_content = serde_json::to_string(&strings).map_err(|e| e.to_string())?;

    let req = ExtractionRequest {
        system_prompt: format!(
            "You are a translator. The user supplies a JSON object whose values are user-interface strings written in English. Translate each value to the BCP-47 locale \"{target_locale}\". Keep the keys exactly as supplied. Preserve every placeholder token like {{count}}, {{currency}}, {{name}}, {{date}} verbatim — do not translate or remove them. The product name \"kinketsu\" must always appear as the bare ASCII string and is never translated. Return only the translations object as JSON."
        ),
        user_content,
        schema: serde_json::json!({
            "type": "object",
            "properties": {
                "translations": {
                    "type": "object",
                    "additionalProperties": { "type": "string" }
                }
            },
            "required": ["translations"]
        }),
    };

    let resp = llm.extract(req).await.map_err(|e| e.to_string())?;
    let obj = resp
        .data
        .get("translations")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "LLM response missing 'translations' object".to_string())?;

    let map: HashMap<String, String> = obj
        .iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
        .collect();
    Ok(map)
}

// ---- Exchange rates ----

#[tauri::command]
#[specta::specta]
async fn refresh_exchange_rates(state: State<'_, AppState>, base: String) -> Result<usize, String> {
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
#[specta::specta]
async fn list_exchange_rates(
    state: State<'_, AppState>,
    base: String,
) -> Result<Vec<ExchangeRate>, String> {
    db::exchange_rates::list_latest_for_base(&state.pool, &base)
        .await
        .map_err(|e| e.to_string())
}

// ---- CSV bulk import ----

#[tauri::command]
#[specta::specta]
async fn import_csv_text(state: State<'_, AppState>, text: String) -> Result<usize, String> {
    let cfg = db::settings::get::<LlmConfig>(&state.pool, db::settings::keys::LLM_CONFIG)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no LLM provider configured".to_string())?;

    let client = LlmClient::from_config(cfg);
    let hints = extract_many_from_text(&client, text)
        .await
        .map_err(|e| e.to_string())?;

    let mut created = 0usize;
    for hint in hints {
        let payload = serde_json::to_value(&hint).map_err(|e| e.to_string())?;
        let summary = hint
            .service_name
            .clone()
            .unwrap_or_else(|| "(unnamed)".to_string());
        let ev = DetectionEvent {
            id: Uuid::now_v7(),
            source: DetectionSource::CsvImport,
            source_ref: None,
            raw_summary: Some(summary),
            sender: None,
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

// ---- Detection events ----

#[tauri::command]
#[specta::specta]
async fn list_detection_events(state: State<'_, AppState>) -> Result<Vec<DetectionEvent>, String> {
    db::detection_events::list(&state.pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
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

    let name = hint
        .service_name
        .ok_or_else(|| "missing service_name".to_string())?;
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
        notes: hint
            .payment_method_hint
            .map(|h| format!("Payment hint: {h}")),
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

    // Learn: future scans treat this sender as known-good.
    if let Some(sender) = ev.sender.as_deref() {
        let _ = db::learned_senders::upsert(&state.pool, sender, LearnedDecision::Allow).await;
    }

    Ok(sub)
}

#[tauri::command]
#[specta::specta]
async fn confirm_detection_event_with_overrides(
    state: State<'_, AppState>,
    id: Uuid,
    sub: NewSubscription,
) -> Result<Subscription, String> {
    let ev = db::detection_events::get(&state.pool, id)
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

    if let Some(sender) = ev.sender.as_deref() {
        let _ = db::learned_senders::upsert(&state.pool, sender, LearnedDecision::Allow).await;
    }

    Ok(new_sub)
}

#[tauri::command]
#[specta::specta]
async fn reject_detection_event(state: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    // The rejected row stays in the table keyed by its merchant `source_ref`, so
    // a future scan's `find_by_source_ref` check skips re-surfacing it — no
    // sender blocklisting needed (and blocklisting a shared sender like a bank
    // or Google Play would wrongly suppress every other merchant under it).
    db::detection_events::update_status(&state.pool, id, DetectionStatus::Rejected, None)
        .await
        .map_err(|e| e.to_string())
}

/// Un-reject a detection: put it back in the pending review queue. Used to undo
/// an accidental reject.
#[tauri::command]
#[specta::specta]
async fn restore_detection_event(state: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    db::detection_events::update_status(&state.pool, id, DetectionStatus::Pending, None)
        .await
        .map_err(|e| e.to_string())
}

/// Reject a batch of pending detection events in one round-trip. Each id is
/// processed independently — same as calling `reject_detection_event` per id,
/// Returns the count actually rejected. Rejected rows stay keyed by merchant
/// `source_ref` so future scans skip them; no sender blocklisting (see
/// `reject_detection_event`).
#[tauri::command]
#[specta::specta]
async fn bulk_reject_detection_events(
    state: State<'_, AppState>,
    ids: Vec<Uuid>,
) -> Result<usize, String> {
    let mut rejected = 0usize;
    for id in ids {
        if let Err(e) =
            db::detection_events::update_status(&state.pool, id, DetectionStatus::Rejected, None)
                .await
        {
            log::warn!("bulk reject: update_status({id}) failed: {e}");
            continue;
        }
        rejected += 1;
    }
    Ok(rejected)
}

// ---- Learned senders ----

#[tauri::command]
#[specta::specta]
async fn list_learned_senders(state: State<'_, AppState>) -> Result<Vec<LearnedSender>, String> {
    db::learned_senders::list_all(&state.pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn delete_learned_sender(state: State<'_, AppState>, sender: String) -> Result<(), String> {
    db::learned_senders::delete(&state.pool, &sender)
        .await
        .map_err(|e| e.to_string())
}

// ---- Extraction pipeline ----

#[tauri::command]
#[specta::specta]
async fn extract_subscription_from_text(
    state: State<'_, AppState>,
    text: String,
) -> Result<ParsedSubscriptionHint, String> {
    let cfg = db::settings::get::<LlmConfig>(&state.pool, db::settings::keys::LLM_CONFIG)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no LLM provider configured".to_string())?;

    let client = LlmClient::from_config(cfg);
    match extract_from_text(&client, text)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(h) => Ok(h),
        None => Err("LLM classified this text as not a recurring subscription".to_string()),
    }
}

/// Build the tauri-specta command registry. Extracted from [`run`] so the
/// `export_bindings_round_trips` test can regenerate `bindings.ts` without
/// standing up a windowed Tauri app.
fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new().commands(tauri_specta::collect_commands![
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
        import_csv_text,
        list_detection_events,
        confirm_detection_event,
        confirm_detection_event_with_overrides,
        reject_detection_event,
        restore_detection_event,
        bulk_reject_detection_events,
        list_learned_senders,
        delete_learned_sender,
        refresh_exchange_rates,
        list_exchange_rates,
        get_default_currency,
        set_default_currency,
        get_user_locale,
        set_user_locale,
        get_translations,
        save_translations,
        translate_strings,
        export_subscriptions_ics,
        save_gmail_oauth_credentials,
        get_gmail_oauth_credentials,
        has_gmail_tokens,
        disconnect_gmail,
        cancel_oauth,
        cancel_scan,
        open_url,
        start_gmail_oauth,
        run_gmail_scan,
        run_gmail_deep_scan,
        preview_gmail_scan,
        get_last_scan_summary,
        save_paypal_oauth_credentials,
        get_paypal_oauth_credentials,
        has_paypal_tokens,
        disconnect_paypal,
        start_paypal_oauth,
        check_renewals_now,
    ])
}

/// Write the TypeScript bindings consumed by the SvelteKit frontend.
///
/// `DetectionEvent::parsed_payload` now carries a `#[specta(type = …)]`
/// override, so the generated file is fully typed — no `@ts-nocheck` escape
/// hatch and no synthesized `Value` alias required. The frontend imports these
/// types and command wrappers directly.
///
/// Desktop-only: the output path is relative to the dev machine's working
/// directory and only exists during `tauri dev` on a host. On Android/iOS the
/// app filesystem is read-only, so attempting the write there aborts the
/// process at startup — hence the `desktop` cfg gate.
#[cfg(all(debug_assertions, desktop))]
fn export_bindings(builder: &tauri_specta::Builder<tauri::Wry>) {
    builder
        .export(
            specta_typescript::Typescript::default(),
            "../src/lib/bindings.ts",
        )
        .expect("failed to export typescript bindings");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta_builder = specta_builder();

    #[cfg(all(debug_assertions, desktop))]
    export_bindings(&specta_builder);

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            specta_builder.mount_events(app);
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

            app.manage(AppState {
                pool,
                oauth_cancel: StdMutex::new(None),
                scan_cancel: Arc::new(AtomicBool::new(false)),
                last_preview: StdMutex::new(None),
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    /// Regenerate `bindings.ts` and assert the export is deterministic. Running
    /// `cargo test -p kinketsu-app export_bindings_round_trips` refreshes the
    /// committed bindings without a GUI build; a non-idempotent export (or a
    /// panicking specta type) fails here.
    #[test]
    fn export_bindings_round_trips() {
        super::export_bindings(&super::specta_builder());
        let first = std::fs::read_to_string("../src/lib/bindings.ts")
            .expect("bindings.ts written by first export");
        super::export_bindings(&super::specta_builder());
        let second = std::fs::read_to_string("../src/lib/bindings.ts")
            .expect("bindings.ts written by second export");
        assert_eq!(first, second, "binding export is not deterministic");
    }
}
