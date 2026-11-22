//! The `grade_session` job: transcript -> grader LLM -> validated scores.

use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    config::GraderConfig,
    domain::metrics::{self, Turn},
    error::{AppError, AppResult},
    prompts::Prompts,
    providers::grader::{Correction, Evidence, GradeInput, Grader},
};

/// Fewer learner words than this cannot be graded fairly.
pub const MIN_LEARNER_WORDS: usize = 25;
/// Keep the grading request bounded (about 15 minutes of speech).
const MAX_TRANSCRIPT_CHARS: usize = 40_000;
const MAX_CORRECTIONS: usize = 8;
const MAX_FIELD_CHARS: usize = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradeResult {
    Ready,
    /// Not enough learner speech; feedback says so instead of guessing.
    Skipped,
}

#[derive(sqlx::FromRow)]
struct SessionForGrading {
    transcript: Option<Value>,
    l1: String,
}

fn clip(s: &str, max: usize) -> String {
    s.trim().chars().take(max).collect()
}

/// The message the grader sees. Tutor turns give context; only learner turns
/// are graded (the prompt says so, and they are labelled).
pub fn grader_message(turns: &[Turn], pronunciation: Option<&str>) -> String {
    let mut body = String::from(
        "Transcript of a speaking session. Grade ONLY the LEARNER turns; TUTOR turns are context.\n\n",
    );
    for t in turns {
        let who = if t.role == "learner" {
            "LEARNER"
        } else {
            "TUTOR"
        };
        let line = format!("{who}: {}\n", t.text.replace('\n', " "));
        if body.len() + line.len() > MAX_TRANSCRIPT_CHARS {
            body.push_str("[transcript truncated]\n");
            break;
        }
        body.push_str(&line);
    }
    match pronunciation {
        Some(p) => {
            body.push_str("\nPronunciation scores (use as evidence for the pronunciation band):\n");
            body.push_str(p);
        }
        None => body.push_str(
            "\nPronunciation: no pronunciation scores were measured for this session. \
             Set \"pronunciation\" to null and leave its evidence empty.",
        ),
    }
    body
}

fn clean_quotes(q: &[String]) -> Vec<String> {
    q.iter()
        .map(|s| clip(s, 200))
        .filter(|s| !s.is_empty())
        .take(2)
        .collect()
}

