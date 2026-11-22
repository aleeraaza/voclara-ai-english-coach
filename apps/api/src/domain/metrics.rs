//! The deterministic half of grading. The LLM judges language; everything
//! that must be exact and repeatable is computed here instead:
//! band snapping, the overall band, filler words and speaking speed.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// One transcript turn as stored on `sessions.transcript`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Turn {
    pub role: String,
    pub text: String,
    #[serde(default)]
    pub t_ms: u64,
}

pub fn learner_turns(turns: &[Turn]) -> impl Iterator<Item = &Turn> {
    turns.iter().filter(|t| t.role == "learner")
}

/// Clamp to 0..=9 and round *down* to the nearest 0.5. Rounding down is the
/// grader prompt's rule: "when unsure between two bands, pick the lower".
pub fn snap_band(raw: f64) -> f64 {
    if !raw.is_finite() {
        return 0.0;
    }
    (raw.clamp(0.0, 9.0) * 2.0).floor() / 2.0
}

/// IELTS-style overall band: the mean of the criteria, rounded to the nearest
/// half band, with .25 rounding up to .5 and .75 up to the next whole band.
/// Without a pronunciation score the mean is taken over three criteria.
pub fn overall_band(fluency: f64, lexical: f64, grammar: f64, pronunciation: Option<f64>) -> f64 {
    let (sum, n) = match pronunciation {
        Some(p) => (fluency + lexical + grammar + p, 4.0),
        None => (fluency + lexical + grammar, 3.0),
    };
    let mean = sum / n;
    // `+ 1e-9` keeps exact quarters (6.25) from landing a hair below .5.
    ((mean * 2.0 + 0.5 + 1e-9).floor() / 2.0).clamp(0.0, 9.0)
}

/// Words in the learner's speech, counting only tokens with a letter or digit.
pub fn word_count(text: &str) -> usize {
    text.split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .count()
}

pub fn learner_words(turns: &[Turn]) -> usize {
    learner_turns(turns).map(|t| word_count(&t.text)).sum()
}

/// Fillers that carry no meaning in learner speech. Kept conservative: words
/// like "so" or "like" are often used correctly, so they are not counted.
const FILLERS: &[&str] = &[
    "um",
    "umm",
    "uh",
    "uhh",
    "er",
    "erm",
    "hmm",
    "ah",
    "you know",
    "i mean",
    "basically",
    "literally",
    "kind of",
    "sort of",
    "matlab",
    "yani",
    "acha",
];

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct FillerCount {
    pub word: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct Fillers {
    pub total: u32,
    /// Fillers per 100 learner words.
    pub per_100_words: f64,
    /// Most frequent first.
    pub top: Vec<FillerCount>,
}

fn normalise(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || c == '\''))
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

pub fn fillers(turns: &[Turn]) -> Fillers {
    let mut counts: Vec<(&str, u32)> = FILLERS.iter().map(|f| (*f, 0)).collect();
    for turn in learner_turns(turns) {
        let words = normalise(&turn.text);
        for (filler, count) in counts.iter_mut() {
            let parts: Vec<&str> = filler.split(' ').collect();
            let hits = words
                .windows(parts.len())
                .filter(|w| w.iter().zip(&parts).all(|(a, b)| a == b))
                .count();
            *count += hits as u32;
        }
    }
    let total: u32 = counts.iter().map(|(_, c)| c).sum();
    let words = learner_words(turns);
    let mut top: Vec<FillerCount> = counts
        .into_iter()
        .filter(|(_, c)| *c > 0)
        .map(|(w, c)| FillerCount {
            word: w.to_owned(),
            count: c,
        })
        .collect();
    top.sort_by(|a, b| b.count.cmp(&a.count).then(a.word.cmp(&b.word)));
    top.truncate(5);
    Fillers {
        total,
        per_100_words: if words == 0 {
            0.0
        } else {
            (f64::from(total) * 1000.0 / words as f64).round() / 10.0
        },
        top,
    }
}

/// Longest gap counted as one learner turn's speaking time.
const MAX_TURN_MS: u64 = 60_000;
/// Below this much measured speech the estimate is noise.
const MIN_MEASURED_MS: u64 = 10_000;

