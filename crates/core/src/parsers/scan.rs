//! Phased Gmail scan pipeline shared by the preview and the real scan.
//!
//! Redesigned around **merchant-keyed recurrence** after studying a real inbox:
//!
//! - The most complete ledger of recurring charges is the pile of bank / card /
//!   wallet *usage notifications* (one per charge). These follow fixed
//!   templates, so [`notifications`] parses (merchant, amount) out of them with
//!   regex — no LLM call. That's both the main cost lever and the recall win.
//! - Aggregators (Google Play, PayPal, a bank) bundle many subscriptions under
//!   one sender, and the same subscription shows up across several sources
//!   (merchant email + PayPal + bank card). So recurrence and de-duplication
//!   key on the **merchant** ([`merchant`] clusters the messy strings), not the
//!   sender, and one subscription collapses to a single detection.
//! - Only freeform merchant receipts (which vary too much for templates) reach
//!   the LLM, gated by [`money_gate`].
//!
//! Phases: list → screen (fetch + deterministic-parse + money gate) →
//! estimate (price the freeform LLM targets) → extract (LLM the freeform, then
//! aggregate everything by merchant and insert one detection per merchant).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Datelike, Utc};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{gmail, merchant, money_gate, notifications};
use crate::db;
use crate::llm::{LlmClient, pricing};
use crate::models::{
    BillingCycle, DetectionEvent, DetectionSource, DetectionStatus, LearnedDecision,
};
use crate::parsers::ParsedSubscriptionHint;
use crate::parsers::gmail::ScanMode;
use crate::parsers::notifications::{NotificationHint, SourceKind};
use crate::{Error, Result};

/// Sentinel returned when the cooperative cancel flag is set mid-pass.
pub const CANCELLED: &str = "scan cancelled";

pub const DEFAULT_MAX_FETCH: usize = 800;
pub const DEFAULT_MAX_LLM: usize = 120;
/// In-flight Gmail/LLM requests per pass.
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

/// A screened message: either pre-parsed deterministically from a known
/// notification template (`notif = Some`, no LLM needed) or a freeform receipt
/// carrying its body for the LLM (`notif = None`).
#[derive(Debug, Clone)]
pub struct Candidate {
    pub id: String,
    pub subject: Option<String>,
    pub sender: Option<String>,
    pub body: String,
    pub year_month: Option<(i32, u32)>,
    pub notif: Option<NotificationHint>,
}

impl Candidate {
    fn is_freeform(&self) -> bool {
        self.notif.is_none()
    }
}

/// Outcome of the screening pass.
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

impl ScreenResult {
    /// Candidates that will each cost one LLM call (freeform receipts).
    #[must_use]
    pub fn llm_targets(&self) -> usize {
        self.candidates.iter().filter(|c| c.is_freeform()).count()
    }
}

/// Progress tick handed to the caller's callback.
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
    pub matched_estimate: u32,
    pub listed: u32,
    pub skipped_seen: u32,
    pub skipped_blocked: u32,
    pub skipped_no_body: u32,
    pub skipped_no_amount: u32,
    pub skipped_recurrence: u32,
    /// Freeform receipts that will each cost one LLM call. Bank/card/processor
    /// notifications are parsed deterministically and cost nothing.
    pub llm_targets: u32,
    /// Messages parsed from notification templates (free).
    pub notification_hits: u32,
    pub truncated_by_max_llm: bool,
    pub input_tokens: u32,
    pub output_tokens_est: u32,
    pub cost_low_usd: f64,
    pub cost_high_usd: f64,
    pub provider: String,
    pub model: String,
    pub is_local: bool,
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

// --- charge records & merchant aggregation ---

/// One charge pulled from a single message (deterministically or via the LLM),
/// before merchant-keyed aggregation.
#[derive(Debug, Clone)]
pub struct ChargeRecord {
    pub message_id: String,
    pub sender: Option<String>,
    pub subject: Option<String>,
    pub month: Option<(i32, u32)>,
    pub merchant_raw: String,
    /// A clean name if one source knew it (LLM receipt); else `None`.
    pub display_name: Option<String>,
    pub amount_minor: Option<i64>,
    pub currency: Option<String>,
    pub billing_cycle: Option<BillingCycle>,
    pub kind: SourceKind,
}

/// One merchant after aggregating its charges across the scanned range.
#[derive(Debug, Clone)]
pub struct SubscriptionCandidate {
    pub name: String,
    pub brand_key: String,
    pub amount_minor: Option<i64>,
    pub currency: Option<String>,
    pub billing_cycle: Option<BillingCycle>,
    pub months: usize,
    pub occurrences: usize,
    pub recurring: bool,
    pub amount_stable: bool,
    pub kind: SourceKind,
    pub sample_message_id: String,
    pub sample_sender: Option<String>,
    pub sample_subject: Option<String>,
}

