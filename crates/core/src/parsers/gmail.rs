//! Gmail integration: query construction, message listing, body extraction,
//! and one-shot extraction via the configured LLM.

use base64::Engine;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::ParsedSubscriptionHint;
use crate::llm::LlmClient;
use crate::{Error, Result};

const API_BASE: &str = "https://gmail.googleapis.com/gmail/v1/users/me";

/// Stage 1 (default) — high precision query. Only subscription-specific
/// vocabulary so one-off Starbucks / Uber Eats / retail receipts don't enter
/// the LLM pipeline at all.
pub const TIGHT_KEYWORDS: &str = "(subscription OR renewal OR \"auto-renewal\" OR \"recurring payment\" OR \"recurring charge\" OR 更新 OR 定期 OR サブスク OR 月額 OR 年額)";

/// Stage 2 (deeper, opt-in) — broader catch-all that picks up generic
/// receipts and invoices. Used together with a recurrence filter so the
/// extra noise gets squeezed out downstream.
pub const BROAD_KEYWORDS: &str = "(invoice OR receipt OR subscription OR \"recurring payment\" OR renewal OR \"請求\" OR \"明細\" OR \"領収\" OR 更新 OR 定期 OR サブスク OR 月額 OR 年額)";

const BODY_CHAR_LIMIT: usize = 8000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanMode {
    /// Stage 1 — tight, precision-favored keywords.
    Fast,
    /// Stage 2 — broad keywords; the caller is expected to apply a
    /// recurrence filter on the results.
    Deep,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type)]
pub struct YearMonth {
    pub year: i32,
    pub month: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailMessageRef {
    pub message_id: String,
    pub subject: Option<String>,
    pub from: Option<String>,
    /// Best-effort timestamp the message was received. Prefers Gmail's
    /// `internalDate` epoch (most accurate); falls back to the RFC 2822
    /// `Date` header.
    pub received_at: Option<DateTime<Utc>>,
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let (ny, nm) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(ny, nm, 1)
        .map(|d| d.pred_opt().map(|p| p.day()).unwrap_or(28))
        .unwrap_or(28)
}

/// Build a Gmail search query for the configured keywords across the supplied
/// year/month ranges. Each month becomes an `after:Y/M/01 before:Y/M/last_day`
/// clause, ORed together. Empty range omits the date filter.
#[must_use]
pub fn build_query_for_range(months: &[YearMonth], mode: ScanMode) -> String {
    let keywords = match mode {
        ScanMode::Fast => TIGHT_KEYWORDS,
        ScanMode::Deep => BROAD_KEYWORDS,
    };
    let mut date_clauses = Vec::new();
    for ym in months {
        let last = last_day_of_month(ym.year, ym.month);
        date_clauses.push(format!(
            "(after:{}/{:02}/01 before:{}/{:02}/{:02})",
            ym.year, ym.month, ym.year, ym.month, last
        ));
    }
    if date_clauses.is_empty() {
        keywords.to_string()
    } else {
        format!("{keywords} ({})", date_clauses.join(" OR "))
    }
}

fn url_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

/// List message IDs matching `query`. Paginates until exhausted or a 500-row
/// safety cap is reached.
pub async fn list_message_ids(access_token: &str, query: &str) -> Result<Vec<String>> {
    const HARD_CAP: usize = 500;
    let mut out: Vec<String> = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut url = format!("{API_BASE}/messages?q={}&maxResults=100", url_encode(query));
        if let Some(pt) = page_token.as_ref() {
            url.push_str("&pageToken=");
            url.push_str(pt);
        }
        let resp = reqwest::Client::new()
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::Parser(format!("gmail list {status}: {text}")));
        }
        let v: Value = resp.json().await?;
        if let Some(arr) = v.get("messages").and_then(Value::as_array) {
            for m in arr {
                if let Some(id) = m.get("id").and_then(Value::as_str) {
                    out.push(id.to_string());
                    if out.len() >= HARD_CAP {
                        return Ok(out);
                    }
                }
            }
        }
        page_token = v
            .get("nextPageToken")
            .and_then(Value::as_str)
            .map(String::from);
        if page_token.is_none() {
            break;
        }
    }
    Ok(out)
}

