use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn plane_cmd() -> assert_cmd::Command {
    cargo_bin_cmd!("plane-cli")
}

/// Helper: returns a Command pre-configured with api-key, workspace, and base-url.
fn plane_cmd_with(mock_uri: &str) -> assert_cmd::Command {
    let mut cmd = plane_cmd();
    cmd.args([
        "--api-key",
        "test",
        "--workspace",
        "test-ws",
        "--base-url",
        mock_uri,
    ]);
    cmd
}

// ── Help & version ──

#[test]
fn no_subcommand_shows_help() {
    plane_cmd()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn version_flag() {
    plane_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

// ── Error handling ──

#[test]
fn missing_api_key_error() {
    plane_cmd()
        .env("PLANE_CLI_HOME", "/tmp/plane-cli-test-nonexistent")
        .env_remove("PLANE_CLI_API_KEY")
        .args(["--workspace", "ws", "projects", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("API key"));
}

#[test]
fn missing_workspace_error() {
    plane_cmd()
        .env("PLANE_CLI_HOME", "/tmp/plane-cli-test-nonexistent")
        .env_remove("PLANE_CLI_WORKSPACE")
        .args(["--api-key", "key", "projects", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("workspace"));
}

#[test]
fn invalid_priority_rejected() {
    plane_cmd()
        .env("PLANE_CLI_HOME", "/tmp/plane-cli-test-nonexistent")
        .args([
            "--api-key",
            "test",
            "--workspace",
            "ws",
            "issues",
            "create",
            "--project",
            "p1",
            "--title",
            "test",
            "--priority",
            "invalid",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[tokio::test]
async fn api_401_shows_unauthorized() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/test-ws/projects/"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    plane_cmd_with(&mock_server.uri())
        .args(["projects", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unauthorized"));
}

#[tokio::test]
async fn api_404_shows_not_found() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/api/v1/workspaces/test-ws/projects/proj1/issues/bad-id/",
        ))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&mock_server)
        .await;

    plane_cmd_with(&mock_server.uri())
        .args(["issues", "get", "--project", "proj1", "--id", "bad-id"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// ── Projects ──

#[tokio::test]
async fn projects_list_json() {
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

    plane_cmd_with(&mock_server.uri())
        .args(["--json", "projects", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Alpha"));
}

#[tokio::test]
async fn projects_list_table() {
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

    plane_cmd_with(&mock_server.uri())
        .args(["projects", "list"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Alpha")
                .and(predicate::str::contains("ALP"))
                .and(predicate::str::contains("Name")),
        );
}

#[tokio::test]
async fn projects_list_empty() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/test-ws/projects/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"results": []})))
        .mount(&mock_server)
        .await;

    plane_cmd_with(&mock_server.uri())
        .args(["projects", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No projects found"));
}

// ── Issues list ──

#[tokio::test]
async fn issues_list_json() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/test-ws/projects/proj1/issues/"))
        .and(query_param("per_page", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {"id": "iss-1", "sequence_id": 1, "name": "Bug A", "priority": "high"},
                {"id": "iss-2", "sequence_id": 2, "name": "Bug B", "priority": "low"}
            ]
        })))
        .mount(&mock_server)
        .await;

    plane_cmd_with(&mock_server.uri())
        .args(["--json", "issues", "list", "--project", "proj1"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Bug A")
                .and(predicate::str::contains("Bug B"))
                .and(predicate::str::contains("iss-1")),
        );
}

#[tokio::test]
async fn issues_list_table() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/test-ws/projects/proj1/issues/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {"id": "iss-1", "sequence_id": 1, "name": "Bug A", "priority": "high"}
            ]
        })))
        .mount(&mock_server)
        .await;

    plane_cmd_with(&mock_server.uri())
        .args(["issues", "list", "--project", "proj1"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Bug A")
                .and(predicate::str::contains("high"))
                .and(predicate::str::contains("Name")),
        );
}

#[tokio::test]
async fn issues_list_with_filters() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/test-ws/projects/proj1/issues/"))
        .and(query_param("state", "state-1"))
        .and(query_param("assignee", "user-1"))
        .and(query_param("per_page", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"results": []})))
        .expect(1)
        .mount(&mock_server)
        .await;

    plane_cmd_with(&mock_server.uri())
        .args([
            "issues",
            "list",
            "--project",
            "proj1",
            "--state",
            "state-1",
            "--assignee",
            "user-1",
            "--per-page",
            "10",
        ])
        .assert()
        .success();
}

