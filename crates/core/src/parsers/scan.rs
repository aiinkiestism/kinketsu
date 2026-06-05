//! Phased Gmail scan pipeline shared by the preview and the real scan.
//!
//! The scan is split so that the expensive LLM step is the *last* thing that
//! runs, and so a cost preview can execute everything up to (but not
//! including) it:
//!
//! 1. **list**   — [`gmail::list_message_ids`]: cheap, a few API calls; also
//!    yields Gmail's overall result-size estimate.
//! 2. **screen** — [`screen`]: fetch each listed message concurrently, drop
//!    the ones already seen, on the sender blocklist, body-less, or with no
//!    currency amount (the [`money_gate`]). Deep mode additionally keeps only
//!    senders recurring across ≥2 months. The survivors carry their trimmed
//!    body, so they can be tokenized for an exact-ish cost figure and reused
//!    by the extraction pass without re-fetching.
//! 3. **estimate** — [`estimate`]: tokenize survivor bodies + price them.
//!    Preview = list + screen + estimate.
//! 4. **extract** — [`extract`]: one LLM call per survivor (concurrent),
//!    inserting a `DetectionEvent` for each real subscription. Scan = list +
//!    screen + extract.
//!
//! Caps: `max_fetch` bounds step 1 (replacing the old hard-coded 500), and
//! `max_llm` bounds how many survivors reach step 4 — the cost-bearing knob.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Datelike, Utc};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{gmail, money_gate};
use crate::db;
use crate::llm::{LlmClient, pricing};
use crate::models::{DetectionEvent, DetectionSource, DetectionStatus, LearnedDecision};
use crate::parsers::gmail::ScanMode;
use crate::{Error, Result};

/// Sentinel returned by [`extract`] when the cooperative cancel flag is set
/// mid-pass. The frontend matches on this substring.
pub const CANCELLED: &str = "scan cancelled";

pub const DEFAULT_MAX_FETCH: usize = 500;
pub const DEFAULT_MAX_LLM: usize = 100;
/// In-flight Gmail/LLM requests per pass. I/O-bound, so a handful keeps the
/// pipeline busy without tripping provider rate limits.
pub const DEFAULT_CONCURRENCY: usize = 6;

/// Knobs for one scan/preview run.
#[derive(Debug, Clone, Copy)]
pub struct ScanOptions {
    pub mode: ScanMode,
    pub max_fetch: usize,
    pub max_llm: usize,
    pub use_purchases: bool,
    pub concurrency: usize,
}

impl ScanOptions {
    #[must_use]
    pub fn new(mode: ScanMode, max_fetch: usize, max_llm: usize, use_purchases: bool) -> Self {
        Self {
            mode,
            max_fetch: max_fetch.max(1),
            max_llm: max_llm.max(1),
            use_purchases,
            concurrency: DEFAULT_CONCURRENCY,
        }
    }
}

/// A message that survived screening and will be sent to the LLM. Carries its
/// trimmed body so it can be both priced and extracted without re-fetching.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub id: String,
    pub subject: Option<String>,
    pub sender: Option<String>,
    pub body: String,
    pub year_month: Option<(i32, u32)>,
}

/// Outcome of the screening pass — the survivor set plus a tally of why the
/// rest were dropped.
#[derive(Debug, Clone)]
pub struct ScreenResult {
    pub matched_estimate: u32,
    pub listed: usize,
    pub skipped_seen: usize,
    pub skipped_blocked: usize,
    pub skipped_no_body: usize,
    pub skipped_no_amount: usize,
    pub skipped_fetch: usize,
    pub skipped_recurrence: usize,
    pub truncated_by_max_llm: bool,
    pub candidates: Vec<Candidate>,
}

/// Progress tick handed to the caller's callback during a pass.
#[derive(Debug, Clone, Copy)]
pub struct Progress {
    pub phase: &'static str,
    pub processed: usize,
    pub total: usize,
    pub created: usize,
    pub skipped_classified: usize,
    pub skipped_seen: usize,
    pub skipped_blocked: usize,
}

/// Cost + counts preview returned to the UI before a scan commits to LLM spend.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ScanEstimate {
    /// Gmail's rough total for the query (can exceed `listed` when capped).
    pub matched_estimate: u32,
    /// Messages actually fetched + screened (≤ `max_fetch`).
    pub listed: u32,
    pub skipped_seen: u32,
    pub skipped_blocked: u32,
    pub skipped_no_body: u32,
    pub skipped_no_amount: u32,
    pub skipped_recurrence: u32,
    /// Survivors that will each cost one LLM call.
    pub llm_targets: u32,
    pub truncated_by_max_llm: bool,
    pub input_tokens: u32,
    pub output_tokens_est: u32,
    pub cost_low_usd: f64,
    pub cost_high_usd: f64,
    pub provider: String,
    pub model: String,
    /// True for Ollama / LM Studio — cost is zero.
    pub is_local: bool,
    /// How the figures were derived. Always `"approximate"` for now (heuristic
    /// token count + list prices); surfaced so the UI can say so.
    pub exactness: String,
}

