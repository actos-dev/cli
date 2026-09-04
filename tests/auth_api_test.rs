use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_auth_register_and_whoami_flow() {
    let mock_server = MockServer::start().await;
    let tmp = NamedTempFile::new().unwrap();

    // 1. Mock POST /auth/register
    Mock::given(method("POST"))
        .and(path("/auth/register"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "actor": {
                "id": "a_alice123",
                "username": "alice",
                "actor_type": "human",
                "display_name": "Alice Wonderland",
                "bio": null,
                "created_at": "2026-09-02T12:00:00Z",
                    "trust_level": 0,
            },
            "api_key": "actos_alice_secret_key_12345",
            "recovery_codes": [
                "1111-2222-3333",
                "4444-5555-6666"
            ]
        })))
        .mount(&mock_server)
        .await;

    // Register with --save
    let mut cmd_reg = Command::cargo_bin("actos").unwrap();
    cmd_reg
        .env("ACTOS_CONFIG", tmp.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args([
            "auth",
            "register",
            "--username",
            "alice",
            "--type",
            "human",
            "--display-name",
            "Alice Wonderland",
            "--save",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("alice"))
        .stdout(predicate::str::contains("a_alice123"))
        .stdout(predicate::str::contains("actos_alice_secret_key_12345"))
        .stdout(predicate::str::contains("1111-2222-3333"));

    // 2. Mock GET /auth/whoami
    Mock::given(method("GET"))
        .and(path("/auth/whoami"))
        .and(header(
            "Authorization",
            "Bearer actos_alice_secret_key_12345",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "actor": {
                "id": "a_alice123",
                "username": "alice",
                "actor_type": "human",
                "display_name": "Alice Wonderland",
                "bio": null,
                "created_at": "2026-09-02T12:00:00Z",
                    "trust_level": 0,
            },
            "roles": ["moderator"],
            "key": {
                "id": "key_uuid_1",
                "label": "default",
                "created_at": "2026-09-02T12:00:00Z",
                "last_used_at": null,
                "revoked_at": null
            }
        })))
        .mount(&mock_server)
        .await;

    // Whoami command (uses saved key in config)
    let mut cmd_whoami = Command::cargo_bin("actos").unwrap();
    cmd_whoami
        .env("ACTOS_CONFIG", tmp.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["auth", "whoami"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alice"))
        .stdout(predicate::str::contains("moderator"))
        .stdout(predicate::str::contains("key_uuid_1"));
}

#[tokio::test]
async fn test_auth_login_stdin() {
    let mock_server = MockServer::start().await;
    let tmp = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/auth/whoami"))
        .and(header("Authorization", "Bearer actos_login_via_stdin_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "actor": {
                "id": "a_bob",
                "username": "bob",
                "actor_type": "ai_agent",
                "display_name": null,
                "bio": null,
                "created_at": "2026-09-02T12:00:00Z",
                    "trust_level": 0,
            },
            "roles": [],
            "key": {
                "id": "key_bob_uuid",
                "label": "cli",
                "created_at": "2026-09-02T12:00:00Z",
                "last_used_at": null,
                "revoked_at": null
            }
        })))
        .mount(&mock_server)
        .await;

    let mut cmd_login = Command::cargo_bin("actos").unwrap();
    cmd_login
        .env("ACTOS_CONFIG", tmp.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["auth", "login", "--stdin"])
        .write_stdin("actos_login_via_stdin_key\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Logged in as bob (ai_agent)"));

    // Verify key was saved into config
    let config = actos::config::Config::load_from_path(tmp.path()).unwrap();
    assert_eq!(
        config.get("default", "api_key").unwrap(),
        "actos_login_via_stdin_key"
    );
    assert_eq!(config.get("default", "username").unwrap(), "bob");
}

#[tokio::test]
async fn test_auth_invalid_key_fails_with_code_3() {
    let mock_server = MockServer::start().await;
    let tmp = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/auth/whoami"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "type": "https://docs.actos.dev/errors/invalid-key",
            "title": "Kimlik bilgisi geçersiz",
            "status": 401,
            "code": "INVALID_KEY"
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_invalid_key")
        .args(["auth", "whoami"])
        .assert()
        .code(3); // ExitCode::AuthFailed
}

#[tokio::test]
async fn test_auth_keys_management() {
    let mock_server = MockServer::start().await;
    let tmp = NamedTempFile::new().unwrap();

    // 1. List keys
    Mock::given(method("GET"))
        .and(path("/auth/keys"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "keys": [
                {
                    "id": "key_1",
                    "label": "laptop",
                    "created_at": "2026-09-02T10:00:00Z",
                    "last_used_at": null,
                    "revoked_at": null
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    // 2. Create key
    Mock::given(method("POST"))
        .and(path("/auth/keys"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "key": {
                "id": "key_2",
                "label": "ci",
                "created_at": "2026-09-02T11:00:00Z",
                "last_used_at": null,
                "revoked_at": null
            },
            "api_key": "actos_new_ci_key"
        })))
        .mount(&mock_server)
        .await;

    // 3. Revoke key
    Mock::given(method("DELETE"))
        .and(path("/auth/keys/key_1"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // List
    let mut cmd_list = Command::cargo_bin("actos").unwrap();
    cmd_list
        .env("ACTOS_CONFIG", tmp.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_master_key")
        .args(["auth", "keys", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("key_1"))
        .stdout(predicate::str::contains("laptop"));

    // Create
    let mut cmd_create = Command::cargo_bin("actos").unwrap();
    cmd_create
        .env("ACTOS_CONFIG", tmp.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_master_key")
        .args(["auth", "keys", "create", "--label", "ci"])
        .assert()
        .success()
        .stdout(predicate::str::contains("key_2"))
        .stdout(predicate::str::contains("actos_new_ci_key"));

    // Revoke
    let mut cmd_revoke = Command::cargo_bin("actos").unwrap();
    cmd_revoke
        .env("ACTOS_CONFIG", tmp.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_master_key")
        .args(["auth", "keys", "revoke", "key_1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("API key 'key_1' revoked."));
}

#[tokio::test]
async fn test_auth_logout() {
    let tmp = NamedTempFile::new().unwrap();
    let toml = r#"
default_profile = "default"

[profiles.default]
api_url = "https://api.actos.dev"
api_key = "actos_logged_in_key"
username = "alice"
"#;
    std::fs::write(tmp.path(), toml).unwrap();

    let mut cmd_logout = Command::cargo_bin("actos").unwrap();
    cmd_logout
        .env("ACTOS_CONFIG", tmp.path())
        .args(["auth", "logout"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Logged out from profile 'default'.",
        ));

    let config = actos::config::Config::load_from_path(tmp.path()).unwrap();
    assert_eq!(config.get("default", "api_key").unwrap(), "");
}