/// Words per minute, estimated from caption timestamps: a learner turn lasts
/// until the next turn starts (capped). `None` when there is too little to
/// measure. Only an estimate: pauses before the tutor replies count too.
pub fn words_per_minute(turns: &[Turn]) -> Option<i32> {
    let mut words = 0usize;
    let mut ms = 0u64;
    for (i, t) in turns.iter().enumerate() {
        if t.role != "learner" {
            continue;
        }
        let Some(next) = turns.get(i + 1) else {
            continue;
        };
        let dur = next.t_ms.saturating_sub(t.t_ms).min(MAX_TURN_MS);
        if dur == 0 {
            continue;
        }
        words += word_count(&t.text);
        ms += dur;
    }
    if ms < MIN_MEASURED_MS || words == 0 {
        return None;
    }
    let wpm = words as f64 / (ms as f64 / 60_000.0);
    Some(wpm.round().clamp(0.0, 300.0) as i32)
}

/// Human name of an explanation language code, for the grader prompt.
pub fn language_name(l1: &str) -> &'static str {
    match l1 {
        "ur" => "Urdu",
        "hi" => "Hindi",
        "bn" => "Bengali",
        "ar" => "Arabic",
        _ => "English",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(role: &str, text: &str, t_ms: u64) -> Turn {
        Turn {
            role: role.into(),
            text: text.into(),
            t_ms,
        }
    }

    #[test]
    fn bands_snap_down_to_halves_and_clamp() {
        assert_eq!(snap_band(6.4), 6.0);
        assert_eq!(snap_band(6.5), 6.5);
        assert_eq!(snap_band(6.99), 6.5);
        assert_eq!(snap_band(12.0), 9.0);
        assert_eq!(snap_band(-1.0), 0.0);
        assert_eq!(snap_band(f64::NAN), 0.0);
    }

    #[test]
    fn overall_uses_ielts_style_rounding() {
        // mean 6.25 -> 6.5; 6.75 -> 7.0; 6.125 -> 6.0; 6.375 -> 6.5
        assert_eq!(overall_band(6.0, 6.0, 6.5, Some(6.5)), 6.5);
        assert_eq!(overall_band(7.0, 7.0, 6.5, Some(6.5)), 7.0);
        assert_eq!(overall_band(6.0, 6.0, 6.0, Some(6.5)), 6.0);
        assert_eq!(overall_band(6.5, 6.5, 6.0, Some(6.5)), 6.5);
        assert_eq!(overall_band(5.0, 5.0, 5.0, Some(5.0)), 5.0);
    }

    #[test]
    fn overall_without_pronunciation_averages_three() {
        // mean 6.1667 -> 6.0; mean 6.3333 -> 6.5
        assert_eq!(overall_band(6.0, 6.0, 6.5, None), 6.0);
        assert_eq!(overall_band(6.5, 6.5, 6.0, None), 6.5);
    }

    #[test]
    fn fillers_count_single_and_multi_word_in_learner_turns_only() {
        let turns = vec![
            t("tutor", "um, you know, tell me", 0),
            t(
                "learner",
                "Um I think, you know, uh it is basically good. Matlab good.",
                1000,
            ),
            t("learner", "I like cricket so much.", 5000),
        ];
        let f = fillers(&turns);
        assert_eq!(f.total, 5);
        assert_eq!(
            f.top[0],
            FillerCount {
                word: "basically".into(),
                count: 1
            }
        );
        assert!(f.top.iter().any(|x| x.word == "you know"));
        assert!(
            !f.top.iter().any(|x| x.word == "like"),
            "'like' is not counted"
        );
        assert!(f.per_100_words > 0.0);
    }

    #[test]
    fn words_per_minute_from_timestamps() {
        // 20 words over 10 s, then 10 words over 10 s -> 30 words / 20 s = 90 wpm.
        let twenty = ["word"; 20].join(" ");
        let ten = ["word"; 10].join(" ");
        let turns = vec![
            t("tutor", "Hello", 0),
            t("learner", &twenty, 2_000),
            t("tutor", "Nice", 12_000),
            t("learner", &ten, 14_000),
            t("tutor", "Okay", 24_000),
        ];
        assert_eq!(words_per_minute(&turns), Some(90));
    }

    #[test]
    fn words_per_minute_needs_enough_speech() {
        let turns = vec![t("learner", "hi there", 0), t("tutor", "hello", 2_000)];
        assert_eq!(words_per_minute(&turns), None);
    }

    #[test]
    fn word_count_ignores_punctuation_tokens() {
        assert_eq!(word_count("I go to office , yesterday ."), 5);
    }
}
