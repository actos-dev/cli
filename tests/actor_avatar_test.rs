//! `actos actor update` — avatar üç durumlu (tri-state) sözleşme testleri.
//!
//! Avatar: `--no-avatar` açıkça JSON `null` gönderir; `--avatar <dosya>`
//! önce `POST /uploads` ile yükler sonra dönen attachment id'sini gönderir;
//! `--avatar f_...` önceden yüklenmiş id olarak doğrudan gönderilir (yükleme
//! yeniden yapılmaz).
use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write as _;
use tempfile::NamedTempFile;
use tempfile::tempdir;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn upload_response(id: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "url": format!("https://cdn.actos.test/{id}.webp"),
        "thumbnail_url": format!("https://cdn.actos.test/{id}.thumb.webp"),
        "mime_type": "image/webp",
        "byte_size": 42,
        "width": null,
        "height": null,
        "checksum_sha256": "abc123",
        "created_at": "2026-09-02T12:00:00Z"
    })
}

#[tokio::test]
async fn test_actor_no_avatar_sends_explicit_null() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // PATCH gövdesi `avatar: null` AÇIKÇA içermeli — `body_partial_json` ile
    // null değeri eşleşmesi zorunlu kılınır. Bayrak hiç verilmediğinde anahtar
    // hiç yer almayacağı için bu mock eşleşmez.
    Mock::given(method("PATCH"))
        .and(path("/actors/me"))
        .and(body_partial_json(serde_json::json!({
            "avatar": serde_json::Value::Null
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "a_me",
            "avatar": null
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["actor", "update", "--no-avatar"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Profile updated successfully."));
}

#[tokio::test]
async fn test_actor_avatar_file_uploads_first_then_patches_id() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();
    let dir = tempdir().unwrap();
    let avatar_path = dir.path().join("avatar.png");
    let mut f = std::fs::File::create(&avatar_path).unwrap();
    f.write_all(b"fake-png-bytes").unwrap();
    drop(f);

    // 1. Önce yükleme ucu çağrılmalı; dönüşteki id PATCH'e gider.
    Mock::given(method("POST"))
        .and(path("/uploads"))
        .respond_with(ResponseTemplate::new(201).set_body_json(upload_response("f_uploaded")))
        .mount(&mock_server)
        .await;

    // 2. PATCH gövdesi, dosyanın değil yüklenen attachment id'sinin geçtiğini
    //    gösterir.
    Mock::given(method("PATCH"))
        .and(path("/actors/me"))
        .and(body_partial_json(
            serde_json::json!({ "avatar": "f_uploaded" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "a_me"
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["actor", "update", "--avatar", avatar_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Profile updated successfully."));
}

#[tokio::test]
async fn test_actor_avatar_attachment_id_patches_directly_no_upload() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // f_ id'si verildiğinde YÜKLEME YAPILMAZ; yalnızca PATCH gövdesinde
    // doğrudan gönderilir. Bu testte /uploads mock yok — eğer araç dosyayı
    // yüklemeye kalksaydı mock eşleşmez, 404 döner ve komut başarısız olurdu.
    Mock::given(method("PATCH"))
        .and(path("/actors/me"))
        .and(body_partial_json(serde_json::json!({ "avatar": "f_pre" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "a_me"
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["actor", "update", "--avatar", "f_pre"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Profile updated successfully."));
}

#[tokio::test]
async fn test_actor_avatar_and_no_avatar_conflict_is_usage_error() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["actor", "update", "--avatar", "f_x", "--no-avatar"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "Use either '--avatar' or '--no-avatar', not both.",
        ));
}
