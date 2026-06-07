//! Merchant-name normalization and clustering.
//!
//! The same subscription surfaces under wildly different merchant strings
//! depending on the source: a Canva charge appears as `CANVA PTY LIMITED`
//! (PayPal receipt), `ＰＡＹＰＡＬ＊ＣＡＮＶＡＰＴＹＬＩＭ` (Sony Bank usage
//! notification, fullwidth + processor prefix + truncated), and `canva.com`
//! (the merchant's own email). To group these into one subscription we
//! normalize aggressively, then cluster by a brand token.
//!
//! This is deterministic and language-agnostic for Latin merchant codes
//! (which all the card-notification ledgers use). Japanese-script merchant
//! names (e.g. `フリー株式会社`) normalize consistently within a source but
//! won't cross-match a Latin variant — that residual de-dup is left to the
//! LLM's canonical-name output for freeform receipts.

/// Fold fullwidth ASCII (U+FF01–FF5E) to ASCII and the ideographic space to a
/// normal space. Card ledgers print merchant codes in fullwidth, so this is
/// what makes `ＧＯＯＧＬＥ＊ＹＯＵＴＵＢＥ` comparable to `GOOGLE*YOUTUBE`.
fn fold_fullwidth(s: &str) -> String {
    s.chars()
        .map(|c| match c as u32 {
            0xFF01..=0xFF5E => char::from_u32(c as u32 - 0xFEE0).unwrap_or(c),
            0x3000 => ' ',
            _ => c,
        })
        .collect()
}

/// Processor / wallet prefixes that wrap the real merchant on a bank line,
/// e.g. `PAYPAL *CANVA`, `GOOGLE*YOUTUBE`, `SQ *COFFEE`. Stripped so the brand
/// underneath is what we cluster on.
///
/// Deliberately excludes `AMAZON`/`APPLE`/`RAKUTEN`: those appear as the
/// merchant itself (`AMAZON CO JP` shopping, `APPLE.COM/BILL`) far more often
/// than as a processor wrapper, and stripping them mangles the brand.
const PROCESSOR_PREFIXES: &[&str] = &[
    "PAYPAL", "GOOGLE", "SQ", "SQUARE", "MERPAY", "STRIPE", "PADDLE",
];

/// Trailing corporate / plan noise that varies between sources for the same
/// brand (`CANVA PTY LIMITED` vs `CANVA`).
const SUFFIX_NOISE: &[&str] = &[
    "PTY",
    "LIMITED",
    "LIMITE",
    "LTD",
    "LLC",
    "INC",
    "INCORPORATED",
    "CORP",
    "CORPORATION",
    "CO",
    "COM",
    "JP",
    "SUBSCRIPTION",
    "SUBSCR",
    "PREMIUM",
    "PLAN",
    "MONTHLY",
    "ANNUAL",
];

/// Normalize a raw merchant string to an uppercase, ASCII-folded, prefix- and
/// punctuation-stripped form suitable for display-agnostic comparison.
#[must_use]
pub fn normalize(raw: &str) -> String {
    let folded = fold_fullwidth(raw).to_uppercase();
    // Replace separators/punctuation with spaces, keep alphanumerics and spaces.
    let mut cleaned: String = folded
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' {
                c
            } else {
                ' '
            }
        })
        .collect();
    cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");

    // Strip a leading processor prefix token (e.g. "PAYPAL CANVA" -> "CANVA").
    if let Some((first, rest)) = cleaned.split_once(' ')
        && PROCESSOR_PREFIXES.contains(&first)
        && !rest.is_empty()
    {
        cleaned = rest.to_string();
    }
    cleaned
}

/// A short brand key for clustering: the normalized form with trailing
/// corporate/plan noise tokens removed and spaces collapsed out. Two merchants
/// are considered the same brand when one brand key is a prefix of the other
/// (handles the bank ledger truncating `CANVA PTY LIMITED` to `CANVAPTYLIM`).
#[must_use]
pub fn brand_key(raw: &str) -> String {
    let norm = normalize(raw);
    let mut tokens: Vec<&str> = norm.split(' ').filter(|t| !t.is_empty()).collect();
    while tokens.len() > 1 && SUFFIX_NOISE.contains(tokens.last().unwrap()) {
        tokens.pop();
    }
    tokens.concat()
}

/// Merchants-of-record / checkout platforms whose name on a card or PayPal line
/// tells you nothing about the actual product (the real service sends its own
/// "via X" receipt). Charges under these names are merged into the matching
/// product by amount+date during aggregation.
// Matched as substrings against the folded, alphanumeric-only name. `LEMONSQUEEZ`
// (not `…Y`) also catches the truncated `PAYPAL *LEMONSQUEEZ` card line.
// `PAYPALWALLET` is the card label for a PayPal-balance-funded purchase where the
// real merchant wasn't passed through — uninformative, varies per charge.
const MERCHANTS_OF_RECORD: &[&str] = &[
    "LEMONSQUEEZ",
    "PADDLE",
    "FASTSPRING",
    "GUMROAD",
    "PAYPALWALLET",
];

