//! Gemini `generateContent` with a JSON schema (structured output).
//! Docs: https://ai.google.dev/api/generate-content and
//! https://ai.google.dev/gemini-api/docs/structured-output

use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::{GradeInput, GradeOutput, GradeUsage, Grader, parse_output, schema};
use crate::{
    error::{AppError, AppResult},
    providers::voice::upstream_error,
};

const NAME: &str = "gemini";

pub struct GeminiGrader {
    http: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl GeminiGrader {
    pub fn new(api_key: String, model: String, base_url: String) -> anyhow::Result<Self> {
        Ok(Self {
            // Grading a 15-minute transcript can take a while.
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
            "systemInstruction": { "parts": [{ "text": input.system_prompt }] },
            "contents": [{ "role": "user", "parts": [{ "text": input.user_message }] }],
            "generationConfig": {
                "responseMimeType": "application/json",
                "responseJsonSchema": schema()
            }
        })
    }
}

#[async_trait]
impl Grader for GeminiGrader {
    fn name(&self) -> &'static str {
        NAME
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn grade(&self, input: &GradeInput) -> AppResult<(GradeOutput, GradeUsage)> {
        let model = self.model.strip_prefix("models/").unwrap_or(&self.model);
        let resp: Value = self
            .http
            .post(format!(
                "{}/v1beta/models/{model}:generateContent",
                self.base_url
            ))
            .header("x-goog-api-key", &self.api_key)
            .json(&self.request(input))
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| upstream_error(NAME, "Gemini grading request", e))?
            .json()
            .await
            .map_err(|e| upstream_error(NAME, "parsing Gemini grading response", e))?;

        let text = resp["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| {
                let reason = resp["candidates"][0]["finishReason"]
                    .as_str()
                    .unwrap_or("no candidate");
                AppError::Internal(anyhow::anyhow!("Gemini returned no grade ({reason})"))
            })?;
        let usage = GradeUsage {
            input_tokens: resp["usageMetadata"]["promptTokenCount"]
                .as_i64()
                .unwrap_or(0),
            output_tokens: resp["usageMetadata"]["candidatesTokenCount"]
                .as_i64()
                .unwrap_or(0),
        };
        Ok((parse_output(text)?, usage))
    }
}