/// Persisted record of the most recent scan, surfaced on the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ScanSummary {
    pub ran_at: DateTime<Utc>,
    pub mode: String,
    pub matched_estimate: u32,
    pub listed: u32,
    pub llm_calls: u32,
    pub created: u32,
    pub skipped_seen: u32,
    pub skipped_blocked: u32,
    pub skipped_no_amount: u32,
    pub skipped_classified: u32,
    pub skipped_recurrence: u32,
}

/// Counts from the extraction pass.
#[derive(Debug, Clone, Copy)]
pub struct ExtractCounts {
    pub created: usize,
    pub skipped_classified: usize,
    pub skipped_extract: usize,
}

enum Outcome {
    Seen,
    Blocked,
    NoBody,
    NoAmount,
    FetchErr,
    Candidate(Box<Candidate>),
}

/// Steps 1–2: list + concurrently fetch & screen. Returns survivors plus the
/// drop tally. `progress` is invoked once per message as it's screened.
pub async fn screen<P: FnMut(&Progress)>(
    pool: &SqlitePool,
    access_token: &str,
    query: &str,
    opts: &ScanOptions,
    cancel: &AtomicBool,
    mut progress: P,
) -> Result<ScreenResult> {
    let listing = gmail::list_message_ids(access_token, query, opts.max_fetch).await?;
    let total = listing.ids.len();
    let matched_estimate = listing.estimate;

    let futs = listing.ids.into_iter().map(|id| {
        let pool = pool.clone();
        async move {
            if let Ok(Some(_)) =
                db::detection_events::find_by_source_ref(&pool, DetectionSource::Gmail, &id).await
            {
                return Outcome::Seen;
            }
            let msg = match gmail::fetch_message(access_token, &id).await {
                Ok(m) => m,
                Err(_) => return Outcome::FetchErr,
            };
            let r = gmail::message_ref_from(&msg, &id);
            let sender = r.from.as_deref().map(gmail::normalize_sender);
            if let Some(s) = &sender
                && let Ok(Some(l)) = db::learned_senders::find(&pool, s).await
                && l.decision == LearnedDecision::Block
            {
                return Outcome::Blocked;
            }
            let Some(body) = gmail::extract_text_body(&msg) else {
                return Outcome::NoBody;
            };
            if !money_gate::has_amount(&body) {
                return Outcome::NoAmount;
            }
            Outcome::Candidate(Box::new(Candidate {
                id,
                subject: r.subject,
                sender,
                body: gmail::trim_body(body),
                year_month: r.received_at.map(|d| (d.year(), d.month())),
            }))
        }
    });

    let mut stream = futures::stream::iter(futs).buffer_unordered(opts.concurrency);

    let mut candidates: Vec<Candidate> = Vec::new();
    let mut skipped_seen = 0;
    let mut skipped_blocked = 0;
    let mut skipped_no_body = 0;
    let mut skipped_no_amount = 0;
    let mut skipped_fetch = 0;
    let mut processed = 0;

    while let Some(outcome) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Parser(CANCELLED.to_string()));
        }
        processed += 1;
        match outcome {
            Outcome::Seen => skipped_seen += 1,
            Outcome::Blocked => skipped_blocked += 1,
            Outcome::NoBody => skipped_no_body += 1,
            Outcome::NoAmount => skipped_no_amount += 1,
            Outcome::FetchErr => skipped_fetch += 1,
            Outcome::Candidate(c) => candidates.push(*c),
        }
        progress(&Progress {
            phase: "indexing",
            processed,
            total,
            created: 0,
            skipped_classified: skipped_no_amount,
            skipped_seen,
            skipped_blocked,
        });
    }

    // Deep mode: keep only senders that recur across ≥2 distinct months in the
    // selected range. (Fast mode trusts its tight keywords and keeps all.)
    let mut skipped_recurrence = 0;
    if opts.mode == ScanMode::Deep {
        let recurring: HashSet<String> = {
            let mut by_sender: HashMap<String, HashSet<(i32, u32)>> = HashMap::new();
            for c in &candidates {
                if let (Some(s), Some(ym)) = (c.sender.clone(), c.year_month) {
                    by_sender.entry(s).or_default().insert(ym);
                }
            }
            by_sender
                .into_iter()
                .filter(|(_, months)| months.len() >= 2)
                .map(|(s, _)| s)
                .collect()
        };
        let before = candidates.len();
        candidates.retain(|c| c.sender.as_deref().is_some_and(|s| recurring.contains(s)));
        skipped_recurrence = before - candidates.len();
    }

    let truncated_by_max_llm = candidates.len() > opts.max_llm;
    if truncated_by_max_llm {
        candidates.truncate(opts.max_llm);
    }

    Ok(ScreenResult {
        matched_estimate,
        listed: total,
        skipped_seen,
        skipped_blocked,
        skipped_no_body,
        skipped_no_amount,
        skipped_fetch,
        skipped_recurrence,
        truncated_by_max_llm,
        candidates,
    })
}

