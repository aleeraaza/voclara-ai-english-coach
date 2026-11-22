//! Golden test: the REAL grader against target bands in packages/content/golden.
//! Costs tokens, so it only runs with GRADER_GOLDEN=1 and a real grader in
//! the environment (GRADER_PROVIDER, GRADER_MODEL and its API key, e.g. from .env).
//!
//! Fails if any overall band is more than 0.5 from the target, or if fewer
//! than 80% of criterion bands are within 0.5 (the guide's agreement target).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use serde::Deserialize;
use voclara_api::{
    config::{Config, GraderKind},
    domain::{
        grading::grader_message,
        metrics::{self, Turn},
    },
    prompts::Prompts,
    providers::grader::{self, GradeInput},
};

#[derive(Deserialize)]
struct Expected {
    fluency: f64,
    lexical: f64,
    grammar: f64,
    overall: f64,
}

#[derive(Deserialize)]
struct Golden {
    id: String,
    l1: String,
    expected: Expected,
    turns: Vec<Turn>,
}

fn golden_files() -> Vec<std::path::PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/content/golden");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    files
}

/// Always runs (free): the golden files parse, are long enough to grade, and
/// each expected overall follows from its criteria by our rounding rule.
#[test]
fn golden_files_are_well_formed() {
    let files = golden_files();
    assert!(files.len() >= 10);
    for path in files {
        let g: Golden = serde_json::from_str(&std::fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{path:?}: {e}"));
        let e = &g.expected;
        assert_eq!(
            metrics::overall_band(e.fluency, e.lexical, e.grammar, None),
            e.overall,
            "{}: expected overall does not follow from its criteria",
            g.id
        );
        assert!(
            metrics::learner_words(&g.turns) >= voclara_api::domain::grading::MIN_LEARNER_WORDS,
            "{}: too short to be graded",
            g.id
        );
    }
}

#[tokio::test]
async fn real_grader_agrees_with_golden_bands() {
    if std::env::var("GRADER_GOLDEN").as_deref() != Ok("1") {
        eprintln!("GRADER_GOLDEN!=1; skipping the paid golden grading test");
        return;
    }
    let _ = dotenvy::from_path("../../.env");
    let config = Config::from_env().expect("config from env");
    assert_ne!(
        config.grader.provider,
        GraderKind::Fake,
        "set a real GRADER_PROVIDER"
    );
    let grader = grader::from_config(&config).unwrap();
    let prompts = Prompts::load().unwrap();

    let files = golden_files();
    assert!(files.len() >= 10, "expected at least 10 golden transcripts");

    let (mut within, mut total, mut overall_misses) = (0, 0, vec![]);
    for path in files {
        let g: Golden = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let input = GradeInput {
            system_prompt: prompts
                .grader
                .render(&[("l1", metrics::language_name(&g.l1))]),
            user_message: grader_message(&g.turns, None),
        };
        let (out, _) = grader.grade(&input).await.expect("grading failed");
        let (f, l, gr) = (
            metrics::snap_band(out.fluency),
            metrics::snap_band(out.lexical),
            metrics::snap_band(out.grammar),
        );
        let overall = metrics::overall_band(f, l, gr, None);
        for (got, want) in [
            (f, g.expected.fluency),
            (l, g.expected.lexical),
            (gr, g.expected.grammar),
        ] {
            total += 1;
            if (got - want).abs() <= 0.5 {
                within += 1;
            }
        }
        let diff = overall - g.expected.overall;
        eprintln!(
            "{:<24} F {f:.1}/{:.1}  L {l:.1}/{:.1}  G {gr:.1}/{:.1}  overall {overall:.1}/{:.1}  ({diff:+.1})",
            g.id, g.expected.fluency, g.expected.lexical, g.expected.grammar, g.expected.overall
        );
        if diff.abs() > 0.5 {
            overall_misses.push(format!(
                "{}: {overall:.1} vs {:.1}",
                g.id, g.expected.overall
            ));
        }
    }
    let agreement = within as f64 / total as f64;
    eprintln!(
        "criterion bands within 0.5: {within}/{total} ({:.0}%)",
        agreement * 100.0
    );
    assert!(
        overall_misses.is_empty(),
        "overall bands off by more than 0.5: {overall_misses:?}"
    );
    assert!(
        agreement >= 0.8,
        "only {:.0}% of criterion bands within 0.5",
        agreement * 100.0
    );
}
