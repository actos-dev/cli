use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_vote_up_down_clear() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. Upvote
    Mock::given(method("PUT"))
        .and(path("/contents/c_vote1/vote"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "value": 1,
            "score": 10,
            "upvotes": 10,
            "downvotes": 0
        })))
        .mount(&mock_server)
        .await;

    // 2. Downvote
    Mock::given(method("PUT"))
        .and(path("/contents/c_vote2/vote"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "value": -1,
            "score": -2,
            "upvotes": 1,
            "downvotes": 3
        })))
        .mount(&mock_server)
        .await;

    // 3. Clear
    Mock::given(method("PUT"))
        .and(path("/contents/c_vote3/vote"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "value": 0,
            "score": 5,
            "upvotes": 5,
            "downvotes": 0
        })))
        .mount(&mock_server)
        .await;

    // Upvote
    let mut cmd_up = Command::cargo_bin("actos").unwrap();
    cmd_up
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["vote", "up", "c_vote1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Upvoted 'c_vote1'"))
        .stdout(predicate::str::contains("Score: 10 (+10 / -0)"));

    // Downvote
    let mut cmd_down = Command::cargo_bin("actos").unwrap();
    cmd_down
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["vote", "down", "c_vote2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Downvoted 'c_vote2'"))
        .stdout(predicate::str::contains("Score: -2 (+1 / -3)"));

    // Clear vote
    let mut cmd_clear = Command::cargo_bin("actos").unwrap();
    cmd_clear
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["vote", "clear", "c_vote3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cleared vote for 'c_vote3'"))
        .stdout(predicate::str::contains("Score: 5 (+5 / -0)"));
}

#[tokio::test]
async fn test_vote_own_content_fails_with_exit_4() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("PUT"))
        .and(path("/contents/c_own_post/vote"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "type": "https://docs.actos.dev/errors/forbidden",
            "title": "Kendi içeriğinize oy veremezsiniz",
            "status": 403,
            "code": "FORBIDDEN"
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["vote", "up", "c_own_post"])
        .assert()
        .code(4); // ExitCode::Forbidden
}

#[tokio::test]
async fn test_vote_status_batch() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/me/votes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "votes": {
                "c_1": 1,
                "c_2": -1
            }
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["vote", "status", "--ids", "c_1,c_2,c_3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("+1 (Up)"))
        .stdout(predicate::str::contains("-1 (Down)"))
        .stdout(predicate::str::contains("0 (None)"));
}

#[tokio::test]
async fn test_save_add_remove_list() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. Add
    Mock::given(method("PUT"))
        .and(path("/contents/c_save1/save"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // 2. Remove
    Mock::given(method("DELETE"))
        .and(path("/contents/c_save1/save"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // 3. List
    Mock::given(method("GET"))
        .and(path("/me/saves"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "saves": [
                {
                    "id": "c_saved_post",
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
                    "title": "Kaydedilen Gönderi",
                    "body": "Gövde",
                    "body_format": "markdown",
                    "metadata": {},
                    "tags": [],
                    "score": 10,
                    "upvotes": 10,
                    "downvotes": 0,
                    "comment_count": 0,
                    "created_at": "2026-09-02T12:00:00Z",
                    "edited_at": null,
                    "deleted": false
                }
            ],
            "next_cursor": null
        })))
        .mount(&mock_server)
        .await;

    // Add save
    let mut cmd_add = Command::cargo_bin("actos").unwrap();
    cmd_add
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["save", "add", "c_save1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Saved 'c_save1'."));

    // Remove save
    let mut cmd_rem = Command::cargo_bin("actos").unwrap();
    cmd_rem
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["save", "remove", "c_save1"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Removed 'c_save1' from saved items.",
        ));

    // List saves
    let mut cmd_list = Command::cargo_bin("actos").unwrap();
    cmd_list
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["save", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("c_saved_post"))
        .stdout(predicate::str::contains("Kaydedilen Gönderi"))
        .stdout(predicate::str::contains("alice"));
}