fn source_rank(k: SourceKind) -> u8 {
    match k {
        SourceKind::MerchantReceipt => 2,
        SourceKind::ProcessorNotification => 1,
        SourceKind::CardNotification => 0,
    }
}

/// Cluster charge records by merchant and summarize each into a subscription
/// candidate. Cross-source duplicates of the same charge collapse here.
#[must_use]
pub fn aggregate(records: Vec<ChargeRecord>) -> Vec<SubscriptionCandidate> {
    // Greedy clustering by brand similarity.
    let mut clusters: Vec<Vec<ChargeRecord>> = Vec::new();
    for r in records {
        if let Some(cl) = clusters
            .iter_mut()
            .find(|cl| merchant::same_merchant(&cl[0].merchant_raw, &r.merchant_raw))
        {
            cl.push(r);
        } else {
            clusters.push(vec![r]);
        }
    }

    let mut out = Vec::with_capacity(clusters.len());
    for cl in clusters {
        let months: std::collections::HashSet<(i32, u32)> =
            cl.iter().filter_map(|r| r.month).collect();
        // Modal amount + how many distinct months carry it.
        let mut by_amount: HashMap<i64, std::collections::HashSet<(i32, u32)>> = HashMap::new();
        for r in &cl {
            if let (Some(a), Some(m)) = (r.amount_minor, r.month) {
                by_amount.entry(a).or_default().insert(m);
            }
        }
        let top = by_amount
            .iter()
            .max_by_key(|(_, ms)| ms.len())
            .map(|(a, ms)| (*a, ms.len()));
        let amount_stable = top.is_some_and(|(_, n)| n >= 2);

        // Prefer the highest-ranked source for the canonical name/amount/cycle.
        let best = cl
            .iter()
            .max_by_key(|r| source_rank(r.kind))
            .expect("non-empty cluster");
        let name = best
            .display_name
            .clone()
            .unwrap_or_else(|| merchant::display_clean(&best.merchant_raw));
        let currency = cl.iter().find_map(|r| r.currency.clone());
        let amount_minor = top.map(|(a, _)| a).or(best.amount_minor);
        let billing_cycle = cl.iter().find_map(|r| r.billing_cycle);
        let kind = cl
            .iter()
            .map(|r| r.kind)
            .max_by_key(|k| source_rank(*k))
            .unwrap();

        out.push(SubscriptionCandidate {
            name,
            brand_key: merchant::brand_key(&best.merchant_raw),
            amount_minor,
            currency,
            billing_cycle,
            months: months.len(),
            occurrences: cl.len(),
            recurring: months.len() >= 2,
            amount_stable,
            kind,
            sample_message_id: best.message_id.clone(),
            sample_sender: best.sender.clone(),
            sample_subject: best.subject.clone(),
        });
    }
    out
}

impl SubscriptionCandidate {
    /// Whether this merchant looks like a real subscription worth surfacing.
    ///
    /// The discriminator is **monthly cadence**, not amount stability: a USD
    /// subscription billed to a JP card shows a different ¥ amount every month
    /// (FX drift), so requiring a repeated amount wrongly drops Splice, Cursor,
    /// ChatGPT, Claude, Toggl, … Instead we ask "is this charged about once a
    /// month?". That excludes variable shopping (Amazon, food delivery) and
    /// high-frequency game IAP, which fire several times a month, while keeping
    /// every real subscription regardless of currency. A merchant's own receipt
    /// is trusted on its own; recall is favored (the user reviews and rejects).
    #[must_use]
    pub fn looks_like_subscription(&self) -> bool {
        let per_month = self.occurrences as f64 / self.months.max(1) as f64;
        let cadence_ok = per_month <= 3.0;
        if !cadence_ok {
            return false;
        }
        match self.kind {
            SourceKind::MerchantReceipt => true,
            _ => self.recurring,
        }
    }

    fn to_hint(&self) -> ParsedSubscriptionHint {
        ParsedSubscriptionHint {
            service_name: Some(self.name.clone()),
            amount_minor: self.amount_minor,
            currency: self.currency.clone(),
            billing_cycle: self.billing_cycle.or(if self.recurring {
                Some(BillingCycle::Monthly)
            } else {
                None
            }),
            payment_method_hint: None,
            charged_at: None,
            months_seen: Some(u32::try_from(self.months).unwrap_or(u32::MAX)),
            occurrences: Some(u32::try_from(self.occurrences).unwrap_or(u32::MAX)),
            recurring: Some(self.recurring),
            source_kind: Some(self.kind),
        }
    }
}

// --- pipeline ---

enum Outcome {
    Blocked,
    NoBody,
    NoAmount,
    FetchErr,
    Candidate(Box<Candidate>),
}

