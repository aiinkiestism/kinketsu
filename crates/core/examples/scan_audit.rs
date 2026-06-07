//! Offline audit: run the real notification-parse + merchant-aggregate code
//! path against the live inbox (using the app's stored OAuth tokens) and print
//! the resulting subscription candidates. No LLM, no raw bodies — just the
//! deduped per-merchant list, to verify the redesigned scan recovers the real
//! subscriptions. Run with:
//!   cargo run -p kinketsu-core --example scan_audit

use std::collections::HashSet;

use chrono::Datelike;
use futures::StreamExt;
use kinketsu_core::db;
use kinketsu_core::oauth::{self, OAuthCredentials, Tokens};
use kinketsu_core::parsers::gmail::{self, YearMonth};
use kinketsu_core::parsers::notifications::{self, SourceKind};
use kinketsu_core::parsers::scan::{ChargeRecord, aggregate};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let home = std::env::var("HOME")?;
    let db_path = format!("{home}/Library/Application Support/dev.kinketsu.app/kinketsu.db");
    let pool = db::connect_file(std::path::Path::new(&db_path)).await?;

    let creds: OAuthCredentials = db::settings::get(&pool, db::settings::keys::GMAIL_OAUTH_CREDS)
        .await?
        .expect("gmail creds");
    let mut tokens: Tokens = db::settings::get(&pool, db::settings::keys::GMAIL_TOKENS)
        .await?
        .expect("gmail tokens");
    let token = oauth::ensure_access_token(&creds, &mut tokens).await?;

    // Last 12 months.
    let months: Vec<YearMonth> = (0..12)
        .map(|i| {
            let m0 = 6i32 - 1 - i; // anchor June 2026
            let (y, m) = (2026 + m0.div_euclid(12), m0.rem_euclid(12) + 1);
            YearMonth {
                year: y,
                month: m as u32,
            }
        })
        .collect();
    // List per month so every month is fully covered (a global cap would drop
    // the older months in this high-volume inbox).
    let mut ids: Vec<String> = Vec::new();
    for m in &months {
        let q = gmail::build_query_for_range(std::slice::from_ref(m), false);
        ids.extend(gmail::list_message_ids(&token, &q, 1000).await?.ids);
    }
    ids.sort_unstable();
    ids.dedup();
    eprintln!(
        "listed {} messages across {} months",
        ids.len(),
        months.len()
    );

    // Fetch concurrently, parse only the deterministic notification path.
    let token = &token;
    let futs = ids.into_iter().map(|id| async move {
        let msg = gmail::fetch_message(token, &id).await.ok()?;
        let r = gmail::message_ref_from(&msg, &id);
        let sender = r.from.as_deref().map(gmail::normalize_sender)?;
        let body = gmail::extract_text_body(&msg)?;
        let subj = r.subject.clone().unwrap_or_default();
        let n = notifications::parse_known(&sender, &subj, &body)?;
        Some(ChargeRecord {
            message_id: id,
            sender: Some(sender),
            subject: r.subject,
            month: r.received_at.map(|d| (d.year(), d.month())),
            merchant_raw: n.merchant_raw,
            display_name: None,
            amount_minor: n.amount_minor,
            currency: n.currency,
            billing_cycle: None,
            charged_on: r.received_at.map(|d| d.date_naive()),
            kind: n.kind,
        })
    });
    let mut stream = futures::stream::iter(futs).buffer_unordered(12);
    let mut records = Vec::new();
    while let Some(r) = stream.next().await {
        if let Some(r) = r {
            records.push(r);
        }
    }
    eprintln!("parsed {} notification charges\n", records.len());

    let mut cands = aggregate(records, 12);
    cands.sort_by_key(|c| {
        (
            std::cmp::Reverse(c.looks_like_subscription()),
            std::cmp::Reverse(c.months),
        )
    });

    let kind = |k: SourceKind| match k {
        SourceKind::MerchantReceipt => "rcpt",
        SourceKind::ProcessorNotification => "proc",
        SourceKind::CardNotification => "card",
    };
    let subs: Vec<_> = cands
        .iter()
        .filter(|c| c.looks_like_subscription())
        .collect();
    println!("=== SUBSCRIPTIONS ({}) ===", subs.len());
    println!(
        "{:>2} {:>4} {:>11} {:>5} {:>10}  name",
        "mo", "occ", "amount", "src", "cycle"
    );
    for c in &subs {
        let amt = c
            .amount_minor
            .map(|a| format!("{} {}", a, c.currency.as_deref().unwrap_or("")))
            .unwrap_or_default();
        println!(
            "{:>2} {:>4} {:>11} {:>5} {:>10?}  {}",
            c.months,
            c.occurrences,
            amt,
            kind(c.kind),
            c.billing_cycle,
            c.name
        );
    }

    let dropped: Vec<_> = cands
        .iter()
        .filter(|c| !c.looks_like_subscription() && c.months >= 2)
        .collect();
    let mut seen = HashSet::new();
    println!(
        "\n=== recurring but NOT classified as subscription ({}) ===",
        dropped.len()
    );
    for c in &dropped {
        if seen.insert(c.brand_key.clone()) {
            println!("{:>2}mo {:>4}occ  {}", c.months, c.occurrences, c.name);
        }
    }
    Ok(())
}
