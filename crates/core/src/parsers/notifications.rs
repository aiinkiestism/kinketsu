//! Deterministic parsers for structured payment notifications.
//!
//! Card / bank / wallet usage notifications follow fixed templates, so we can
//! pull (merchant, amount) out of them with regex — no LLM call. These are the
//! highest-volume mail in a real inbox (one per charge), so skipping the LLM
//! here is the main cost lever, and they form the most complete ledger of
//! recurring charges. The merchant string is messy (fullwidth, processor
//! prefixes, truncated) — [`super::merchant`] handles clustering it.
//!
//! Freeform merchant receipts (Anthropic, Canva's own mail, …) vary too much
//! for templates and fall through to the LLM extractor instead.

use std::sync::LazyLock;

use regex::Regex;

use serde::{Deserialize, Serialize};

/// Where a charge record came from. Drives cross-source de-duplication
/// preference (a merchant's own receipt wins over a processor or card line).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// The merchant's own receipt / invoice email.
    MerchantReceipt,
    /// A payment processor's notice (PayPal, Stripe, Braintree).
    ProcessorNotification,
    /// A bank / card-issuer usage notification.
    CardNotification,
}

/// A charge pulled deterministically from a known notification template.
#[derive(Debug, Clone)]
pub struct NotificationHint {
    pub merchant_raw: String,
    pub amount_minor: Option<i64>,
    pub currency: Option<String>,
    pub kind: SourceKind,
}

// --- amount parsing ---

// Leftmost currency amount: $12.34 (USD), ¥1,180 / ￥1,180 (JPY), 1,180円 (JPY).
static AMOUNT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:\$\s*([0-9][0-9,]*(?:\.[0-9]{1,2})?))|(?:[¥￥]\s*([0-9][0-9,]*))|(?:([0-9][0-9,]*)\s*円)")
        .expect("amount regex")
});

/// Parse the first currency amount in `text` into minor units + ISO code.
#[must_use]
pub fn parse_amount(text: &str) -> Option<(i64, String)> {
    let caps = AMOUNT_RE.captures(text)?;
    if let Some(m) = caps.get(1) {
        let v: f64 = m.as_str().replace(',', "").parse().ok()?;
        return Some(((v * 100.0).round() as i64, "USD".into()));
    }
    let yen = caps.get(2).or_else(|| caps.get(3))?;
    let v: i64 = yen.as_str().replace(',', "").parse().ok()?;
    Some((v, "JPY".into()))
}

// --- merchant label patterns ---

static SONY_MERCHANT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"ご利用加盟店[：:]\s*(.+)").expect("sony merchant"));
static MUFG_MERCHANT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"【ご利用先】\s*(.+)").expect("mufg merchant"));
// Generic JP card label fallback (covers PayPay Bank, au, アプラス, …).
static GENERIC_MERCHANT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:ご利用先|ご利用店名|ご利用先名|加盟店名?|利用先)[】\s：:]*([^\r\n【]+)")
        .expect("generic merchant")
});
// PayPal puts the merchant in the subject.
static PAYPAL_SUBJECT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(.+?)(?:への自動支払い|様への支払い|へのお支払い|への支払い)")
        .expect("paypal subject")
});

fn trim_merchant(s: &str) -> String {
    s.trim()
        .trim_matches(|c| c == '：' || c == ':' || c == '　')
        .trim()
        .to_string()
}

/// Try to parse a known structured notification. Returns `None` for senders /
/// formats we don't recognize (those go to the LLM).
#[must_use]
pub fn parse_known(sender: &str, subject: &str, body: &str) -> Option<NotificationHint> {
    let s = sender.to_ascii_lowercase();

    // PayPal — merchant in subject, amount in body.
    if s.contains("paypal.com") || s.contains("@paypal") {
        if let Some(c) = PAYPAL_SUBJECT.captures(subject) {
            // "お客さまはNintendo様への支払い…" → the merchant is "Nintendo".
            let captured = c[1].trim().strip_prefix("お客さまは").unwrap_or(&c[1]);
            let merchant = trim_merchant(captured);
            if !merchant.is_empty() {
                let amt = parse_amount(body);
                return Some(NotificationHint {
                    merchant_raw: merchant,
                    amount_minor: amt.as_ref().map(|a| a.0),
                    currency: amt.map(|a| a.1),
                    kind: SourceKind::ProcessorNotification,
                });
            }
        }
        return None;
    }

    // Sony Bank WALLET.
    if s.contains("sonybank.jp") {
        return card_hint(&SONY_MERCHANT, body);
    }
    // MUFG JCB debit.
    if s.contains("jcbdebit.bk.mufg.jp") || s.contains("mufg.jp") {
        return card_hint(&MUFG_MERCHANT, body).or_else(|| card_hint(&GENERIC_MERCHANT, body));
    }
    // Other JP card / bank usage notifications, recognized by subject phrasing.
    if (subject.contains("ご利用のお知らせ")
        || subject.contains("ご利用代金")
        || subject.contains("ご利用明細"))
        && (s.contains("bank") || s.contains("card") || s.contains("debit") || s.contains("jcb"))
    {
        return card_hint(&GENERIC_MERCHANT, body);
    }
    None
}

