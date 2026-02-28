use anyhow::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::header::{HeaderMap, HeaderValue};
use std::time::Duration;

use crate::settings::Settings;

pub struct Client {
    http: reqwest::Client,
    base_url: String,
    show_spinner: bool,
}

impl Client {
    pub fn new(settings: &Settings, json_mode: bool) -> Result<Self> {
        let api_key = settings
            .api_key
            .as_deref()
            .context("API key is required — set it via --api-key, PLANE_CLI_API_KEY, or config file")?;

        let mut headers = HeaderMap::new();
        headers.insert(
            "X-API-Key",
            HeaderValue::from_str(api_key).context("invalid API key value")?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(settings.timeout))
            .build()
            .context("failed to build HTTP client")?;

        let base_url = format!("{}/api/v1", settings.base_url.trim_end_matches('/'));

        Ok(Self {
            http,
            base_url,
            show_spinner: !json_mode,
        })
    }

    fn spinner(&self, message: &str) -> Option<ProgressBar> {
        if !self.show_spinner {
            return None;
        }
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["   ", ".  ", ".. ", "...", " ..", "  .", "   "])
                .template("{spinner} {msg}")
                .unwrap(),
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(Duration::from_millis(120));
        Some(pb)
    }

    pub async fn get(&self, path: &str) -> Result<serde_json::Value> {
        self.get_with_params(path, &[]).await
    }

    pub async fn get_with_params(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<serde_json::Value> {
        let spinner = self.spinner("Fetching...");
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let response = self
            .http
            .get(&url)
            .query(params)
            .send()
            .await
            .context("GET request failed")?;
        let result = handle_response(response).await;
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }
        result
    }

    pub async fn post(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let spinner = self.spinner("Sending...");
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let response = self
            .http
            .post(&url)
            .json(body)
            .send()
            .await
            .context("POST request failed")?;
        let result = handle_response(response).await;
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }
        result
    }

    pub async fn patch(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let spinner = self.spinner("Updating...");
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let response = self
            .http
            .patch(&url)
            .json(body)
            .send()
            .await
            .context("PATCH request failed")?;
        let result = handle_response(response).await;
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }
        result
    }

    pub async fn delete(&self, path: &str) -> Result<serde_json::Value> {
        let spinner = self.spinner("Deleting...");
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let response = self
            .http
            .delete(&url)
            .send()
            .await
            .context("DELETE request failed")?;
        let result = handle_response(response).await;
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }
        result
    }
}

async fn handle_response(response: reqwest::Response) -> Result<serde_json::Value> {
    let status = response.status();

    if status.is_success() {
        let body = response
            .json::<serde_json::Value>()
            .await
            .context("failed to parse response JSON")?;
        return Ok(body);
    }

    let body_text = response.text().await.unwrap_or_default();

    match status.as_u16() {
        401 => bail!("unauthorized — check your API key"),
        404 => bail!("not found: {body_text}"),
        429 => bail!("rate limited — try again later"),
        500..=599 => bail!("server error ({status}): {body_text}"),
        _ => bail!("request failed ({status}): {body_text}"),
    }
}
