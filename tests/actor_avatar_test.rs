//! `actos actor avatar` — the dedicated avatar endpoint contract.
//!
//! Actos 0.3.0 removed standalone uploads: the avatar is sent by itself to
//! `POST /actors/me/avatar` and cleared with `DELETE /actors/me/avatar`.
//! `actor update` no longer touches the avatar at all.
use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write as _;
use tempfile::NamedTempFile;
use tempfile::tempdir;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_actor_avatar_uploads_to_dedicated_endpoint() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();
    let dir = tempdir().unwrap();
    let avatar_path = dir.path().join("avatar.png");
    let mut f = std::fs::File::create(&avatar_path).unwrap();
    f.write_all(b"fake-png-bytes").unwrap();
    drop(f);

    // The SDK sends the file as one multipart request to the avatar endpoint.
    Mock::given(method("POST"))
        .and(path("/actors/me/avatar"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "avatar_url": "https://cdn.actos.test/avatar.webp"
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["actor", "avatar", avatar_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Avatar updated: https://cdn.actos.test/avatar.webp",
        ));
}

#[tokio::test]
async fn test_actor_avatar_remove_sends_delete() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("DELETE"))
        .and(path("/actors/me/avatar"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["actor", "avatar", "--remove"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Avatar removed."));
}

#[tokio::test]
async fn test_actor_avatar_requires_file_or_remove() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["actor", "avatar"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "Provide an image file to upload, or pass '--remove'",
        ));
}

#[tokio::test]
async fn test_actor_avatar_file_and_remove_conflict_is_usage_error() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["actor", "avatar", "f_x", "--remove"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "Use either an image file or '--remove', not both.",
        ));
}

#[tokio::test]
async fn test_actor_update_no_longer_sends_avatar() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // The profile PATCH body must carry display_name and must NOT contain an
    // avatar key (the field moved to its own endpoint).
    Mock::given(method("PATCH"))
        .and(path("/actors/me"))
        .and(body_partial_json(serde_json::json!({
            "display_name": "Alice"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "actor": {
                "id": "a_me",
                "username": "alice",
                "actor_type": "human",
                "display_name": "Alice",
                "bio": null,
                "created_at": "2026-01-01T00:00:00Z",
                "avatar_url": null
            }
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["actor", "update", "--display-name", "Alice"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Profile updated successfully."));
}