pub async fn fetch_message(access_token: &str, message_id: &str) -> Result<Value> {
    let url = format!("{API_BASE}/messages/{message_id}?format=full");
    let resp = reqwest::Client::new()
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(Error::Parser(format!("gmail get {status}: {text}")));
    }
    let v: Value = resp.json().await?;
    Ok(v)
}

fn extract_header<'a>(headers: &'a Value, name: &str) -> Option<&'a str> {
    headers.as_array()?.iter().find_map(|h| {
        let n = h.get("name")?.as_str()?;
        if n.eq_ignore_ascii_case(name) {
            h.get("value")?.as_str()
        } else {
            None
        }
    })
}

/// Pull text from a Gmail message: prefer text/plain parts, fall back to a
/// crudely HTML-stripped text/html.
#[must_use]
pub fn extract_text_body(message: &Value) -> Option<String> {
    let payload = message.get("payload")?;
    if let Some(plain) = extract_from_part(payload, "text/plain") {
        return Some(plain);
    }
    extract_from_part(payload, "text/html").map(|h| strip_html(&h))
}

fn extract_from_part(part: &Value, prefer_mime: &str) -> Option<String> {
    let mime = part.get("mimeType").and_then(Value::as_str).unwrap_or("");
    if mime == prefer_mime
        && let Some(data) = part.pointer("/body/data").and_then(Value::as_str)
    {
        let bytes = base64::engine::general_purpose::URL_SAFE
            .decode(data)
            .ok()?;
        return String::from_utf8(bytes).ok();
    }
    if let Some(parts) = part.get("parts").and_then(Value::as_array) {
        for sub in parts {
            if let Some(s) = extract_from_part(sub, prefer_mime) {
                return Some(s);
            }
        }
    }
    None
}

fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    // Collapse runs of whitespace
    let mut compact = String::with_capacity(out.len());
    let mut last_was_space = false;
    for c in out.chars() {
        if c.is_whitespace() {
            if !last_was_space {
                compact.push(' ');
                last_was_space = true;
            }
        } else {
            compact.push(c);
            last_was_space = false;
        }
    }
    compact.trim().to_string()
}

/// Trim a body to the LLM-context limit while keeping the head of the message
/// (where receipts usually surface the amount + service name).
fn trim_body(body: String) -> String {
    if body.len() <= BODY_CHAR_LIMIT {
        body
    } else {
        body.chars().take(BODY_CHAR_LIMIT).collect()
    }
}

pub fn message_ref_from(message: &Value, id: &str) -> GmailMessageRef {
    let headers = message.pointer("/payload/headers");
    let subject = headers
        .and_then(|h| extract_header(h, "Subject"))
        .map(String::from);
    let from = headers
        .and_then(|h| extract_header(h, "From"))
        .map(String::from);

    // `internalDate` is the Gmail-side timestamp (epoch millis as a string).
    // Fall back to the message's Date header if it's not present.
    let received_at = message
        .get("internalDate")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i64>().ok())
        .and_then(DateTime::from_timestamp_millis)
        .or_else(|| {
            headers
                .and_then(|h| extract_header(h, "Date"))
                .and_then(|d| DateTime::parse_from_rfc2822(d).ok())
                .map(|dt| dt.with_timezone(&Utc))
        });

    GmailMessageRef {
        message_id: id.to_string(),
        subject,
        from,
        received_at,
    }
}

/// Fetch + extract body + run LLM extraction in one shot. Returns `Ok(None)`
/// when the LLM classifies the message as non-subscription.
pub async fn parse_message(
    provider: &LlmClient,
    access_token: &str,
    message_id: &str,
) -> Result<Option<ParsedSubscriptionHint>> {
    let msg = fetch_message(access_token, message_id).await?;
    let body = extract_text_body(&msg)
        .ok_or_else(|| Error::Parser(format!("gmail message {message_id}: no text body")))?;
    super::extract_from_text(provider, trim_body(body)).await
}
