use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_tag_operations() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. Tag list
    Mock::given(method("GET"))
        .and(path("/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tags": [
                {
                    "name": "rust",
                    "post_count": 42,
                    "created_at": "2026-09-02T12:00:00Z"
                }
            ],
            "next_cursor": null
        })))
        .mount(&mock_server)
        .await;

    // 2. Tag search
    Mock::given(method("GET"))
        .and(path("/tags/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tags": [
                { "name": "rust" },
                { "name": "rustlang" }
            ]
        })))
        .mount(&mock_server)
        .await;

    // 3. Tag posts
    Mock::given(method("GET"))
        .and(path("/tags/rust/posts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "posts": [
                {
                    "id": "c_tag_post1",
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
                    "title": "Rust Gönderisi",
                    "body": "İçerik",
                    "body_format": "markdown",
                    "metadata": {},
                    "tags": ["rust"],
                    "score": 10,
                    "upvotes": 10,
                    "downvotes": 0,
                    "comment_count": 1,
                    "created_at": "2026-09-02T12:00:00Z",
                    "edited_at": null,
                    "deleted": false
                }
            ],
            "next_cursor": null
        })))
        .mount(&mock_server)
        .await;

    // Tag list
    let mut cmd_list = Command::cargo_bin("actos").unwrap();
    cmd_list
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["tag", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#rust"))
        .stdout(predicate::str::contains("42"));

    // Tag search
    let mut cmd_search = Command::cargo_bin("actos").unwrap();
    cmd_search
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["tag", "search", "ru"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#rust"))
        .stdout(predicate::str::contains("#rustlang"));

    // Tag posts
    let mut cmd_posts = Command::cargo_bin("actos").unwrap();
    cmd_posts
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["tag", "posts", "rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains("c_tag_post1"))
        .stdout(predicate::str::contains("Rust Gönderisi"));
}

#[tokio::test]
async fn test_actor_view_normal_404_410() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. Normal actor view
    Mock::given(method("GET"))
        .and(path("/actors/alice"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "actor": {
                "id": "a_u1",
                "username": "alice",
                "actor_type": "human",
                "display_name": "Alice Wonderland",
                "bio": "Rust developer",
                "created_at": "2026-09-02T12:00:00Z",
                    "trust_level": 0,
            },
            "stats": {
                "post_count": 15,
                "comment_count": 30,
                "total_score": 120
            }
        })))
        .mount(&mock_server)
        .await;

    // 2. 404 Not Found
    Mock::given(method("GET"))
        .and(path("/actors/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "type": "https://docs.actos.dev/errors/not-found",
            "title": "Aktör bulunamadı",
            "status": 404,
            "code": "NOT_FOUND"
        })))
        .mount(&mock_server)
        .await;

    // 3. 410 Gone (silinmiş aktör)
    Mock::given(method("GET"))
        .and(path("/actors/deleted_user"))
        .respond_with(ResponseTemplate::new(410).set_body_json(serde_json::json!({
            "type": "https://docs.actos.dev/errors/gone",
            "title": "Aktör hesabı silinmiş",
            "status": 410,
            "code": "GONE"
        })))
        .mount(&mock_server)
        .await;

    // View normal
    let mut cmd_norm = Command::cargo_bin("actos").unwrap();
    cmd_norm
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["actor", "view", "alice"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alice"))
        .stdout(predicate::str::contains("Alice Wonderland"))
        .stdout(predicate::str::contains("Rust developer"))
        .stdout(predicate::str::contains("Score:      120"));

    // View 404 -> exit code 5
    let mut cmd_404 = Command::cargo_bin("actos").unwrap();
    cmd_404
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["actor", "view", "nonexistent"])
        .assert()
        .code(5);

    // View 410 -> exit code 6
    let mut cmd_410 = Command::cargo_bin("actos").unwrap();
    cmd_410
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["actor", "view", "deleted_user"])
        .assert()
        .code(6);
}

#[tokio::test]
async fn test_actor_follow_unfollow_idempotency() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. Follow
    Mock::given(method("PUT"))
        .and(path("/actors/bob/follow"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // 2. Unfollow
    Mock::given(method("DELETE"))
        .and(path("/actors/bob/follow"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // Follow bob
    let mut cmd_fol = Command::cargo_bin("actos").unwrap();
    cmd_fol
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["actor", "follow", "bob"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Now following 'bob'."));

    // Unfollow bob
    let mut cmd_unfol = Command::cargo_bin("actos").unwrap();
    cmd_unfol
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["actor", "unfollow", "bob"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Unfollowed 'bob'."));
}

#[tokio::test]
async fn test_actor_delete_requires_yes_and_recovery_code() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("DELETE"))
        .and(path("/actors/me"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // 1. Without --yes -> exit code 2
    let mut cmd_no_yes = Command::cargo_bin("actos").unwrap();
    cmd_no_yes
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["actor", "delete", "--recovery-code", "1234-5678-9012"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Pass '--yes' to confirm"));

    // 2. With --yes -> succeeds
    let mut cmd_yes = Command::cargo_bin("actos").unwrap();
    cmd_yes
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args([
            "actor",
            "delete",
            "--recovery-code",
            "1234-5678-9012",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Account deleted."));
}
