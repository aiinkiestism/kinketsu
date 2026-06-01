//! PII redaction before LLM round-trip.
//!
//! The goal is *defense in depth* against remote LLM providers (Claude /
//! OpenAI / Gemini) seeing personally identifiable data the model doesn't
//! need to do its job. The model only needs the merchant identity, amount,
//! currency, cycle, and date hints — addresses / phone numbers / card
//! numbers / personal email addresses are all dead weight.
//!
//! We pattern-redact and substitute deterministic placeholders
//! (`<EMAIL_1>`, `<CARD_1>`, …). Placeholders are 1-indexed per category
//! within a single message so a re-reading human can still tell two
//! occurrences apart.
//!
//! What is intentionally NOT redacted:
//! - Merchant / service names (would defeat the purpose of the extraction)
//! - Amounts, currencies, plan keywords
//! - Short order numbers (high false-positive rate vs. low PII value)
//! - Personal names (would require a NER model; out of scope for the
//!   regex-only first pass — users who want this should run a local LLM)

use std::sync::LazyLock;

use regex::Regex;

static EMAIL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b[a-z0-9._%+\-]+@[a-z0-9.\-]+\.[a-z]{2,}\b").expect("email regex")
});

// Card-like digit runs (13–19 digits) with optional space / hyphen separators.
// Luhn validation at use-site drops long order numbers that aren't card PANs.
static CARD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:\d[ \-]?){12,18}\d\b").expect("card regex")
});

// JP phone: 0X-XXXX-XXXX style. \b anchored so we don't slice into the
// middle of an IBAN digit group or other digit sequence.
static JP_PHONE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b0\d{1,4}[ \-]\d{1,4}[ \-]\d{3,4}\b").expect("jp phone regex")
});

// International phone with explicit "+" country code.
static INTL_PHONE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\+\d{1,3}[ \-]\d{1,4}[ \-]\d{1,4}[ \-]\d{3,4}").expect("intl phone regex")
});

// JP postal codes (NNN-NNNN). \b anchored so we don't false-positive on
// JP phone digit groups (e.g. inside "03-1111-2222").
static POSTAL_JP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d{3}-\d{4}\b").expect("postal regex"));

// IBAN: country (2 letters) + 2 check digits + 11–30 alphanumerics with
// optional space / hyphen separators between characters. Conservative —
// requires the leading uppercase country code so plain digit blocks don't
// match.
static IBAN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b[A-Z]{2}\d{2}(?:[ \-]?[A-Z0-9]){11,30}\b").expect("iban regex")
});

/// Apply the Luhn checksum to a sequence of ASCII digits. Used to drop card
/// candidates that aren't valid card numbers (likely order/transaction IDs).
fn luhn_ok(digits: &str) -> bool {
    let mut sum = 0u32;
    let mut alt = false;
    for c in digits.chars().rev() {
        let Some(d) = c.to_digit(10) else {
            return false;
        };
        let v = if alt {
            let dd = d * 2;
            if dd > 9 { dd - 9 } else { dd }
        } else {
            d
        };
        sum += v;
        alt = !alt;
    }
    sum % 10 == 0
}

fn digits_only(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// Redact PII before sending the text to a remote LLM. Returns the rewritten
/// string. The category counters reset per call so the redaction map for one
/// message is independent of another.
#[must_use]
pub fn redact(input: &str) -> String {
    // Order: IBAN before phone (otherwise the phone regex slices into the
    // IBAN's `\d+ \d+ \d+` digit groups and leaves nothing for IBAN_RE to
    // see). POSTAL is \b-anchored so it's safe to run first. EMAIL last
    // because the address never contributes to any other pattern.
    let mut out = input.to_string();

    let mut postal_n = 0;
    out = POSTAL_JP_RE
        .replace_all(&out, |_: &regex::Captures| {
            postal_n += 1;
            format!("<POSTAL_{postal_n}>")
        })
        .into_owned();

    let mut iban_n = 0;
    out = IBAN_RE
        .replace_all(&out, |_: &regex::Captures| {
            iban_n += 1;
            format!("<IBAN_{iban_n}>")
        })
        .into_owned();

    let mut phone_n = 0;
    out = INTL_PHONE_RE
        .replace_all(&out, |_: &regex::Captures| {
            phone_n += 1;
            format!("<PHONE_{phone_n}>")
        })
        .into_owned();
    out = JP_PHONE_RE
        .replace_all(&out, |_: &regex::Captures| {
            phone_n += 1;
            format!("<PHONE_{phone_n}>")
        })
        .into_owned();

    let mut card_n = 0;
    out = CARD_RE
        .replace_all(&out, |caps: &regex::Captures| {
            let m = caps.get(0).unwrap().as_str();
            let digits = digits_only(m);
            if luhn_ok(&digits) {
                card_n += 1;
                format!("<CARD_{card_n}>")
            } else {
                m.to_string()
            }
        })
        .into_owned();

    let mut email_n = 0;
    out = EMAIL_RE
        .replace_all(&out, |_: &regex::Captures| {
            email_n += 1;
            format!("<EMAIL_{email_n}>")
        })
        .into_owned();

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_addresses_are_redacted() {
        let out = redact("From: john.doe+receipts@example.com\nTo: support@netflix.co.jp");
        assert!(out.contains("<EMAIL_1>"));
        assert!(out.contains("<EMAIL_2>"));
        assert!(!out.contains("john.doe"));
        assert!(!out.contains("support@netflix"));
    }

    #[test]
    fn jp_phone_numbers_are_redacted() {
        let out = redact("お問い合わせ: 03-1234-5678 / 0120-555-666 / +81-90-1234-5678");
        assert!(out.contains("<PHONE_"));
        assert!(!out.contains("03-1234-5678"));
        assert!(!out.contains("0120-555-666"));
        assert!(!out.contains("+81-90-1234-5678"));
    }

    #[test]
    fn jp_postal_codes_are_redacted() {
        let out = redact("〒150-0001 東京都渋谷区神宮前 1-2-3");
        assert!(out.contains("<POSTAL_1>"));
        assert!(!out.contains("150-0001"));
    }

    #[test]
    fn valid_card_numbers_are_redacted_but_random_digits_pass_through() {
        // Visa test number — passes Luhn.
        let out = redact("Card on file: 4111 1111 1111 1111");
        assert!(out.contains("<CARD_1>"));
        assert!(!out.contains("4111 1111 1111 1111"));

        // 16 random digits unlikely to pass Luhn — should stay.
        let order = "Order ID: 1234567890123456";
        let out2 = redact(order);
        assert_eq!(out2, order, "non-Luhn digit run must not be redacted");
    }

    #[test]
    fn iban_is_redacted() {
        let out = redact("Bank: DE89 3704 0044 0532 0130 00");
        assert!(out.contains("<IBAN_1>"));
        assert!(!out.contains("DE89 3704"));
    }

    #[test]
    fn dates_amounts_and_service_names_are_preserved() {
        // The exact pieces the LLM actually needs must survive untouched.
        let body = "Netflix monthly plan — ¥1,980 charged on 2026-06-01. Next billing: 2026-07-01.";
        let out = redact(body);
        assert_eq!(out, body);
    }

    #[test]
    fn placeholders_are_independently_numbered_across_categories() {
        let out = redact("a@b.com 03-1111-2222 c@d.com");
        assert!(out.contains("<EMAIL_1>"));
        assert!(out.contains("<EMAIL_2>"));
        assert!(out.contains("<PHONE_1>"));
    }
}
