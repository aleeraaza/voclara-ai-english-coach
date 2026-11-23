//! Practice content, embedded at compile time from `packages/content`.
//!
//! The IELTS-style cue card bank drives the mock: each card carries its own
//! Part 1 questions, the Part 2 task, a rounding-off question and Part 3
//! discussion questions, so all three parts share one theme.

use std::collections::HashSet;

use anyhow::{Context, Result, ensure};
use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

const CUE_CARDS: &str = include_str!("../../../packages/content/ielts/cue-cards.json");

/// Fewer cards than this and regular learners see repeats too soon.
pub const MIN_CUE_CARDS: usize = 30;

#[derive(Debug, Clone, Deserialize)]
pub struct CueCard {
    pub id: String,
    pub topic: String,
    pub part1: Vec<String>,
    pub prompt: String,
    pub bullets: Vec<String>,
    pub rounding_off: Vec<String>,
    pub part3: Vec<String>,
}

/// What the app shows for Part 2. Part 1 and Part 3 questions stay on the
/// server so the learner cannot read them in advance.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CueCardView {
    pub id: String,
    /// Theme of the whole test, e.g. "Travel".
    pub topic: String,
    /// The Part 2 task, e.g. "Describe a memorable journey you have made."
    pub prompt: String,
    /// "You should say:" points; the last one usually starts with "and explain".
    pub bullets: Vec<String>,
}

impl CueCard {
    pub fn view(&self) -> CueCardView {
        CueCardView {
            id: self.id.clone(),
            topic: self.topic.clone(),
            prompt: self.prompt.clone(),
            bullets: self.bullets.clone(),
        }
    }

    /// Values for the examiner prompt's placeholders (besides `level`).
    pub fn prompt_vars(&self) -> CardVars {
        let list = |qs: &[String]| {
            qs.iter()
                .map(|q| format!("  - {q}"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let bullets = self
            .bullets
            .iter()
            .map(|b| format!("    - {b}"))
            .collect::<Vec<_>>()
            .join("\n");
        CardVars {
            part1: list(&self.part1),
            cue_card: format!("  {}\n  You should say:\n{bullets}", self.prompt),
            rounding_off: self.rounding_off.join(" "),
            part3: list(&self.part3),
        }
    }
}

pub struct CardVars {
    pub part1: String,
    pub cue_card: String,
    pub rounding_off: String,
    pub part3: String,
}

#[derive(Debug, Clone)]
pub struct Content {
    cue_cards: Vec<CueCard>,
}

impl Content {
    pub fn load() -> Result<Self> {
        let cue_cards: Vec<CueCard> =
            serde_json::from_str(CUE_CARDS).context("parsing ielts/cue-cards.json")?;
        validate(&cue_cards)?;
        Ok(Self { cue_cards })
    }

    pub fn cue_card(&self, id: &str) -> Option<&CueCard> {
        self.cue_cards.iter().find(|c| c.id == id)
    }

    pub fn cue_cards(&self) -> &[CueCard] {
        &self.cue_cards
    }

    /// A random card, avoiding `recent` ids while any other card is left.
    /// `None` only if the bank is empty, which `load` rules out.
    pub fn pick_cue_card(&self, recent: &[String]) -> Option<&CueCard> {
        let fresh: Vec<&CueCard> = self
            .cue_cards
            .iter()
            .filter(|c| !recent.contains(&c.id))
            .collect();
        let mut rng = rand::rng();
        fresh
            .choose(&mut rng)
            .copied()
            .or_else(|| self.cue_cards.choose(&mut rng))
    }
}

fn validate(cards: &[CueCard]) -> Result<()> {
    ensure!(
        cards.len() >= MIN_CUE_CARDS,
        "need at least {MIN_CUE_CARDS} cue cards, found {}",
        cards.len()
    );
    let mut ids = HashSet::new();
    for c in cards {
        ensure!(ids.insert(c.id.as_str()), "duplicate cue card id {}", c.id);
        ensure!(
            !c.id.is_empty()
                && c.id
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'),
            "cue card id {:?} must be lowercase-kebab",
            c.id
        );
        ensure!(
            !c.topic.trim().is_empty() && !c.prompt.trim().is_empty(),
            "{}: empty topic or prompt",
            c.id
        );
        ensure!((3..=5).contains(&c.bullets.len()), "{}: 3-5 bullets", c.id);
        ensure!(
            (4..=5).contains(&c.part1.len()),
            "{}: 4-5 Part 1 questions",
            c.id
        );
        ensure!(
            (1..=2).contains(&c.rounding_off.len()),
            "{}: 1-2 rounding-off questions",
            c.id
        );
        ensure!(
            (3..=5).contains(&c.part3.len()),
            "{}: 3-5 Part 3 questions",
            c.id
        );
        // Text goes into the system prompt: keep it on one line, no braces
        // (which would look like unfilled placeholders).
        let all = c
            .part1
            .iter()
            .chain(&c.bullets)
            .chain(&c.rounding_off)
            .chain(&c.part3)
            .chain([&c.prompt, &c.topic]);
        for text in all {
            ensure!(
                !text.trim().is_empty() && !text.contains(['\n', '{', '}']),
                "{}: bad text {text:?}",
                c.id
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_bank_is_valid_and_large_enough() {
        let content = Content::load().unwrap();
        assert!(content.cue_cards().len() >= MIN_CUE_CARDS);
        assert!(content.cue_card("describe-a-teacher").is_some());
    }

    #[test]
    fn pick_avoids_recent_cards_until_none_are_left() {
        let content = Content::load().unwrap();
        let all: Vec<String> = content.cue_cards().iter().map(|c| c.id.clone()).collect();
        let keep = &all[0];
        let recent: Vec<String> = all[1..].to_vec();
        for _ in 0..20 {
            assert_eq!(&content.pick_cue_card(&recent).unwrap().id, keep);
        }
        // All used recently: still returns a card.
        assert!(all.contains(&content.pick_cue_card(&all).unwrap().id));
    }

    #[test]
    fn prompt_vars_list_every_question() {
        let content = Content::load().unwrap();
        let card = content.cue_card("describe-a-journey").unwrap();
        let v = card.prompt_vars();
        assert_eq!(v.part1.lines().count(), card.part1.len());
        assert!(v.cue_card.contains("Describe a memorable journey"));
        assert!(v.cue_card.contains("You should say:"));
        assert_eq!(v.part3.lines().count(), card.part3.len());
    }

    #[test]
    fn validation_rejects_bad_cards() {
        let mut cards = Content::load().unwrap().cue_cards;
        cards[1].id = cards[0].id.clone();
        assert!(validate(&cards).is_err(), "duplicate id");
        let mut cards = Content::load().unwrap().cue_cards;
        cards[0].part3[0] = "Fill {this}".into();
        assert!(validate(&cards).is_err(), "braces");
        let cards = Content::load().unwrap().cue_cards;
        assert!(validate(&cards[..5]).is_err(), "too few cards");
    }
}