/// Grade one session. Idempotent: re-running overwrites the score.
pub async fn grade_session(
    db: &PgPool,
    grader: &dyn Grader,
    prompts: &Prompts,
    prices: &GraderConfig,
    session_id: Uuid,
) -> AppResult<GradeResult> {
    let row: SessionForGrading = sqlx::query_as(
        "SELECT s.transcript, u.l1 FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.id = $1",
    )
    .bind(session_id)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound)?;

    let turns: Vec<Turn> = row
        .transcript
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    if metrics::learner_words(&turns) < MIN_LEARNER_WORDS {
        sqlx::query("UPDATE sessions SET grading_status = 'skipped' WHERE id = $1")
            .bind(session_id)
            .execute(db)
            .await?;
        return Ok(GradeResult::Skipped);
    }

    // Pronunciation scoring (Azure) is step 6b; until then there is none.
    let pronunciation_evidence: Option<&str> = None;
    let input = GradeInput {
        system_prompt: prompts
            .grader
            .render(&[("l1", metrics::language_name(&row.l1))]),
        user_message: grader_message(&turns, pronunciation_evidence),
    };
    let (out, usage) = grader.grade(&input).await?;

    // Validate and snap; the model never decides the overall band.
    let fluency = metrics::snap_band(out.fluency);
    let lexical = metrics::snap_band(out.lexical);
    let grammar = metrics::snap_band(out.grammar);
    let pronunciation = pronunciation_evidence
        .and(out.pronunciation)
        .map(metrics::snap_band);
    let overall = metrics::overall_band(fluency, lexical, grammar, pronunciation);

    let evidence = Evidence {
        fluency: clean_quotes(&out.evidence.fluency),
        lexical: clean_quotes(&out.evidence.lexical),
        grammar: clean_quotes(&out.evidence.grammar),
        pronunciation: if pronunciation.is_some() {
            clean_quotes(&out.evidence.pronunciation)
        } else {
            vec![]
        },
    };
    let corrections: Vec<Correction> = out
        .corrections
        .iter()
        .filter(|c| !c.said.trim().is_empty() && !c.better.trim().is_empty())
        .take(MAX_CORRECTIONS)
        .map(|c| Correction {
            said: clip(&c.said, MAX_FIELD_CHARS),
            better: clip(&c.better, MAX_FIELD_CHARS),
            why_en: clip(&c.why_en, MAX_FIELD_CHARS),
            why_l1: clip(&c.why_l1, MAX_FIELD_CHARS),
        })
        .collect();
    let fillers = metrics::fillers(&turns);
    let wpm = metrics::words_per_minute(&turns);
    let usd = usage.input_tokens as f64 / 1e6 * prices.usd_per_1m_input
        + usage.output_tokens as f64 / 1e6 * prices.usd_per_1m_output;

    let mut tx = db.begin().await?;
    sqlx::query(
        "INSERT INTO scores (session_id, fluency, lexical, grammar, pronunciation, overall,
                             evidence, corrections, fillers, wpm, next_focus,
                             grader_version, grader_model, l1)
         VALUES ($1, $2::numeric, $3::numeric, $4::numeric, $5::numeric, $6::numeric,
                 $7, $8, $9, $10, $11, $12, $13, $14)
         ON CONFLICT (session_id) DO UPDATE SET
             fluency = EXCLUDED.fluency, lexical = EXCLUDED.lexical, grammar = EXCLUDED.grammar,
             pronunciation = EXCLUDED.pronunciation, overall = EXCLUDED.overall,
             evidence = EXCLUDED.evidence, corrections = EXCLUDED.corrections,
             fillers = EXCLUDED.fillers, wpm = EXCLUDED.wpm, next_focus = EXCLUDED.next_focus,
             grader_version = EXCLUDED.grader_version, grader_model = EXCLUDED.grader_model,
             l1 = EXCLUDED.l1, created_at = now()",
    )
    .bind(session_id)
    .bind(fluency)
    .bind(lexical)
    .bind(grammar)
    .bind(pronunciation)
    .bind(overall)
    .bind(to_json(&evidence)?)
    .bind(to_json(&corrections)?)
    .bind(to_json(&fillers)?)
    .bind(wpm)
    .bind(clip(&out.next_focus, MAX_FIELD_CHARS))
    .bind(grader_version(grader, &prompts.grader.version))
    .bind(grader.model())
    .bind(&row.l1)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO session_costs (session_id, provider, input_secs, output_secs, tokens, usd)
         VALUES ($1, $2, 0, 0, $3, $4)",
    )
    .bind(session_id)
    .bind(format!("grader:{}", grader.name()))
    .bind(json!({ "kind": "grading", "model": grader.model(),
                   "input": usage.input_tokens, "output": usage.output_tokens }))
    .bind(usd)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE sessions SET grading_status = 'ready' WHERE id = $1")
        .bind(session_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(GradeResult::Ready)
}

/// The version stored on the score: the prompt version, or "fake" so demo
/// feedback can never be mistaken for a real grade.
fn grader_version(grader: &dyn Grader, prompt_version: &str) -> String {
    if grader.name() == "fake" {
        "fake".to_owned()
    } else {
        prompt_version.to_owned()
    }
}

/// The job gave up after all retries.
pub async fn mark_failed(db: &PgPool, session_id: Uuid) -> AppResult<()> {
    sqlx::query(
        "UPDATE sessions SET grading_status = 'failed' WHERE id = $1 AND grading_status <> 'ready'",
    )
    .bind(session_id)
    .execute(db)
    .await?;
    Ok(())
}

fn to_json<T: serde::Serialize>(v: &T) -> AppResult<Value> {
    serde_json::to_value(v).map_err(|e| AppError::Internal(e.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(role: &str, text: &str) -> Turn {
        Turn {
            role: role.into(),
            text: text.into(),
            t_ms: 0,
        }
    }

    #[test]
    fn message_labels_speakers_and_states_missing_pronunciation() {
        let m = grader_message(
            &[t("tutor", "Hi"), t("learner", "I go office\nyesterday")],
            None,
        );
        assert!(m.contains("TUTOR: Hi\n"));
        assert!(m.contains("LEARNER: I go office yesterday\n"), "{m}");
        assert!(m.contains("Set \"pronunciation\" to null"));
    }

    #[test]
    fn message_is_truncated_when_huge() {
        let long = "word ".repeat(2_000);
        let turns: Vec<Turn> = (0..20).map(|_| t("learner", &long)).collect();
        let m = grader_message(&turns, None);
        assert!(m.len() < MAX_TRANSCRIPT_CHARS + 500);
        assert!(m.contains("[transcript truncated]"));
    }
}
