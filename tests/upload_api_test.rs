use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_upload_create_and_delete() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. Upload POST /uploads
    Mock::given(method("POST"))
        .and(path("/uploads"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "f_upload123",
            "url": "https://cdn.actos.dev/f_upload123.webp",
            "thumbnail_url": "https://cdn.actos.dev/thumb_f_upload123.webp",
            "mime_type": "image/webp",
            "byte_size": 1024,
            "width": 100,
            "height": 100,
            "checksum_sha256": "abcdef123456",
            "created_at": "2026-09-02T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    // Create a valid image file
    let mut tmp_img = tempfile::Builder::new().suffix(".png").tempfile().unwrap();
    tmp_img.write_all(b"fake png content").unwrap();

    // Upload create
    let mut cmd_up = Command::cargo_bin("actos").unwrap();
    cmd_up
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["upload", "create", tmp_img.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Upload successful: f_upload123"))
        .stdout(predicate::str::contains(
            "https://cdn.actos.dev/f_upload123.webp",
        ));

    // 2. Upload DELETE /uploads/{id}
    Mock::given(method("DELETE"))
        .and(path("/uploads/f_upload123"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // Delete without --yes -> exit code 2
    let mut cmd_del_no_yes = Command::cargo_bin("actos").unwrap();
    cmd_del_no_yes
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["upload", "delete", "f_upload123"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Pass '--yes' to confirm"));

    // Delete with --yes -> exit code 0
    let mut cmd_del_yes = Command::cargo_bin("actos").unwrap();
    cmd_del_yes
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["upload", "delete", "f_upload123", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Upload 'f_upload123' deleted."));
}

#[tokio::test]
async fn test_upload_client_side_validations() {
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. Unsupported extension (e.g. .txt)
    let mut tmp_txt = tempfile::Builder::new().suffix(".txt").tempfile().unwrap();
    tmp_txt.write_all(b"text file").unwrap();

    let mut cmd_unsupp = Command::cargo_bin("actos").unwrap();
    cmd_unsupp
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["upload", "create", tmp_txt.path().to_str().unwrap()])
        .assert()
        .code(8) // ExitCode::ValidationError
        .stderr(predicate::str::contains("Unsupported file extension"));

    // 2. File size exceeds 8 MB (8 * 1024 * 1024 + 1 bytes)
    let mut tmp_big = tempfile::Builder::new().suffix(".jpg").tempfile().unwrap();
    // Seek to 8.1 MB
    tmp_big
        .as_file_mut()
        .set_len(8 * 1024 * 1024 + 100)
        .unwrap();

    let mut cmd_big = Command::cargo_bin("actos").unwrap();
    cmd_big
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["upload", "create", tmp_big.path().to_str().unwrap()])
        .assert()
        .code(8) // ExitCode::ValidationError
        .stderr(predicate::str::contains("File size exceeds maximum allowed limit"));
}

#[tokio::test]
async fn test_post_create_with_attach() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. Upload mock
    Mock::given(method("POST"))
        .and(path("/uploads"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "f_attached_img",
            "url": "https://cdn.actos.dev/f_attached_img.webp",
            "thumbnail_url": "https://cdn.actos.dev/thumb.webp",
            "mime_type": "image/webp",
            "byte_size": 2048,
            "width": 200,
            "height": 200,
            "checksum_sha256": "123456",
            "created_at": "2026-09-02T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    // 2. Post create mock with attachment_ids
    Mock::given(method("POST"))
        .and(path("/posts"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "c_post_with_attach",
            "content_type": "post",
            "author": {
                "id": "a_u1",
                "username": "alice",
                "actor_type": "human",
                "display_name": null,
                "bio": null,
                "created_at": "2026-09-02T12:00:00Z",
                    "trust_level": 0,
            },
            "author_deleted": false,
            "title": "Ekli Post",
            "body": "Görsel eklendi",
            "body_format": "markdown",
            "metadata": {},
            "tags": [],
            "score": 0,
            "upvotes": 0,
            "downvotes": 0,
            "comment_count": 0,
            "created_at": "2026-09-02T12:00:00Z",
            "edited_at": null,
            "deleted": false
        })))
        .mount(&mock_server)
        .await;

    let mut tmp_img = tempfile::Builder::new().suffix(".webp").tempfile().unwrap();
    tmp_img.write_all(b"fake webp data").unwrap();

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args([
            "post",
            "create",
            "--title",
            "Ekli Post",
            "--body",
            "Görsel eklendi",
            "--attach",
            tmp_img.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("c_post_with_attach"));
}
