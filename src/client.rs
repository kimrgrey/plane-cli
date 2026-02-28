use anyhow::{bail, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use std::time::Duration;

use crate::settings::Settings;

pub struct Client {
    http: reqwest::Client,
    base_url: String,
}

impl Client {
    pub fn new(settings: &Settings) -> Result<Self> {
        let api_key = settings
            .api_key
            .as_deref()
            .context("API key is required — set it via --api-key, PLANE_API_KEY, or config file")?;

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

        Ok(Self { http, base_url })
    }

    pub async fn get(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let response = self.http.get(&url).send().await.context("GET request failed")?;
        handle_response(response).await
    }

    pub async fn post(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let response = self
            .http
            .post(&url)
            .json(body)
            .send()
            .await
            .context("POST request failed")?;
        handle_response(response).await
    }

    pub async fn patch(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let response = self
            .http
            .patch(&url)
            .json(body)
            .send()
            .await
            .context("PATCH request failed")?;
        handle_response(response).await
    }

    pub async fn delete(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let response = self
            .http
            .delete(&url)
            .send()
            .await
            .context("DELETE request failed")?;
        handle_response(response).await
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
