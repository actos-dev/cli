use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_api_get_and_post_with_fields() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. GET /health
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok"
        })))
        .mount(&mock_server)
        .await;

    // 2. POST /posts
    Mock::given(method("POST"))
        .and(path("/posts"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "c_api_post",
            "title": "API Post",
            "body": "Gövde"
        })))
        .mount(&mock_server)
        .await;

    // GET /health
    let mut cmd_get = Command::cargo_bin("actos").unwrap();
    cmd_get
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["api", "GET", "/health", "-f", "verbose=true"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"ok\""));

    // POST /posts with -f fields
    let mut cmd_post = Command::cargo_bin("actos").unwrap();
    cmd_post
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args([
            "api",
            "POST",
            "/posts",
            "-f",
            "title=API Post",
            "-f",
            "body=Gövde",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"id\": \"c_api_post\""));
}

#[tokio::test]
async fn test_api_raw() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/raw_text"))
        .respond_with(ResponseTemplate::new(200).set_body_string("PLAIN TEXT RESPONSE"))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["api", "GET", "/raw_text", "--raw"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PLAIN TEXT RESPONSE"));
}

#[tokio::test]
async fn test_docs_agent_markdown() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/docs/agent"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("# Actos Agent Documentation\n\nWelcome agents."),
        )
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["docs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Actos Agent Documentation"))
        .stdout(predicate::str::contains("Welcome agents."));
}

#[tokio::test]
async fn test_quota_and_version() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. /health with rate limit headers
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("x-ratelimit-limit", "100")
                .append_header("x-ratelimit-remaining", "95")
                .append_header("x-ratelimit-reset", "1725280000")
                .set_body_json(serde_json::json!({ "status": "ok" })),
        )
        .mount(&mock_server)
        .await;

    // 2. /version
    Mock::given(method("GET"))
        .and(path("/version"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "actos-api",
            "version": "0.1.0",
            "git_sha": "abc1234",
            "api_version": "v1"
        })))
        .mount(&mock_server)
        .await;

    // Quota
    let mut cmd_quota = Command::cargo_bin("actos").unwrap();
    cmd_quota
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["quota"])
        .assert()
        .success()
        .stdout(predicate::str::contains("100"))
        .stdout(predicate::str::contains("95"));

    // Version JSON
    let mut cmd_ver = Command::cargo_bin("actos").unwrap();
    cmd_ver
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["version", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"cli\""))
        .stdout(predicate::str::contains("\"server\""))
        .stdout(predicate::str::contains("\"abc1234\""));
}
