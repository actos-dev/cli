use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_feed_public_and_following() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. Public feed
    Mock::given(method("GET"))
        .and(path("/feed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "posts": [
                {
                    "id": "c_feed_1",
                    "content_type": "post",
                    "author": {
                        "id": "a_u1",
                        "username": "alice",
                        "actor_type": "human",
                        "display_name": null,
                        "bio": null,
                        "created_at": "2026-09-02T12:00:00Z"
                    },
                    "author_deleted": false,
                    "title": "Akış Gönderisi",
                    "body": "Gövde metni",
                    "body_format": "markdown",
                    "metadata": {},
                    "tags": ["trending"],
                    "score": 42,
                    "upvotes": 45,
                    "downvotes": 3,
                    "comment_count": 8,
                    "created_at": "2026-09-02T12:00:00Z",
                    "edited_at": null,
                    "deleted": false
                }
            ],
            "next_cursor": null
        })))
        .mount(&mock_server)
        .await;

    // 2. Following feed with auth
    Mock::given(method("GET"))
        .and(path("/feed/following"))
        .and(header("Authorization", "Bearer actos_feed_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "posts": [
                {
                    "id": "c_follow_1",
                    "content_type": "post",
                    "author": {
                        "id": "a_u2",
                        "username": "bob",
                        "actor_type": "human",
                        "display_name": null,
                        "bio": null,
                        "created_at": "2026-09-02T12:00:00Z"
                    },
                    "author_deleted": false,
                    "title": "Takip Edilen Gönderi",
                    "body": "Takip edilen yazar gövdesi",
                    "body_format": "markdown",
                    "metadata": {},
                    "tags": [],
                    "score": 15,
                    "upvotes": 15,
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

    // Public feed test
    let mut cmd_pub = Command::cargo_bin("actos").unwrap();
    cmd_pub
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["feed", "--sort", "hot", "--window", "week"])
        .assert()
        .success()
        .stdout(predicate::str::contains("c_feed_1"))
        .stdout(predicate::str::contains("Akış Gönderisi"))
        .stdout(predicate::str::contains("42"));

    // Following feed without auth fails with exit code 3
    let mut cmd_fol_no_auth = Command::cargo_bin("actos").unwrap();
    cmd_fol_no_auth
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["feed", "--following"])
        .assert()
        .code(3);

    // Following feed with auth succeeds
    let mut cmd_fol_auth = Command::cargo_bin("actos").unwrap();
    cmd_fol_auth
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_feed_key")
        .args(["feed", "--following"])
        .assert()
        .success()
        .stdout(predicate::str::contains("c_follow_1"))
        .stdout(predicate::str::contains("Takip Edilen Gönderi"))
        .stdout(predicate::str::contains("bob"));
}

#[tokio::test]
async fn test_search_types_and_empty_handling() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. Search post
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {
                    "id": "c_search_post",
                    "content_type": "post",
                    "author": {
                        "id": "a_u1",
                        "username": "alice",
                        "actor_type": "human",
                        "display_name": null,
                        "bio": null,
                        "created_at": "2026-09-02T12:00:00Z"
                    },
                    "author_deleted": false,
                    "title": "Arama Sonucu Post",
                    "body": "Rust ve Actos araması",
                    "body_format": "markdown",
                    "metadata": {},
                    "tags": ["rust"],
                    "score": 10,
                    "upvotes": 10,
                    "downvotes": 0,
                    "comment_count": 3,
                    "created_at": "2026-09-02T12:00:00Z",
                    "edited_at": null,
                    "deleted": false
                }
            ],
            "next_cursor": null
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Search post
    let mut cmd_post = Command::cargo_bin("actos").unwrap();
    cmd_post
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["search", "rust", "--type", "post"])
        .assert()
        .success()
        .stdout(predicate::str::contains("c_search_post"))
        .stdout(predicate::str::contains("Arama Sonucu Post"));

    // 2. Empty results (must exit 0 and print polite message)
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [],
            "next_cursor": null
        })))
        .mount(&mock_server)
        .await;

    let mut cmd_empty = Command::cargo_bin("actos").unwrap();
    cmd_empty
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["search", "nonexistent_query", "--type", "actor"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "No results found for query 'nonexistent_query'.",
        ));
}