/// Phases 1–2: list + concurrently fetch & screen. Known notifications are
/// parsed deterministically (and kept regardless of the money gate); freeform
/// mail must show a currency amount to survive.
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
            let subj = r.subject.clone().unwrap_or_default();
            let notif = sender
                .as_deref()
                .and_then(|s| notifications::parse_known(s, &subj, &body));
            // Freeform mail with no amount is dropped; notifications are kept.
            if notif.is_none() && !money_gate::has_amount(&body) {
                return Outcome::NoAmount;
            }
            Outcome::Candidate(Box::new(Candidate {
                id,
                subject: r.subject,
                sender,
                body: gmail::trim_body(body),
                year_month: r.received_at.map(|d| (d.year(), d.month())),
                notif,
            }))
        }
    });

    let mut stream = futures::stream::iter(futs).buffer_unordered(opts.concurrency);

    let mut candidates: Vec<Candidate> = Vec::new();
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
            skipped_seen: 0,
            skipped_blocked,
        });
    }

    // Cap only the freeform (LLM-costed) candidates; keep all free notifications.
    let (notifs, mut freeform): (Vec<Candidate>, Vec<Candidate>) =
        candidates.into_iter().partition(|c| c.notif.is_some());
    let truncated_by_max_llm = freeform.len() > opts.max_llm;
    if truncated_by_max_llm {
        freeform.truncate(opts.max_llm);
    }
    let mut candidates = notifs;
    candidates.extend(freeform);

    Ok(ScreenResult {
        matched_estimate,
        listed: total,
        skipped_seen: 0,
        skipped_blocked,
        skipped_no_body,
        skipped_no_amount,
        skipped_fetch,
        skipped_recurrence: 0,
        truncated_by_max_llm,
        candidates,
    })
}

