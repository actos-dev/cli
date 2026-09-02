use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;
use wiremock::matchers::{header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_post_create_literal_file_stdin() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("POST"))
        .and(path("/posts"))
        .and(header_exists("Idempotency-Key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "c_post123",
            "content_type": "post",
            "author": {
                "id": "a_user1",
                "username": "alice",
                "actor_type": "human",
                "display_name": null,
                "bio": null,
                "created_at": "2026-09-02T12:00:00Z"
            },
            "author_deleted": false,
            "title": "Başlık",
            "body": "Gövde metni",
            "body_format": "markdown",
            "metadata": {},
            "tags": ["rust", "cli"],
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

    // 1. Literal body
    let mut cmd_lit = Command::cargo_bin("actos").unwrap();
    cmd_lit
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args([
            "post",
            "create",
            "--title",
            "Başlık",
            "--body",
            "Gövde metni",
            "--tag",
            "rust",
            "--tag",
            "cli",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("c_post123"));

    // 2. Body from file @file.md
    let mut file_tmp = NamedTempFile::new().unwrap();
    write!(file_tmp, "File content markdown").unwrap();
    let file_arg = format!("@{}", file_tmp.path().display());

    let mut cmd_file = Command::cargo_bin("actos").unwrap();
    cmd_file
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args([
            "post",
            "create",
            "--title",
            "Dosyadan Başlık",
            "--body",
            &file_arg,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("c_post123"));

    // 3. Body from stdin (-)
    let mut cmd_stdin = Command::cargo_bin("actos").unwrap();
    cmd_stdin
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["post", "create", "--title", "Stdin Başlık", "--body", "-"])
        .write_stdin("Stdin content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("c_post123"));
}

#[tokio::test]
async fn test_post_view_200_and_url_parsing() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/posts/c_abc123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "c_abc123",
            "content_type": "post",
            "author": {
                "id": "a_user1",
                "username": "alice",
                "actor_type": "human",
                "display_name": "Alice",
                "bio": null,
                "created_at": "2026-09-02T12:00:00Z"
            },
            "author_deleted": false,
            "title": "Harika Bir Gönderi",
            "body": "Gönderi içeriği burada yer alıyor.",
            "body_format": "markdown",
            "metadata": {},
            "tags": ["rust"],
            "score": 5,
            "upvotes": 5,
            "downvotes": 0,
            "comment_count": 0,
            "created_at": "2026-09-02T12:00:00Z",
            "edited_at": null,
            "deleted": false
        })))
        .mount(&mock_server)
        .await;

    // URL olarak ID geçilmesi testi
    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["post", "view", "https://actos.com.tr/posts/c_abc123"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Harika Bir Gönderi"))
        .stdout(predicate::str::contains("Alice"))
        .stdout(predicate::str::contains("Score:   5"));
}

#[tokio::test]
async fn test_post_view_404_vs_410_exit_codes() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 404 Not Found
    Mock::given(method("GET"))
        .and(path("/posts/c_nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "type": "https://docs.actos.dev/errors/not-found",
            "title": "Kayıt yok",
            "status": 404,
            "code": "NOT_FOUND"
        })))
        .mount(&mock_server)
        .await;

    // 410 Gone (silinmiş içerik)
    Mock::given(method("GET"))
        .and(path("/posts/c_deleted"))
        .respond_with(ResponseTemplate::new(410).set_body_json(serde_json::json!({
            "type": "https://docs.actos.dev/errors/gone",
            "title": "İçerik silinmiş",
            "status": 410,
            "code": "GONE"
        })))
        .mount(&mock_server)
        .await;

    // 404 -> exit code 5
    let mut cmd_404 = Command::cargo_bin("actos").unwrap();
    cmd_404
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["post", "view", "c_nonexistent"])
        .assert()
        .code(5);

    // 410 -> exit code 6
    let mut cmd_410 = Command::cargo_bin("actos").unwrap();
    cmd_410
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["post", "view", "c_deleted"])
        .assert()
        .code(6);
}

#[tokio::test]
async fn test_post_edit() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("PATCH"))
        .and(path("/posts/c_edit123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "c_edit123",
            "content_type": "post",
            "author": {
                "id": "a_user1",
                "username": "alice",
                "actor_type": "human",
                "display_name": null,
                "bio": null,
                "created_at": "2026-09-02T12:00:00Z"
            },
            "author_deleted": false,
            "title": "Güncellenmiş Başlık",
            "body": "Yeni içerik",
            "body_format": "markdown",
            "metadata": {},
            "tags": [],
            "score": 0,
            "upvotes": 0,
            "downvotes": 0,
            "comment_count": 0,
            "created_at": "2026-09-02T12:00:00Z",
            "edited_at": "2026-09-02T13:00:00Z",
            "deleted": false
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args([
            "post",
            "edit",
            "c_edit123",
            "--title",
            "Güncellenmiş Başlık",
            "--body",
            "Yeni içerik",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Post 'c_edit123' updated successfully.",
        ));
}

#[tokio::test]
async fn test_post_delete_confirmation() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("DELETE"))
        .and(path("/posts/c_del123"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // 1. --yes olmadan silme işlemi: Exit code 2 (USAGE)
    let mut cmd_no_yes = Command::cargo_bin("actos").unwrap();
    cmd_no_yes
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["post", "delete", "c_del123"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Pass '--yes' to confirm"));

    // 2. --yes ile silme işlemi: Exit code 0 (Başarı)
    let mut cmd_yes = Command::cargo_bin("actos").unwrap();
    cmd_yes
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["post", "delete", "c_del123", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Post 'c_del123' deleted."));
}

#[tokio::test]
async fn test_post_list_pagination() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/actors/alice/posts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "posts": [
                {
                    "id": "c_1",
                    "content_type": "post",
                    "author": {
                        "id": "a_1",
                        "username": "alice",
                        "actor_type": "human",
                        "display_name": null,
                        "bio": null,
                        "created_at": "2026-09-02T12:00:00Z"
                    },
                    "author_deleted": false,
                    "title": "Post 1",
                    "body": "Body 1",
                    "body_format": "markdown",
                    "metadata": {},
                    "tags": ["tech"],
                    "score": 10,
                    "upvotes": 10,
                    "downvotes": 0,
                    "comment_count": 2,
                    "created_at": "2026-09-02T12:00:00Z",
                    "edited_at": null,
                    "deleted": false
                }
            ],
            "next_cursor": null
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["post", "list", "--actor", "alice"])
        .assert()
        .success()
        .stdout(predicate::str::contains("c_1"))
        .stdout(predicate::str::contains("Post 1"))
        .stdout(predicate::str::contains("alice"));
}
