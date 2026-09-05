//! `actos inbox` — liste ve okundu-işaretleme sözleşme testleri.
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn inbox_payload() -> serde_json::Value {
    serde_json::json!({
        "notifications": [
            {
                "id": "n_1",
                "kind": "reply_to_comment",
                "actor": {
                    "id": "a_u2",
                    "username": "bob",
                    "actor_type": "human",
                    "display_name": null,
                    "bio": null,
                    "created_at": "2026-09-02T12:00:00Z",
                    "trust_level": 0
                },
                "target_type": "content",
                "target_id": "c_1",
                "payload": {},
                "created_at": "2026-09-02T12:00:00Z",
                "read_at": null
            }
        ],
        "next_cursor": null,
        "unread_count": 3
    })
}

#[tokio::test]
async fn test_inbox_list_table_and_total() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/me/inbox"))
        .respond_with(ResponseTemplate::new(200).set_body_json(inbox_payload()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["inbox", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("n_1"))
        .stdout(predicate::str::contains("bob"))
        .stdout(predicate::str::contains(
            "Total unread: 3 (this page shows 1 notification(s))",
        ));
}

#[tokio::test]
async fn test_inbox_list_json_includes_unread_count() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/me/inbox"))
        .respond_with(ResponseTemplate::new(200).set_body_json(inbox_payload()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    let out = cmd
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["--json", "inbox", "list"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "inbox list --json should succeed, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let v: serde_json::Value =
        serde_json::from_str(&stdout).expect("inbox list --json must emit valid JSON on stdout");
    assert_eq!(v["unread_count"], 3, "unread_count must be echoed");
    assert_eq!(
        v["notifications"][0]["id"], "n_1",
        "notifications array must be carried through"
    );
}

#[tokio::test]
async fn test_inbox_list_unread_passes_unread_query() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // Yalnızca `unread=true` sorgu parametresi mevcutsa eşleşir — böylece
    // bayrağın gerçekten query olarak iletildiği doğrulanır.
    Mock::given(method("GET"))
        .and(path("/me/inbox"))
        .and(query_param("unread", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(inbox_payload()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["inbox", "list", "--unread"])
        .assert()
        .success()
        .stdout(predicate::str::contains("n_1"));
}

#[tokio::test]
async fn test_inbox_read_all_is_idempotent() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // Aynı çağrı iki kez yapılsa da aynı istikrarlı yanıtla başarılı olur —
    // backend `marked` sayısını aynı döndürür (idempotent).
    Mock::given(method("POST"))
        .and(path("/me/inbox/read"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "marked": 5 })))
        .mount(&mock_server)
        .await;

    for _ in 0..2 {
        let mut cmd = Command::cargo_bin("actos").unwrap();
        cmd.env("ACTOS_CONFIG", tmp_config.path())
            .env("ACTOS_API_URL", mock_server.uri())
            .env("ACTOS_API_KEY", "actos_test_key")
            .args(["inbox", "read", "--all"])
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Marked 5 notification(s) as read.",
            ));
    }
}

#[tokio::test]
async fn test_inbox_read_single_updates_through_patch() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("PATCH"))
        .and(path("/me/inbox/n_1/read"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["inbox", "read", "n_1"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Notification 'n_1' marked as read.",
        ));
}

#[tokio::test]
async fn test_inbox_read_usage_errors() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // Snippet: ne id ne de --all verildi.
    let mut cmd_noargs = Command::cargo_bin("actos").unwrap();
    cmd_noargs
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["inbox", "read"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "Provide a notification <id> to read, or --all",
        ));

    // Snippet: hem id hem --all verildi.
    let mut cmd_both = Command::cargo_bin("actos").unwrap();
    cmd_both
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["inbox", "read", "n_1", "--all"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "Provide either a single notification <id> or --all, not both.",
        ));
}