/// Phase 3: price the freeform LLM targets for the given provider/model.
#[must_use]
pub fn estimate(screen: &ScreenResult, provider: &str, model: &str) -> ScanEstimate {
    let overhead = crate::parsers::extraction_prompt_token_overhead();
    let mut input_tokens = 0u32;
    let mut llm_n = 0u32;
    let mut notif_n = 0u32;
    for c in &screen.candidates {
        if c.is_freeform() {
            llm_n += 1;
            input_tokens = input_tokens
                .saturating_add(pricing::estimate_tokens(&c.body).saturating_add(overhead));
        } else {
            notif_n += 1;
        }
    }
    let output_tokens_est = llm_n.saturating_mul(pricing::OUTPUT_TOKENS_PER_CALL);
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
        llm_targets: llm_n,
        notification_hits: notif_n,
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

/// Phase 4: LLM the freeform receipts, then aggregate every charge by merchant
/// and insert one `DetectionEvent` per merchant that looks like a subscription.
pub async fn extract<P: FnMut(&Progress)>(
    pool: &SqlitePool,
    llm: &LlmClient,
    screen: &ScreenResult,
    concurrency: usize,
    cancel: &AtomicBool,
    mut progress: P,
) -> Result<ExtractCounts> {
    let total = screen.candidates.len();
    let mut records: Vec<ChargeRecord> = Vec::new();
    let mut skipped_classified = 0;
    let mut skipped_extract = 0;
    let mut processed = 0;

    // Deterministic notification charges — no LLM.
    for c in screen.candidates.iter().filter(|c| c.notif.is_some()) {
        let n = c.notif.as_ref().unwrap();
        records.push(ChargeRecord {
            message_id: c.id.clone(),
            sender: c.sender.clone(),
            subject: c.subject.clone(),
            month: c.year_month,
            merchant_raw: n.merchant_raw.clone(),
            display_name: None,
            amount_minor: n.amount_minor,
            currency: n.currency.clone(),
            billing_cycle: None,
            kind: n.kind,
        });
        processed += 1;
    }
    progress(&Progress {
        phase: "extracting",
        processed,
        total,
        created: 0,
        skipped_classified,
        skipped_seen: screen.skipped_seen,
        skipped_blocked: screen.skipped_blocked,
    });

    // Freeform receipts — one LLM call each, concurrently.
    type Job = (
        String,
        Option<String>,
        Option<String>,
        Option<(i32, u32)>,
        String,
    );
    let jobs: Vec<Job> = screen
        .candidates
        .iter()
        .filter(|c| c.is_freeform())
        .map(|c| {
            (
                c.id.clone(),
                c.subject.clone(),
                c.sender.clone(),
                c.year_month,
                c.body.clone(),
            )
        })
        .collect();
    let futs = jobs
        .into_iter()
        .map(|(id, subject, sender, month, body)| async move {
            let hint = super::extract_from_text(llm, body).await;
            (id, subject, sender, month, hint)
        });
    let mut stream = futures::stream::iter(futs).buffer_unordered(concurrency);

    while let Some((id, subject, sender, month, hint)) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Parser(CANCELLED.to_string()));
        }
        processed += 1;
        match hint {
            Ok(Some(h)) => {
                let merchant_raw = h
                    .service_name
                    .clone()
                    .or_else(|| sender.clone())
                    .unwrap_or_default();
                records.push(ChargeRecord {
                    message_id: id,
                    sender,
                    subject,
                    month,
                    merchant_raw,
                    display_name: h.service_name.clone(),
                    amount_minor: h.amount_minor,
                    currency: h.currency.clone(),
                    billing_cycle: h.billing_cycle,
                    kind: SourceKind::MerchantReceipt,
                });
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
            created: 0,
            skipped_classified,
            skipped_seen: screen.skipped_seen,
            skipped_blocked: screen.skipped_blocked,
        });
    }

    // Aggregate by merchant and insert one detection per subscription-like merchant.
    let mut created = 0;
    for cand in aggregate(records) {
        if !cand.looks_like_subscription() {
            continue;
        }
        let source_ref = format!("merchant:{}", cand.brand_key);
        if db::detection_events::find_by_source_ref(pool, DetectionSource::Gmail, &source_ref)
            .await?
            .is_some()
        {
            continue; // already surfaced in a prior scan
        }
        let payload = serde_json::to_value(cand.to_hint()).map_err(Error::from)?;
        let ev = DetectionEvent {
            id: uuid::Uuid::now_v7(),
            source: DetectionSource::Gmail,
            source_ref: Some(source_ref),
            raw_summary: cand.sample_subject.clone(),
            sender: cand.sample_sender.clone(),
            parsed_payload: payload,
            confidence: if cand.recurring && cand.amount_stable {
                0.9
            } else {
                0.5
            },
            status: DetectionStatus::Pending,
            matched_subscription_id: None,
            reviewed_at: None,
            created_at: Utc::now(),
        };
        db::detection_events::insert(pool, &ev).await?;
        created += 1;
    }

    Ok(ExtractCounts {
        created,
        skipped_classified,
        skipped_extract,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(merchant: &str, amt: i64, ym: (i32, u32), kind: SourceKind) -> ChargeRecord {
        ChargeRecord {
            message_id: format!("{merchant}-{}-{}", ym.0, ym.1),
            sender: None,
            subject: None,
            month: Some(ym),
            merchant_raw: merchant.to_string(),
            display_name: None,
            amount_minor: Some(amt),
            currency: Some("JPY".into()),
            billing_cycle: None,
            kind,
        }
    }

    #[test]
    fn cross_source_duplicates_collapse_to_one_merchant() {
        // Canva charged once a month, seen via both PayPal and the Sony card line.
        let records = vec![
            rec(
                "CANVA PTY LIMITED",
                1180,
                (2026, 1),
                SourceKind::ProcessorNotification,
            ),
            rec(
                "PAYPAL *CANVAPTYLIM",
                1180,
                (2026, 1),
                SourceKind::CardNotification,
            ),
            rec(
                "CANVA PTY LIMITED",
                1180,
                (2026, 2),
                SourceKind::ProcessorNotification,
            ),
            rec(
                "PAYPAL *CANVAPTYLIM",
                1180,
                (2026, 2),
                SourceKind::CardNotification,
            ),
        ];
        let aggs = aggregate(records);
        assert_eq!(aggs.len(), 1, "all Canva charges cluster into one merchant");
        let c = &aggs[0];
        assert_eq!(c.months, 2);
        assert!(c.recurring && c.amount_stable);
        assert!(c.looks_like_subscription());
    }

    #[test]
    fn variable_amount_shopping_is_not_a_subscription() {
        // Amazon: recurs every month but amounts vary and there are many charges.
        let mut records = Vec::new();
        for (i, m) in (1..=9).enumerate() {
            for k in 0..6 {
                records.push(rec(
                    "AMAZON CO JP",
                    400 + (i * 100 + k) as i64,
                    (2026, m),
                    SourceKind::CardNotification,
                ));
            }
        }
        let aggs = aggregate(records);
        assert_eq!(aggs.len(), 1);
        assert!(
            !aggs[0].looks_like_subscription(),
            "variable high-frequency spend rejected"
        );
    }

    #[test]
    fn single_merchant_receipt_surfaces() {
        let aggs = aggregate(vec![rec(
            "Anthropic",
            3000,
            (2026, 6),
            SourceKind::MerchantReceipt,
        )]);
        assert!(aggs[0].looks_like_subscription());
    }

    #[test]
    fn distinct_merchants_stay_separate() {
        let aggs = aggregate(vec![
            rec(
                "GOOGLE*APPLE MUSIC",
                1080,
                (2026, 1),
                SourceKind::CardNotification,
            ),
            rec(
                "GOOGLE*FITBIT",
                640,
                (2026, 1),
                SourceKind::CardNotification,
            ),
        ]);
        assert_eq!(aggs.len(), 2);
    }
}
