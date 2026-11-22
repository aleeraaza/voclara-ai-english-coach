//! OpenAI Chat Completions with a strict JSON schema (structured outputs).
//! Docs: https://platform.openai.com/docs/guides/structured-outputs

use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::{GradeInput, GradeOutput, GradeUsage, Grader, parse_output, schema};
use crate::{
    error::{AppError, AppResult},
    providers::voice::upstream_error,
};

const NAME: &str = "openai";

pub struct OpenAiGrader {
    http: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAiGrader {
    pub fn new(api_key: String, model: String, base_url: String) -> anyhow::Result<Self> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(90))
                .build()?,
            api_key,
            model,
            base_url: base_url.trim_end_matches('/').to_owned(),
        })
    }

    pub fn request(&self, input: &GradeInput) -> Value {
        json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": input.system_prompt },
                { "role": "user", "content": input.user_message }
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": { "name": "speaking_grade", "strict": true, "schema": schema() }
            }
        })
    }
}

#[async_trait]
impl Grader for OpenAiGrader {
    fn name(&self) -> &'static str {
        NAME
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn grade(&self, input: &GradeInput) -> AppResult<(GradeOutput, GradeUsage)> {
        let resp: Value = self
            .http
            .post(format!("{}/v1/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&self.request(input))
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| upstream_error(NAME, "OpenAI grading request", e))?
            .json()
            .await
            .map_err(|e| upstream_error(NAME, "parsing OpenAI grading response", e))?;

        let message = &resp["choices"][0]["message"];
        if let Some(refusal) = message["refusal"].as_str() {
            return Err(AppError::Internal(anyhow::anyhow!(
                "OpenAI refused to grade: {refusal}"
            )));
        }
        let text = message["content"]
            .as_str()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("OpenAI returned no grade")))?;
        let usage = GradeUsage {
            input_tokens: resp["usage"]["prompt_tokens"].as_i64().unwrap_or(0),
            output_tokens: resp["usage"]["completion_tokens"].as_i64().unwrap_or(0),
        };
        Ok((parse_output(text)?, usage))
    }
}