// ── Issues get ──

#[tokio::test]
async fn issues_get_json() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/api/v1/workspaces/test-ws/projects/proj1/issues/iss-1/",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "iss-1",
            "sequence_id": 42,
            "name": "Login Bug",
            "priority": "urgent",
            "state": "state-1",
            "created_at": "2025-01-01T00:00:00Z",
            "assignees": ["user-1"],
            "labels": ["label-1"],
            "description_html": "<p>Details</p>"
        })))
        .mount(&mock_server)
        .await;

    plane_cmd_with(&mock_server.uri())
        .args([
            "--json",
            "issues",
            "get",
            "--project",
            "proj1",
            "--id",
            "iss-1",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Login Bug")
                .and(predicate::str::contains("iss-1"))
                .and(predicate::str::contains("urgent")),
        );
}

#[tokio::test]
async fn issues_get_table() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/api/v1/workspaces/test-ws/projects/proj1/issues/iss-1/",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "iss-1",
            "sequence_id": 42,
            "name": "Login Bug",
            "priority": "urgent",
            "state": "state-1",
            "created_at": "2025-01-01T00:00:00Z",
            "assignees": ["user-1"],
            "labels": ["label-1"],
            "description_html": "<p>Details</p>"
        })))
        .mount(&mock_server)
        .await;

    plane_cmd_with(&mock_server.uri())
        .args(["issues", "get", "--project", "proj1", "--id", "iss-1"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Login Bug")
                .and(predicate::str::contains("priority:"))
                .and(predicate::str::contains("user-1")),
        );
}

// ── Issues create ──

#[tokio::test]
async fn issues_create_json() {
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

    plane_cmd_with(&mock_server.uri())
        .args([
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

#[tokio::test]
async fn issues_create_table() {
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

    plane_cmd_with(&mock_server.uri())
        .args([
            "issues",
            "create",
            "--project",
            "proj1",
            "--title",
            "New Bug",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Created")
                .and(predicate::str::contains("#42"))
                .and(predicate::str::contains("New Bug")),
        );
}

// ── States ──

#[tokio::test]
async fn states_list_json() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/test-ws/projects/proj1/states/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {"id": "s1", "name": "Todo", "group": "backlog"},
                {"id": "s2", "name": "In Progress", "group": "started"}
            ]
        })))
        .mount(&mock_server)
        .await;

    plane_cmd_with(&mock_server.uri())
        .args(["--json", "states", "list", "--project", "proj1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Todo").and(predicate::str::contains("In Progress")));
}

#[tokio::test]
async fn states_list_table() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/test-ws/projects/proj1/states/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {"id": "s1", "name": "Todo", "group": "backlog"}
            ]
        })))
        .mount(&mock_server)
        .await;

    plane_cmd_with(&mock_server.uri())
        .args(["states", "list", "--project", "proj1"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Todo")
                .and(predicate::str::contains("backlog"))
                .and(predicate::str::contains("Group")),
        );
}

// ── Labels ──

#[tokio::test]
async fn labels_list_json() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/test-ws/projects/proj1/labels/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {"id": "l1", "name": "bug"},
                {"id": "l2", "name": "feature"}
            ]
        })))
        .mount(&mock_server)
        .await;

    plane_cmd_with(&mock_server.uri())
        .args(["--json", "labels", "list", "--project", "proj1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bug").and(predicate::str::contains("feature")));
}

#[tokio::test]
async fn labels_list_table() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/test-ws/projects/proj1/labels/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {"id": "l1", "name": "bug"}
            ]
        })))
        .mount(&mock_server)
        .await;

    plane_cmd_with(&mock_server.uri())
        .args(["labels", "list", "--project", "proj1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bug").and(predicate::str::contains("Name")));
}

// ── Members ──

#[tokio::test]
async fn members_list_json() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/test-ws/projects/proj1/members/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"id": "m1", "display_name": "Alice"},
            {"id": "m2", "display_name": "Bob"}
        ])))
        .mount(&mock_server)
        .await;

    plane_cmd_with(&mock_server.uri())
        .args(["--json", "members", "list", "--project", "proj1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice").and(predicate::str::contains("Bob")));
}

#[tokio::test]
async fn members_list_table() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/test-ws/projects/proj1/members/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"id": "m1", "display_name": "Alice"}
        ])))
        .mount(&mock_server)
        .await;

    plane_cmd_with(&mock_server.uri())
        .args(["members", "list", "--project", "proj1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice").and(predicate::str::contains("Name")));
}
