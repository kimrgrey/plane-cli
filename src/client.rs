use anyhow::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::header::{HeaderMap, HeaderValue};
use std::time::Duration;

use crate::settings::Settings;

#[derive(Debug)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use wiremock::matchers::{header, method, path, query_param, body_json};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_settings(base_url: &str) -> Settings {
        Settings {
            api_key: Some("test-key".to_string()),
            base_url: base_url.to_string(),
            workspace: Some("test-ws".to_string()),
            timeout: 5,
        }
    }

    // ── Construction ──

    #[test]
    fn test_new_requires_api_key() {
        let settings = Settings {
            api_key: None,
            base_url: "https://example.com".to_string(),
            workspace: None,
            timeout: 30,
        };
        let err = Client::new(&settings, true).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("API key is required"), "got: {msg}");
    }

    #[test]
    fn test_new_succeeds_with_api_key() {
        let settings = test_settings("https://example.com");
        assert!(Client::new(&settings, true).is_ok());
    }

    // ── HTTP methods ──

    #[tokio::test]
    async fn test_get_sends_request_with_api_key_header() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/test-path"))
            .and(header("X-API-Key", "test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = Client::new(&test_settings(&mock_server.uri()), true).unwrap();
        let result = client.get("test-path").await.unwrap();
        assert_eq!(result["ok"], true);
    }

    #[tokio::test]
    async fn test_get_with_params_includes_query_string() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/items"))
            .and(query_param("per_page", "10"))
            .and(query_param("state", "active"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"results": []})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = Client::new(&test_settings(&mock_server.uri()), true).unwrap();
        let result = client
            .get_with_params("items", &[("per_page", "10"), ("state", "active")])
            .await
            .unwrap();
        assert_eq!(result["results"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn test_post_sends_json_body() {
        let mock_server = MockServer::start().await;
        let body = serde_json::json!({"name": "Test Issue"});
        Mock::given(method("POST"))
            .and(path("/api/v1/issues"))
            .and(body_json(&body))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "123"})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = Client::new(&test_settings(&mock_server.uri()), true).unwrap();
        let result = client.post("issues", &body).await.unwrap();
        assert_eq!(result["id"], "123");
    }

    #[tokio::test]
    async fn test_patch_sends_json_body() {
        let mock_server = MockServer::start().await;
        let body = serde_json::json!({"name": "Updated"});
        Mock::given(method("PATCH"))
            .and(path("/api/v1/issues/123"))
            .and(body_json(&body))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "123", "name": "Updated"})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = Client::new(&test_settings(&mock_server.uri()), true).unwrap();
        let result = client.patch("issues/123", &body).await.unwrap();
        assert_eq!(result["name"], "Updated");
    }

    #[tokio::test]
    async fn test_delete_sends_request() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/issues/123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = Client::new(&test_settings(&mock_server.uri()), true).unwrap();
        let result = client.delete("issues/123").await.unwrap();
        assert_eq!(result, serde_json::json!({}));
    }

    // ── Error handling ──

    #[tokio::test]
    async fn test_error_401() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/test"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = Client::new(&test_settings(&mock_server.uri()), true).unwrap();
        let err = client.get("test").await.unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("unauthorized"), "got: {msg}");
    }

    #[tokio::test]
    async fn test_error_404() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/test"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let client = Client::new(&test_settings(&mock_server.uri()), true).unwrap();
        let err = client.get("test").await.unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("not found"), "got: {msg}");
    }

    #[tokio::test]
    async fn test_error_429() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/test"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let client = Client::new(&test_settings(&mock_server.uri()), true).unwrap();
        let err = client.get("test").await.unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("rate limited"), "got: {msg}");
    }

    #[tokio::test]
    async fn test_error_500() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/test"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal"))
            .mount(&mock_server)
            .await;

        let client = Client::new(&test_settings(&mock_server.uri()), true).unwrap();
        let err = client.get("test").await.unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("server error"), "got: {msg}");
    }
}
