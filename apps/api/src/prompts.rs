//! Product prompts, embedded at compile time from `packages/prompts`.
//!
//! Each file starts with front matter holding a `version` (e.g. `tutor-v1`).
//! That version is stored on every session so a change in tutor behaviour or
//! grading can always be traced back to the prompt that produced it.

use anyhow::{Context, Result};

const TUTOR_V1: &str = include_str!("../../../packages/prompts/tutor/v1.md");
// examiner-v2: scripted mock with signal sentences the app listens for.
const EXAMINER_V2: &str = include_str!("../../../packages/prompts/examiner/v2.md");
const INTERVIEWER_V1: &str = include_str!("../../../packages/prompts/interviewer/v1.md");
const GRADER_V1: &str = include_str!("../../../packages/prompts/grader/v1.md");

#[derive(Debug, Clone)]
pub struct Prompt {
    pub version: String,
    template: String,
}

impl Prompt {
    fn parse(source: &str) -> Result<Self> {
        let source = source.replace("\r\n", "\n");
        let rest = source
            .strip_prefix("---\n")
            .context("prompt must start with '---' front matter")?;
        let (front, body) = rest
            .split_once("\n---\n")
            .context("prompt front matter is not closed with '---'")?;
        let version = front
            .lines()
            .find_map(|l| l.strip_prefix("version:"))
            .map(|v| v.trim().to_owned())
            .filter(|v| !v.is_empty())
            .context("prompt front matter has no version")?;
        Ok(Self {
            version,
            template: body.trim().to_owned(),
        })
    }

    /// Fill `{name}` placeholders. Values are already sanitised by the caller.
    pub fn render(&self, vars: &[(&str, &str)]) -> String {
        let mut out = self.template.clone();
        for (name, value) in vars {
            out = out.replace(&format!("{{{name}}}"), value);
        }
        out
    }
}

#[derive(Debug, Clone)]
pub struct Prompts {
    pub tutor: Prompt,
    pub examiner: Prompt,
    pub interviewer: Prompt,
    pub grader: Prompt,
}

impl Prompts {
    pub fn load() -> Result<Self> {
        Ok(Self {
            tutor: Prompt::parse(TUTOR_V1).context("tutor prompt")?,
            examiner: Prompt::parse(EXAMINER_V2).context("examiner prompt")?,
            interviewer: Prompt::parse(INTERVIEWER_V1).context("interviewer prompt")?,
            grader: Prompt::parse(GRADER_V1).context("grader prompt")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_prompts_parse_with_versions() {
        let p = Prompts::load().unwrap();
        assert_eq!(p.tutor.version, "tutor-v1");
        assert_eq!(p.examiner.version, "examiner-v2");
        assert_eq!(p.interviewer.version, "interviewer-v1");
        assert_eq!(p.grader.version, "grader-v1");
        let g = p.grader.render(&[("l1", "Urdu")]);
        assert!(
            g.contains("in English and in Urdu") && !g.contains('{'),
            "{g}"
        );
    }

    #[test]
    fn render_fills_every_placeholder() {
        let p = Prompts::load().unwrap();
        let tutor = p
            .tutor
            .render(&[("level", "b1"), ("goal", "ielts"), ("topic", "travel")]);
        assert!(tutor.contains("Level: b1. Goal: ielts. Topic: travel."));
        assert!(!tutor.contains("{level}") && !tutor.contains("{topic}"));

        let interviewer = p
            .interviewer
            .render(&[("role", "nursing"), ("level", "b2")]);
        assert!(interviewer.contains("nursing job"));
        assert!(!interviewer.contains('{'));

        let examiner = p.examiner.render(&[
            ("level", "b1"),
            ("topic", "work"),
            ("part1", "- Q1"),
            ("cue_card", "Describe a job."),
            ("rounding_off", "Q?"),
            ("part3", "- Q3"),
        ]);
        assert!(!examiner.contains('{'), "{examiner}");
        // The app listens for these exact sentences to switch screens.
        for signal in [
            "Now, Part 2.",
            "You have one minute to prepare.",
            "Now, Part 3.",
            "end of the speaking test",
        ] {
            assert!(examiner.contains(signal), "missing signal {signal}");
        }
    }

    #[test]
    fn bad_front_matter_is_an_error() {
        assert!(Prompt::parse("no front matter").is_err());
        assert!(
            Prompt::parse("---\nid: x\n---\nbody").is_err(),
            "missing version"
        );
        assert!(Prompt::parse("---\nversion: v1\nbody").is_err(), "unclosed");
    }
}
