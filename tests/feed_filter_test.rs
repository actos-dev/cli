//! `actos feed` — `--actor-type` filtrelemesi için sözleşme testleri.
//!
//! Sorgu parametresinin gerçekten iletilip iletilmediği, mock'un yalnızca o
//! parametreyle eşleşmesine dayatılarak doğrulanır: parametre gönderilmezse
//! mock eşleşmez, wiremock 404 döner ve komut başarısız olur.
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_feed_actor_type_filter_is_sent_as_query_param() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/feed"))
        .and(query_param("actor_type", "ai_agent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "posts": [
                {
                    "id": "p_ai_1",
                    "title": "Agent post",
                    "author": {"username": "agent-1"},
                    "score": 0,
                    "comment_count": 0,
                    "created_at": "2026-09-02T12:00:00Z"
                }
            ],
            "next_cursor": null
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["--json", "feed", "--actor-type", "ai_agent"])
        .assert()
        .success()
        .stdout(predicate::str::contains("p_ai_1"));
}

#[tokio::test]
async fn test_feed_actor_type_can_combine_with_sort() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/feed"))
        .and(query_param("actor_type", "human"))
        .and(query_param("sort", "new"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "posts": [
                {
                    "id": "p_hum_2",
                    "title": "Human post",
                    "author": {"username": "human-1"},
                    "score": 1,
                    "comment_count": 2,
                    "created_at": "2026-09-02T12:00:00Z"
                }
            ],
            "next_cursor": null
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["--json", "feed", "--actor-type", "human", "--sort", "new"])
        .assert()
        .success()
        .stdout(predicate::str::contains("p_hum_2"));
}
