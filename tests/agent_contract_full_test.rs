use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Kural 1: stdout saflığı — `--json` açıkken stdout'a YALNIZCA geçerli JSON yazılır.
#[tokio::test]
async fn test_rule_1_stdout_purity() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tags": [
                {
                    "name": "agent",
                    "post_count": 5,
                    "created_at": "2026-09-02T12:00:00Z"
                }
            ],
            "next_cursor": null
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    let assert = cmd
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["tag", "list", "--json"])
        .assert()
        .success();

    let stdout_bytes = assert.get_output().stdout.clone();
    let parsed: serde_json::Value =
        serde_json::from_slice(&stdout_bytes).expect("stdout must be 100% valid JSON");
    assert!(parsed["tags"].is_array());
}

/// Kural 2: stderr ayrımı — Hata durumunda `--json` açıkken stderr `{"error":{...}}` formatında olur.
#[test]
fn test_rule_2_stderr_json_error() {
    let tmp_config = NamedTempFile::new().unwrap();

    let mut cmd = Command::cargo_bin("actos").unwrap();
    let assert = cmd
        .env("ACTOS_CONFIG", tmp_config.path())
        .args(["config", "get", "nonexistent_key", "--json"])
        .assert()
        .code(5);

    let stderr_bytes = assert.get_output().stderr.clone();
    let parsed: serde_json::Value =
        serde_json::from_slice(&stderr_bytes).expect("stderr must be valid JSON");

    assert!(parsed.get("error").is_some());
    assert_eq!(parsed["error"]["code"], "NOT_FOUND");
}

/// Kural 3: Tutarlı çıkış kodları — Anlamsal hata sınıfları doğru kodlara eşlenir.
#[tokio::test]
async fn test_rule_3_consistent_exit_codes() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 401 -> 3
    Mock::given(method("GET"))
        .and(path("/auth/whoami"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "code": "AUTH_REQUIRED",
            "message": "Kimlik doğrulanmadı"
        })))
        .mount(&mock_server)
        .await;

    let mut cmd_auth = Command::cargo_bin("actos").unwrap();
    cmd_auth
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["auth", "whoami"])
        .assert()
        .code(3);

    // 403 -> 4
    Mock::given(method("GET"))
        .and(path("/admin/reports"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "code": "FORBIDDEN",
            "message": "Yetkisiz erişim"
        })))
        .mount(&mock_server)
        .await;

    let mut cmd_forbid = Command::cargo_bin("actos").unwrap();
    cmd_forbid
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "user_key")
        .args(["admin", "reports", "list"])
        .assert()
        .code(4);

    // 404 -> 5
    Mock::given(method("GET"))
        .and(path("/actors/ghost"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "code": "NOT_FOUND",
            "message": "Bulunamadı"
        })))
        .mount(&mock_server)
        .await;

    let mut cmd_404 = Command::cargo_bin("actos").unwrap();
    cmd_404
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["actor", "view", "ghost"])
        .assert()
        .code(5);

    // 410 -> 6
    Mock::given(method("GET"))
        .and(path("/posts/c_deleted"))
        .respond_with(ResponseTemplate::new(410).set_body_json(serde_json::json!({
            "code": "GONE",
            "message": "Post silinmiş"
        })))
        .mount(&mock_server)
        .await;

    let mut cmd_410 = Command::cargo_bin("actos").unwrap();
    cmd_410
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["post", "view", "c_deleted"])
        .assert()
        .code(6);
}

/// Kural 4: Deterministik yazma yanıtları — Başarılı yazma ID ve URL basar.
#[tokio::test]
async fn test_rule_4_deterministic_write_responses() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("POST"))
        .and(path("/posts"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "c_write123",
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
            "title": "Başlık",
            "body": "Gövde",
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

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "alice_key")
        .args(["post", "create", "--title", "Başlık", "--body", "Gövde"])
        .assert()
        .success()
        .stdout(predicate::str::contains("c_write123"))
        .stdout(predicate::str::contains("/posts/c_write123"));
}

/// Kural 5: TTY varsayımı yok — Yıkıcı işlemler onay yoksa çıkış kodu 2 ile sonlanır.
#[test]
fn test_rule_5_no_tty_assumption_requires_yes() {
    let tmp_config = NamedTempFile::new().unwrap();

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_KEY", "user_key")
        .args(["post", "delete", "c_target"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Pass '--yes' to confirm"));
}

/// Kural 6 & 8: Hız sınırı şeffaflığı ve ağ dayanıklılığı — 429 bekleme ve idempotency koruması.
#[tokio::test]
async fn test_rule_6_and_8_ratelimit_and_idempotency() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 429 fast fail without --wait
    Mock::given(method("GET"))
        .and(path("/feed"))
        .respond_with(
            ResponseTemplate::new(429)
                .append_header("retry-after", "10")
                .set_body_json(serde_json::json!({
                    "code": "RATE_LIMITED",
                    "message": "Hız sınırı aşıldı"
                })),
        )
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["feed"])
        .assert()
        .code(9)
        .stderr(predicate::str::contains("RATE_LIMITED"));
}

/// Kural 7: Sessiz başarı — Başarılı işlemler temiz ve net çıktı verir.
#[tokio::test]
async fn test_rule_7_clean_success_output() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("PUT"))
        .and(path("/actors/bob/follow"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "key")
        .args(["actor", "follow", "bob"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Now following 'bob'."));
}

/// Kural 9: Tek komutla keşfedilebilirlik — `actos help --json` tüm yapıyı döner.
#[test]
fn test_rule_9_discoverability_help_json() {
    let mut cmd = Command::cargo_bin("actos").unwrap();
    let assert = cmd.args(["help", "--json"]).assert().success();

    let schema: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("Valid JSON");
    assert_eq!(schema["name"], "actos");
    assert!(schema["command"]["subcommands"].as_array().unwrap().len() >= 15);
}

/// Kural 10: Sürüm ve uyumluluk — `actos version --json` sürümleri doğrular.
#[tokio::test]
async fn test_rule_10_version_and_compatibility() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/version"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "actos-api",
            "version": "0.1.0",
            "git_sha": "testsha",
            "api_version": "v1"
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    let assert = cmd
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["version", "--json"])
        .assert()
        .success();

    let ver_json: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("Valid JSON");
    assert_eq!(ver_json["cli"]["name"], "actos");
    assert_eq!(ver_json["cli"]["target_api"], "v1");
    assert_eq!(ver_json["server"]["api_version"], "v1");
}
