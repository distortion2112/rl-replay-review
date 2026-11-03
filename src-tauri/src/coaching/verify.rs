//! Deterministic checks on coaching text against its evidence pack (#11).
//!
//! Used on the optional LLM narrator's output (fail closed to templates) and
//! on the templates themselves in tests. Rules:
//! - every number matches a pack value within the rounding it's shown at
//! - no rank names other than the pack's current and target tiers
//! - every cited `[finding.id]` exists in the pack
//! - optionally, every sentence with a number cites a finding

use serde::Serialize;

use super::models::EvidencePack;
use super::text::tier_name;
use crate::benchmark::TIERS;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Violation {
    pub rule: &'static str,
    pub detail: String,
}

/// Numbers text may use that aren't pack values: the "80%" in "80% range".
const ALLOWED_CONSTANTS: [f64; 1] = [80.0];

fn pack_numbers(pack: &EvidencePack) -> Vec<f64> {
    let mut v = vec![pack.games as f64, pack.min_games as f64];
    v.extend(ALLOWED_CONSTANTS);
    for f in &pack.findings {
        v.extend([
            f.value,
            f.interval.0,
            f.interval.1,
            f.current_tier_median,
            f.target_tier_median,
            f.pct_current_tier,
            f.pct_target_tier,
            f.rank_signal,
            f.games as f64,
        ]);
    }
    v
}

/// Removes rank names (longest first, so "Grand Champion 1" isn't also read as
/// "Champion 1"), returning the remaining text and the tiers found.
fn strip_tiers(text: &str) -> (String, Vec<&'static str>) {
    let mut names: Vec<(String, &'static str)> = TIERS
        .iter()
        .flat_map(|t| [(tier_name(t).to_lowercase(), *t), (t.to_string(), *t)])
        .collect();
    names.sort_by_key(|(n, _)| std::cmp::Reverse(n.len()));
    let mut lower = text.to_lowercase();
    let mut found = Vec::new();
    for (name, tier) in names {
        while let Some(i) = lower.find(&name) {
            lower.replace_range(i..i + name.len(), &" ".repeat(name.len()));
            found.push(tier);
        }
    }
    (lower, found)
}

/// `[finding.id]` citations, and the text with them blanked out.
fn strip_citations(text: &str) -> (String, Vec<String>) {
    let mut out = String::with_capacity(text.len());
    let mut ids = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        let Some(len) = rest[start..].find(']') else { break };
        let inner = &rest[start + 1..start + len];
        out.push_str(&rest[..start]);
        if inner.contains('.') && !inner.contains(' ') {
            ids.push(inner.to_string());
            out.push_str(&" ".repeat(len + 1));
        } else {
            out.push_str(&rest[start..start + len + 1]);
        }
        rest = &rest[start + len + 1..];
    }
    out.push_str(rest);
    (out, ids)
}

/// Numbers in text with how many decimals they were written with. Thousands
/// separators are accepted; a leading '-' is kept.
fn numbers(text: &str) -> Vec<(f64, usize, String)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let starts = chars[i].is_ascii_digit()
            && (i == 0 || !(chars[i - 1].is_ascii_alphanumeric() || chars[i - 1] == '.'));
        if !starts {
            i += 1;
            continue;
        }
        let begin = if i > 0 && chars[i - 1] == '-' { i - 1 } else { i };
        let mut j = i;
        while j < chars.len()
            && (chars[j].is_ascii_digit()
                || (chars[j] == ',' && j + 3 < chars.len() + 1 && chars[j + 1..].iter().take(3).all(|c| c.is_ascii_digit()))
                || (chars[j] == '.' && j + 1 < chars.len() && chars[j + 1].is_ascii_digit()))
        {
            j += 1;
        }
        let raw: String = chars[begin..j].iter().collect();
        let clean = raw.replace(',', "");
        let decimals = clean.split_once('.').map_or(0, |(_, d)| d.len());
        if let Ok(x) = clean.parse::<f64>() {
            out.push((x, decimals, raw));
        }
        i = j;
    }
    out
}

/// Splits on '.', '!' or '?' followed by whitespace or the end, so decimals
/// like "344.0" stay inside their sentence.
fn sentences(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    for (i, &b) in bytes.iter().enumerate() {
        let ends = matches!(b, b'.' | b'!' | b'?') && bytes.get(i + 1).is_none_or(|n| n.is_ascii_whitespace());
        if ends {
            out.push(&text[start..=i]);
            start = i + 1;
        }
    }
    out.push(&text[start..]);
    out.into_iter().filter(|s| !s.trim().is_empty()).collect()
}

pub fn verify(text: &str, pack: &EvidencePack, require_citations: bool) -> Vec<Violation> {
    let mut violations = Vec::new();
    let allowed = [
        pack.current_tier.as_str(),
        pack.current_tier_used.as_str(),
        pack.target_tier.as_str(),
        pack.target_tier_used.as_str(),
    ];

    let (no_cites, cited) = strip_citations(text);
    for id in &cited {
        if !pack.findings.iter().any(|f| &f.id == id) {
            violations.push(Violation { rule: "unknown_citation", detail: id.clone() });
        }
    }
    let (plain, tiers) = strip_tiers(&no_cites);
    for t in tiers {
        if !allowed.contains(&t) {
            violations.push(Violation { rule: "extra_tier", detail: tier_name(t) });
        }
    }

    let known = pack_numbers(pack);
    for (x, decimals, raw) in numbers(&plain) {
        let tol = 0.5 * 10f64.powi(-(decimals as i32)) + 1e-9;
        if !known.iter().any(|k| (k - x).abs() <= tol) {
            violations.push(Violation { rule: "unknown_number", detail: raw });
        }
    }

    if require_citations {
        // Sentences are split on the text with citations still present
        for sentence in sentences(text) {
            let (s_plain, _) = strip_tiers(&strip_citations(sentence).0);
            if !numbers(&s_plain).is_empty() && !sentence.contains('[') {
                violations.push(Violation { rule: "uncited_claim", detail: sentence.trim().to_string() });
            }
        }
    }
    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_extraction() {
        let n: Vec<(f64, usize)> = numbers("Speed 1,523 uu/s, 12.3% and 0.82 per minute; f24; -4").iter().map(|(x, d, _)| (*x, *d)).collect();
        assert_eq!(n, vec![(1523.0, 0), (12.3, 1), (0.82, 2), (-4.0, 0)]);
    }

    #[test]
    fn longest_tier_names_win() {
        let (rest, found) = strip_tiers("Grand Champion 1 and Champion 1 and diamond-2");
        assert_eq!(found, vec!["grand-champion-1", "champion-1", "diamond-2"]);
        assert!(!rest.contains('1'));
    }

    #[test]
    fn sentences_keep_decimals() {
        assert_eq!(sentences("Uses 344.0 boost. Then 12.5% more!"), vec!["Uses 344.0 boost.", " Then 12.5% more!"]);
    }

    #[test]
    fn citations() {
        let (rest, ids) = strip_citations("Low boost [boost.bpm] and [note] here");
        assert_eq!(ids, vec!["boost.bpm"]);
        assert!(rest.contains("[note]"));
    }
}
