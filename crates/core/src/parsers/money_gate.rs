//! Deterministic "does this text mention a monetary amount?" gate.
//!
//! Runs *before* the LLM in the scan pipeline. A direct billing / renewal
//! email always states a charge amount; an email with no currency amount
//! anywhere in its body is overwhelmingly unlikely to be an actionable
//! subscription charge (there is nothing to record). Dropping those here
//! saves an LLM round-trip per message — the single biggest cost lever in a
//! large mailbox.
//!
//! This is intentionally *recall-favoring*: it only filters out mail with no
//! amount at all, so the rare "your plan renews next week" reminder with no
//! figure is the only realistic false-negative — and that mail has no amount
//! to extract anyway. Everything with a price still reaches the LLM, which
//! makes the final subscription/not-subscription call.

use std::sync::LazyLock;

use regex::Regex;

// A currency amount in any of the shapes receipts use:
//   ¥1,980  ￥1980  $9.99  €12  £7   (symbol then digits)
//   1,980円  1980 JPY  9.99 USD  980 yen  500 ドル  (digits then code/word)
//   JPY 1980  USD9.99  (code then digits)
//   9.99  (a bare 2-decimal amount — minor-unit price)
static AMOUNT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?ix)
        (?:
            [¥￥$€£]\s?\d                                              # symbol → digits
          | \d[\d,]*\s?(?:円|JPY|USD|EUR|GBP|yen|ドル|ユーロ|ポンド|元)  # digits → code/word
          | (?:JPY|USD|EUR|GBP)\s?\d                                  # code → digits
          | \b\d+\.\d{2}\b                                            # bare decimal amount
        )
    ",
    )
    .expect("amount regex")
});

/// True when `text` contains at least one recognizable currency amount.
#[must_use]
pub fn has_amount(text: &str) -> bool {
    AMOUNT_RE.is_match(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_yen_symbol_and_kanji() {
        assert!(has_amount("ご請求金額 ¥1,980"));
        assert!(has_amount("月額 1980円 のお支払い"));
        assert!(has_amount("お支払い ￥500"));
    }

    #[test]
    fn detects_western_currencies() {
        assert!(has_amount("Your card was charged $9.99 today"));
        assert!(has_amount("Total: 12.00 USD"));
        assert!(has_amount("Amount due €15"));
        assert!(has_amount("£7.50 per month"));
    }

    #[test]
    fn detects_code_then_digits() {
        assert!(has_amount("JPY 2980 will be charged"));
        assert!(has_amount("USD9.99/mo"));
    }

    #[test]
    fn detects_bare_decimal() {
        assert!(has_amount("Subtotal 19.99 before tax"));
    }

    #[test]
    fn rejects_amount_free_text() {
        assert!(!has_amount("Your password was reset successfully."));
        assert!(!has_amount(
            "新しいフォロワーがいます。プロフィールを確認しましょう。"
        ));
        assert!(!has_amount(
            "Welcome to the newsletter! Read this week's digest."
        ));
    }

    #[test]
    fn plain_integers_without_currency_do_not_match() {
        // Order numbers / dates / counts must not look like money.
        assert!(!has_amount("Order #100245 shipped on 2026-06-01"));
        assert!(!has_amount("You have 3 new messages"));
    }
}
