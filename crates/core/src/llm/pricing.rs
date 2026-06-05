//! Token estimation + provider pricing for the scan cost preview.
//!
//! Goal: give the user a *before-you-spend* estimate of how many tokens the
//! configured provider will be billed for, and roughly what it costs, before
//! a scan fires off one LLM call per surviving email.
//!
//! Honesty note on precision: we do **not** ship a per-provider BPE tokenizer
//! (that would bundle multi-megabyte vocab tables into a privacy-focused local
//! app, and Anthropic/Google tokenizers have no exact offline implementation
//! anyway). Instead we use a script-aware heuristic — CJK characters and Latin
//! characters tokenize at very different rates — and present cost as a **range**
//! rather than a single number. The caller is expected to label it as an
//! estimate. Local providers (Ollama / LM Studio) report zero cost.

/// USD price per 1,000,000 tokens, input and output, for a model.
#[derive(Debug, Clone, Copy)]
pub struct ModelPrice {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
}

/// Roughly how many output tokens one extraction call emits. The structured
/// output is a small fixed-shape JSON object, so this is a conservative
/// upper-ish bound used for the high end of the cost range.
pub const OUTPUT_TOKENS_PER_CALL: u32 = 220;

/// Approximate published list prices (USD / 1M tokens) as of early 2026.
/// Matched by substring on the model name so new point-releases inherit the
/// family price. Returns `None` for local providers, which are free.
///
/// These are deliberately coarse — they back a labeled *estimate*, not a bill.
#[must_use]
pub fn price_for(provider: &str, model: &str) -> Option<ModelPrice> {
    let p = |input_per_mtok, output_per_mtok| ModelPrice {
        input_per_mtok,
        output_per_mtok,
    };
    let m = model.to_ascii_lowercase();
    let price = match provider {
        "ollama" | "lmstudio" => return None,
        "claude" => {
            if m.contains("opus") {
                p(15.0, 75.0)
            } else if m.contains("haiku") {
                p(0.80, 4.0)
            } else {
                // sonnet / unknown claude
                p(3.0, 15.0)
            }
        }
        "openai" => {
            if m.contains("mini") || m.contains("nano") {
                p(0.15, 0.60)
            } else {
                // gpt-4o / 4.1 / o-series default
                p(2.50, 10.0)
            }
        }
        "gemini" => {
            if m.contains("flash") {
                p(0.10, 0.40)
            } else {
                // pro / unknown gemini
                p(1.25, 5.0)
            }
        }
        // Unknown remote provider: assume a mid-tier price so the estimate is
        // never silently zero.
        _ => p(3.0, 15.0),
    };
    Some(price)
}

fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{3040}'..='\u{30FF}'   // hiragana + katakana
        | '\u{3400}'..='\u{4DBF}' // CJK ext A
        | '\u{4E00}'..='\u{9FFF}' // CJK unified
        | '\u{F900}'..='\u{FAFF}' // CJK compatibility ideographs
        | '\u{FF00}'..='\u{FFEF}' // halfwidth/fullwidth forms
    )
}

/// Heuristic token count for `text`. CJK scripts pack ~1.5 chars per token;
/// Latin/space-delimited text averages ~4 chars per token. This straddles the
/// real tokenizers closely enough for a cost estimate.
#[must_use]
pub fn estimate_tokens(text: &str) -> u32 {
    let mut cjk = 0usize;
    let mut other = 0usize;
    for c in text.chars() {
        if is_cjk(c) {
            cjk += 1;
        } else {
            other += 1;
        }
    }
    ((cjk as f64 / 1.5) + (other as f64 / 4.0)).ceil() as u32
}

/// A low/high USD cost band for `input_tokens` of input plus an estimated
/// output, under `price`. The band widens by ±20% to acknowledge the
/// heuristic token count, and the high end carries the full output estimate.
#[must_use]
pub fn cost_band(input_tokens: u32, output_tokens: u32, price: ModelPrice) -> (f64, f64) {
    let input_cost = f64::from(input_tokens) / 1_000_000.0 * price.input_per_mtok;
    let output_cost = f64::from(output_tokens) / 1_000_000.0 * price.output_per_mtok;
    let mid = input_cost + output_cost;
    (mid * 0.8, mid * 1.25)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_providers_are_free() {
        assert!(price_for("ollama", "llama3").is_none());
        assert!(price_for("lmstudio", "qwen2.5").is_none());
    }

    #[test]
    fn claude_tiers_differ() {
        let opus = price_for("claude", "claude-opus-4-8").unwrap();
        let haiku = price_for("claude", "claude-haiku-4-5").unwrap();
        assert!(opus.input_per_mtok > haiku.input_per_mtok);
    }

    #[test]
    fn cjk_costs_more_tokens_than_latin_for_same_char_count() {
        let jp = estimate_tokens(&"あ".repeat(30));
        let en = estimate_tokens(&"a".repeat(30));
        assert!(jp > en);
    }

    #[test]
    fn cost_band_is_ordered_and_positive() {
        let price = price_for("claude", "sonnet").unwrap();
        let (lo, hi) = cost_band(10_000, 220, price);
        assert!(lo > 0.0 && hi > lo);
    }
}