/// True when `raw` is a merchant-of-record / payment platform rather than the
/// actual product (e.g. `Lemon Squeezy LLC`, `PADDLE.NET`). Checks the folded
/// name *before* processor-prefix stripping, since `PADDLE`/`PAYPAL` are
/// themselves stripped by [`brand_key`].
#[must_use]
pub fn is_merchant_of_record(raw: &str) -> bool {
    let folded: String = fold_fullwidth(raw)
        .to_uppercase()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    MERCHANTS_OF_RECORD.iter().any(|m| folded.contains(m))
}

fn title_case(token: &str) -> String {
    let mut chars = token.chars();
    match chars.next() {
        Some(first) => first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect(),
        None => String::new(),
    }
}

/// A best-effort human-readable name from a raw ledger merchant string, used
/// when no cleaner name (e.g. an LLM-canonicalized receipt name) is available.
/// Folds fullwidth, strips the processor prefix and trailing corporate/plan
/// noise, and title-cases. Not perfect for run-together codes, but readable —
/// the user can rename on confirm.
#[must_use]
pub fn display_clean(raw: &str) -> String {
    let norm = normalize(raw);
    let mut tokens: Vec<&str> = norm.split(' ').filter(|t| !t.is_empty()).collect();
    while tokens.len() > 1 && SUFFIX_NOISE.contains(tokens.last().unwrap()) {
        tokens.pop();
    }
    if tokens.is_empty() {
        return raw.trim().to_string();
    }
    tokens
        .iter()
        .map(|t| title_case(t))
        .collect::<Vec<_>>()
        .join(" ")
}

/// True when two raw merchant strings denote the same brand. Uses prefix
/// containment on the brand key (min 4 chars) so truncated ledger codes match
/// their fuller forms.
#[must_use]
pub fn same_merchant(a: &str, b: &str) -> bool {
    let (ka, kb) = (brand_key(a), brand_key(b));
    if ka.is_empty() || kb.is_empty() {
        return false;
    }
    if ka == kb {
        return true;
    }
    let (short, long) = if ka.len() <= kb.len() {
        (&ka, &kb)
    } else {
        (&kb, &ka)
    };
    short.len() >= 4 && long.starts_with(short.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_fullwidth_and_strips_processor_prefix() {
        // Sony Bank ledger string for YouTube Premium.
        assert_eq!(
            normalize("ＧＯＯＧＬＥ＊ＹＯＵＴＵＢＥＰＲＥＭＩＵＭ"),
            "YOUTUBEPREMIUM"
        );
        assert_eq!(normalize("PAYPAL *CANVAPTYLIM"), "CANVAPTYLIM");
        assert_eq!(normalize("GOOGLE*APPLE MUSIC"), "APPLE MUSIC");
    }

    #[test]
    fn canva_variants_cluster_across_sources() {
        // PayPal receipt / Sony ledger / merchant email all → same brand.
        assert!(same_merchant("CANVA PTY LIMITED", "PAYPAL *CANVAPTYLIM"));
        assert!(same_merchant("CANVA PTY LIMITED", "canva.com"));
        assert!(same_merchant(
            "ＰＡＹＰＡＬ＊ＣＡＮＶＡＰＴＹＬＩＭ",
            "Canva"
        ));
    }

    #[test]
    fn distinct_brands_do_not_cluster() {
        // Different Google products must stay separate.
        assert!(!same_merchant(
            "GOOGLE*YOUTUBEPREMIUM",
            "GOOGLE*YOUTUBE SUPER"
        ));
        assert!(!same_merchant("NETFLIX.COM", "SPOTIFY"));
        assert!(!same_merchant("GOOGLE*APPLE MUSIC", "GOOGLE*FITBIT"));
    }

    #[test]
    fn slate_and_splice_real_strings() {
        assert!(same_merchant("SLATE DIGITAL LLC", "PAYPAL *SLATE DIGIT"));
        assert!(same_merchant("PAYPAL *SPLICE", "Splice"));
    }

    #[test]
    fn recognizes_merchants_of_record() {
        assert!(is_merchant_of_record("Lemon Squeezy LLC"));
        assert!(is_merchant_of_record("PADDLE.NET"));
        assert!(!is_merchant_of_record("Canva Pty Limited"));
        assert!(!is_merchant_of_record("3D AI Studio"));
    }

    #[test]
    fn brand_key_drops_corporate_suffixes() {
        assert_eq!(brand_key("CANVA PTY LIMITED"), "CANVA");
        assert_eq!(brand_key("SLATE DIGITAL LLC"), "SLATEDIGITAL");
        assert_eq!(brand_key("Toggl OÜ"), "TOGGLOÜ".to_string());
    }
}
