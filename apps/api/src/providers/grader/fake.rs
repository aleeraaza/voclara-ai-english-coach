//! Demo grader for development: deterministic feedback derived from the
//! transcript, so the feedback screen can be built without an LLM key.
//! Refused in production. Scores it writes carry `grader_version` "fake".

use async_trait::async_trait;

use super::{Correction, Evidence, GradeInput, GradeOutput, GradeUsage, Grader};
use crate::error::AppResult;

pub struct FakeGrader;

#[async_trait]
impl Grader for FakeGrader {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn model(&self) -> &str {
        "fake-grader"
    }

    async fn grade(&self, input: &GradeInput) -> AppResult<(GradeOutput, GradeUsage)> {
        // The first learner line becomes the evidence quote and a sample correction.
        let first = input
            .user_message
            .lines()
            .find_map(|l| l.strip_prefix("LEARNER: "))
            .unwrap_or("")
            .to_owned();
        let quote = first.chars().take(80).collect::<String>();
        let corrections = if quote.is_empty() {
            vec![]
        } else {
            vec![Correction {
                said: quote.clone(),
                better: quote.clone(),
                why_en: "Demo feedback: connect a grader model for real corrections.".into(),
                why_l1: "Demo feedback.".into(),
            }]
        };
        Ok((
            GradeOutput {
                fluency: 6.0,
                lexical: 6.0,
                grammar: 5.5,
                pronunciation: None,
                evidence: Evidence {
                    fluency: vec![quote.clone()],
                    lexical: vec![quote.clone()],
                    grammar: vec![quote],
                    pronunciation: vec![],
                },
                corrections,
                next_focus: "Demo feedback: set GRADER_PROVIDER to a real model.".into(),
            },
            GradeUsage::default(),
        ))
    }
}
