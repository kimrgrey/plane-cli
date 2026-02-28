use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn plane_cmd() -> assert_cmd::Command {
    cargo_bin_cmd!("plane-cli").into()
}

#[test]
fn test_no_subcommand_shows_help() {
    plane_cmd()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[tokio::test]
async fn test_projects_list_json() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/test-ws/projects/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {"id": "p1", "name": "Alpha", "identifier": "ALP"}
            ]
        })))
        .mount(&mock_server)
        .await;

    plane_cmd()
        .args([
            "--api-key",
            "test",
            "--workspace",
            "test-ws",
            "--base-url",
            &mock_server.uri(),
            "--json",
            "projects",
            "list",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Alpha"));
}

#[tokio::test]
async fn test_projects_list_table() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/test-ws/projects/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {"id": "p1", "name": "Alpha", "identifier": "ALP"}
            ]
        })))
        .mount(&mock_server)
        .await;

    plane_cmd()
        .args([
            "--api-key",
            "test",
            "--workspace",
            "test-ws",
            "--base-url",
            &mock_server.uri(),
            "projects",
            "list",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Alpha"));
}

#[test]
fn test_missing_api_key_error() {
    plane_cmd()
        .env("PLANE_CLI_HOME", "/tmp/plane-cli-test-nonexistent")
        .env_remove("PLANE_CLI_API_KEY")
        .args(["--workspace", "ws", "projects", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("API key"));
}

#[test]
fn test_missing_workspace_error() {
    plane_cmd()
        .env("PLANE_CLI_HOME", "/tmp/plane-cli-test-nonexistent")
        .env_remove("PLANE_CLI_WORKSPACE")
        .args(["--api-key", "key", "projects", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("workspace"));
}

#[tokio::test]
async fn test_issues_create_json() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/workspaces/test-ws/projects/proj1/issues/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "iss-1",
            "sequence_id": 42,
            "name": "New Bug"
        })))
        .mount(&mock_server)
        .await;

    plane_cmd()
        .args([
            "--api-key",
            "test",
            "--workspace",
            "test-ws",
            "--base-url",
            &mock_server.uri(),
            "--json",
            "issues",
            "create",
            "--project",
            "proj1",
            "--title",
            "New Bug",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("iss-1"));
}