fn card_hint(re: &Regex, body: &str) -> Option<NotificationHint> {
    let m = re.captures(body)?;
    let merchant = trim_merchant(&m[1]);
    if merchant.is_empty() {
        return None;
    }
    let amt = parse_amount(body);
    Some(NotificationHint {
        merchant_raw: merchant,
        amount_minor: amt.as_ref().map(|a| a.0),
        currency: amt.map(|a| a.1),
        kind: SourceKind::CardNotification,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_amounts() {
        assert_eq!(
            parse_amount("ご利用金額：3,000円"),
            Some((3000, "JPY".into()))
        );
        assert_eq!(
            parse_amount("新町様、$31.90 USDのお支払い"),
            Some((3190, "USD".into()))
        );
        assert_eq!(parse_amount("¥1,180 JPY"), Some((1180, "JPY".into())));
        assert_eq!(parse_amount("no money here"), None);
    }

    #[test]
    fn parses_sony_bank() {
        let body = "ご利用金額：3,000円\nご利用加盟店：ＧＯＯＧＬＥ＊ＹＯＵＴＵＢＥ　ＳＵＰＥＲ\n配信設定…";
        let h = parse_known(
            "banking@sonybank.jp",
            "［ソニー銀行］ご利用のお知らせ",
            body,
        )
        .unwrap();
        assert_eq!(h.kind, SourceKind::CardNotification);
        assert_eq!(h.merchant_raw, "ＧＯＯＧＬＥ＊ＹＯＵＴＵＢＥ　ＳＵＰＥＲ");
        assert_eq!(h.amount_minor, Some(3000));
        assert_eq!(h.currency.as_deref(), Some("JPY"));
    }

    #[test]
    fn parses_mufg_debit() {
        let body =
            "【ご利用金額】　4,530円\n【ご利用先】　デイ−エムエムドツトコム\nご利用先名等は…";
        let h = parse_known(
            "mail@jcbdebit.bk.mufg.jp",
            "【三菱UFJ-JCBデビット】ご利用のお知らせ",
            body,
        )
        .unwrap();
        assert_eq!(h.merchant_raw, "デイ−エムエムドツトコム");
        assert_eq!(h.amount_minor, Some(4530));
    }

    #[test]
    fn parses_paypal_subject() {
        let h = parse_known(
            "service-jp@paypal.com",
            "Lemon Squeezy LLCへの自動支払いを行いました",
            "新町 恵史様、$31.90 USDのお支払いの領収書\n支払金額\n$31.90 USD",
        )
        .unwrap();
        assert_eq!(h.kind, SourceKind::ProcessorNotification);
        assert_eq!(h.merchant_raw, "Lemon Squeezy LLC");
        assert_eq!(h.amount_minor, Some(3190));
        assert_eq!(h.currency.as_deref(), Some("USD"));
    }

    #[test]
    fn paypal_strips_okyakusama_prefix() {
        let h = parse_known(
            "service-jp@paypal.com",
            "お客さまはNintendo様への支払いを承認されました",
            "¥6,578",
        )
        .unwrap();
        assert_eq!(h.merchant_raw, "Nintendo");
        assert_eq!(h.amount_minor, Some(6578));
    }

    #[test]
    fn unknown_sender_returns_none() {
        assert!(
            parse_known("no-reply@account.canva.com", "Your Canva invoice", "¥1,180").is_none()
        );
    }
}
