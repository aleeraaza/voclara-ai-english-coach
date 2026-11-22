//! Session grading by a text LLM, behind the `Grader` trait.
//!
//! The model returns only what needs judgment (criterion bands, evidence,
//! corrections, next focus) as JSON matching [`schema`]. Snapping, the overall
//! band, fillers and speed are computed in `domain::metrics`, not by the model.

pub mod fake;
pub mod gemini;
pub mod openai;

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::ToSchema;

use crate::{
    config::{Config, GraderKind},
    error::{AppError, AppResult},
};

/// What the grader is asked.
#[derive(Debug, Clone)]
pub struct GradeInput {
    /// Rendered grader prompt (packages/prompts/grader).
    pub system_prompt: String,
    /// Transcript plus instructions about the evidence available.
    pub user_message: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, ToSchema)]
pub struct Evidence {
    pub fluency: Vec<String>,
    pub lexical: Vec<String>,
    pub grammar: Vec<String>,
    pub pronunciation: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
pub struct Correction {
    /// What the learner said.
    pub said: String,
    /// A better way to say it.
    pub better: String,
    /// One-line reason in English.
    pub why_en: String,
    /// The same reason in the learner's explanation language.
    pub why_l1: String,
}

/// The model's judgment, before validation and snapping.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct GradeOutput {
    pub fluency: f64,
    pub lexical: f64,
    pub grammar: f64,
    /// `null` when no pronunciation evidence was given.
    pub pronunciation: Option<f64>,
    pub evidence: Evidence,
    pub corrections: Vec<Correction>,
    pub next_focus: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GradeUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[async_trait]
pub trait Grader: Send + Sync {
    /// "gemini" | "openai" | "fake"; stored on cost rows.
    fn name(&self) -> &'static str;
    fn model(&self) -> &str;
    async fn grade(&self, input: &GradeInput) -> AppResult<(GradeOutput, GradeUsage)>;
}

/// JSON Schema for [`GradeOutput`]. Written to satisfy both OpenAI strict mode
/// (every property required, no extra properties, nullable via a type array)
/// and Gemini's supported subset. Band ranges live in descriptions: bounds
/// enlarge the decoding grammar, and `domain::metrics` clamps anyway.
pub fn schema() -> Value {
    let band = |name: &str| json!({ "type": "number", "description": format!("{name} band, 0-9 in steps of 0.5") });
    let quotes = json!({
        "type": "array",
        "items": { "type": "string" },
        "description": "1-2 short exact quotes from LEARNER turns"
    });
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["fluency", "lexical", "grammar", "pronunciation", "evidence", "corrections", "next_focus"],
        "properties": {
            "fluency": band("Fluency and Coherence"),
            "lexical": band("Lexical Resource"),
            "grammar": band("Grammatical Range and Accuracy"),
            "pronunciation": {
                "type": ["number", "null"],
                "description": "Pronunciation band 0-9 in steps of 0.5, or null when no pronunciation scores were provided"
            },
            "evidence": {
                "type": "object",
                "additionalProperties": false,
                "required": ["fluency", "lexical", "grammar", "pronunciation"],
                "properties": {
                    "fluency": quotes,
                    "lexical": quotes,
                    "grammar": quotes,
                    "pronunciation": quotes
                }
            },
            "corrections": {
                "type": "array",
                "description": "Up to 8 of the most useful corrections, most important first",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["said", "better", "why_en", "why_l1"],
                    "properties": {
                        "said": { "type": "string", "description": "exact learner words" },
                        "better": { "type": "string", "description": "a natural correct version" },
                        "why_en": { "type": "string", "description": "one-line reason in English" },
                        "why_l1": { "type": "string", "description": "the same reason in the learner's language" }
                    }
                }
            },
            "next_focus": { "type": "string", "description": "one concrete thing to practise next" }
        }
    })
}

/// Parse and sanity-check the model's JSON. Anything malformed is an error so
/// the job retries rather than storing garbage.
pub fn parse_output(text: &str) -> AppResult<GradeOutput> {
    let out: GradeOutput = serde_json::from_str(text.trim())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("grader returned invalid JSON: {e}")))?;
    for (name, v) in [
        ("fluency", out.fluency),
        ("lexical", out.lexical),
        ("grammar", out.grammar),
    ] {
        if !v.is_finite() {
            return Err(AppError::Internal(anyhow::anyhow!(
                "grader returned a non-numeric {name} band"
            )));
        }
    }
    Ok(out)
}

pub fn from_config(cfg: &Config) -> anyhow::Result<Arc<dyn Grader>> {
    use anyhow::Context;
    let model = || cfg.grader.model.clone().context("GRADER_MODEL");
    Ok(match cfg.grader.provider {
        GraderKind::Gemini => Arc::new(gemini::GeminiGrader::new(
            cfg.voice.gemini_api_key.clone().context("GEMINI_API_KEY")?,
            model()?,
            cfg.voice.gemini_base_url.clone(),
        )?),
        GraderKind::OpenAi => Arc::new(openai::OpenAiGrader::new(
            cfg.voice.openai_api_key.clone().context("OPENAI_API_KEY")?,
            model()?,
            cfg.voice.openai_base_url.clone(),
        )?),
        GraderKind::Fake => Arc::new(fake::FakeGrader),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_strict_mode_shaped() {
        fn check(v: &Value) {
            if v["type"] == "object" {
                assert_eq!(v["additionalProperties"], false, "{v}");
                let props: Vec<&String> = v["properties"].as_object().unwrap().keys().collect();
                let required: Vec<&str> = v["required"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| r.as_str().unwrap())
                    .collect();
                assert_eq!(props.len(), required.len(), "every property required: {v}");
                for p in v["properties"].as_object().unwrap().values() {
                    check(p);
                }
            }
            if let Some(items) = v.get("items") {
                check(items);
            }
        }
        check(&schema());
    }

    #[test]
    fn parses_valid_output_and_rejects_junk() {
        let ok = json!({
            "fluency": 6.5, "lexical": 6.0, "grammar": 5.5, "pronunciation": null,
            "evidence": { "fluency": ["I go office"], "lexical": [], "grammar": [], "pronunciation": [] },
            "corrections": [{ "said": "I go office", "better": "I went to the office", "why_en": "past tense", "why_l1": "ماضی" }],
            "next_focus": "past tense"
        });
        let out = parse_output(&ok.to_string()).unwrap();
        assert_eq!(out.pronunciation, None);
        assert_eq!(out.corrections.len(), 1);
        assert!(parse_output("not json").is_err());
        assert!(parse_output(r#"{"fluency": 6}"#).is_err(), "missing fields");
    }
}