/// Step 3: price the survivors for the given provider/model.
#[must_use]
pub fn estimate(screen: &ScreenResult, provider: &str, model: &str) -> ScanEstimate {
    let overhead = crate::parsers::extraction_prompt_token_overhead();
    let mut input_tokens = 0u32;
    for c in &screen.candidates {
        input_tokens =
            input_tokens.saturating_add(pricing::estimate_tokens(&c.body).saturating_add(overhead));
    }
    let n = u32::try_from(screen.candidates.len()).unwrap_or(u32::MAX);
    let output_tokens_est = n.saturating_mul(pricing::OUTPUT_TOKENS_PER_CALL);

    let price = pricing::price_for(provider, model);
    let is_local = price.is_none();
    let (cost_low_usd, cost_high_usd) = match price {
        Some(p) => pricing::cost_band(input_tokens, output_tokens_est, p),
        None => (0.0, 0.0),
    };

    ScanEstimate {
        matched_estimate: screen.matched_estimate,
        listed: u32::try_from(screen.listed).unwrap_or(u32::MAX),
        skipped_seen: u32::try_from(screen.skipped_seen).unwrap_or(u32::MAX),
        skipped_blocked: u32::try_from(screen.skipped_blocked).unwrap_or(u32::MAX),
        skipped_no_body: u32::try_from(screen.skipped_no_body).unwrap_or(u32::MAX),
        skipped_no_amount: u32::try_from(screen.skipped_no_amount).unwrap_or(u32::MAX),
        skipped_recurrence: u32::try_from(screen.skipped_recurrence).unwrap_or(u32::MAX),
        llm_targets: n,
        truncated_by_max_llm: screen.truncated_by_max_llm,
        input_tokens,
        output_tokens_est,
        cost_low_usd,
        cost_high_usd,
        provider: provider.to_string(),
        model: model.to_string(),
        is_local,
        exactness: "approximate".to_string(),
    }
}

/// Step 4: one LLM call per survivor (concurrent), inserting a `DetectionEvent`
/// for each real subscription. DB inserts happen on the driving task as each
/// extraction completes, so SQLite writes stay serialized. Honors `cancel`.
pub async fn extract<P: FnMut(&Progress)>(
    pool: &SqlitePool,
    llm: &LlmClient,
    screen: &ScreenResult,
    concurrency: usize,
    cancel: &AtomicBool,
    mut progress: P,
) -> Result<ExtractCounts> {
    let total = screen.candidates.len();
    let mut created = 0;
    let mut skipped_classified = 0;
    let mut skipped_extract = 0;
    let mut processed = 0;

    // Each future takes an *owned* job tuple rather than `&Candidate`, so the
    // future never borrows the candidate slice — `buffer_unordered` can't prove
    // a borrowed argument outlives the out-of-order stream (a higher-ranked
    // lifetime error otherwise).
    type Job = (String, Option<String>, Option<String>, String);
    let jobs: Vec<Job> = screen
        .candidates
        .iter()
        .map(|c| {
            (
                c.id.clone(),
                c.subject.clone(),
                c.sender.clone(),
                c.body.clone(),
            )
        })
        .collect();
    let futs = jobs
        .into_iter()
        .map(|(id, subject, sender, body)| async move {
            let hint = super::extract_from_text(llm, body).await;
            (id, subject, sender, hint)
        });
    let mut stream = futures::stream::iter(futs).buffer_unordered(concurrency);

    while let Some((id, subject, sender, hint)) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Parser(CANCELLED.to_string()));
        }
        processed += 1;
        match hint {
            Ok(Some(h)) => {
                let payload = serde_json::to_value(&h).map_err(Error::from)?;
                let ev = DetectionEvent {
                    id: uuid::Uuid::now_v7(),
                    source: DetectionSource::Gmail,
                    source_ref: Some(id),
                    raw_summary: Some(subject.unwrap_or_else(|| "(no subject)".to_string())),
                    sender,
                    parsed_payload: payload,
                    confidence: 0.0,
                    status: DetectionStatus::Pending,
                    matched_subscription_id: None,
                    reviewed_at: None,
                    created_at: Utc::now(),
                };
                db::detection_events::insert(pool, &ev).await?;
                created += 1;
            }
            Ok(None) => skipped_classified += 1,
            Err(e) => {
                tracing::warn!("scan: LLM extract failed: {e}");
                skipped_extract += 1;
            }
        }
        progress(&Progress {
            phase: "extracting",
            processed,
            total,
            created,
            skipped_classified,
            skipped_seen: screen.skipped_seen,
            skipped_blocked: screen.skipped_blocked,
        });
    }

    Ok(ExtractCounts {
        created,
        skipped_classified,
        skipped_extract,
    })
}
