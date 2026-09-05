//! `actos watch` — JSONL akışı ve dedupe ile ilgili sözleşme testleri.
//!
//! `watch` sonsuz bir yoklama döngüsüdür (Ctrl-C ile durur), bu yüzden binary
//! üzerinden doğrudan uçmak pratik değildir. Döngü gövdesi `poll_once` ve
//! sınırlı `scan_polls` yardımcılarına ayrılmıştır; testler bu iki fonksiyonu
//! uçurarak aynı sözleşmeyi spin-forever olmadan doğrular.
use std::collections::HashSet;

use actos::client::ApiClient;
use actos::commands::watch::{poll_once, scan_polls};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client_for(base_url: &str) -> ApiClient {
    ApiClient::new(
        base_url.to_string(),
        Some("actos_test_key".to_string()),
        10,
        false,
        false,
    )
    .unwrap_or_else(|e| panic!("could not build client: {e}"))
}

fn notif(id: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "kind": "reply_to_comment",
        "actor": {"id": "a_u2", "username": "bob", "actor_type": "human"},
        "target_type": "content",
        "target_id": "c_123",
        "payload": {},
        "created_at": "2026-09-02T12:00:00Z",
        "read_at": null
    })
}

fn inbox_body(notifications: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "notifications": notifications,
        "next_cursor": null,
        "unread_count": 1
    })
}

#[tokio::test]
async fn test_watch_poll_emits_one_json_line_per_notification() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me/inbox"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(inbox_body(serde_json::json!([notif("n_1"), notif("n_2"),]))),
        )
        .mount(&mock_server)
        .await;

    let client = client_for(&mock_server.uri());
    let mut seen = HashSet::new();
    let mut buf = Vec::new();
    poll_once(&client, &[("limit", "25")], &mut seen, &mut buf)
        .await
        .expect("poll should succeed");

    let text = String::from_utf8(buf).expect("output is UTF-8");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "one JSON line per notification");

    for line in lines {
        let v: serde_json::Value =
            serde_json::from_str(line).expect("each watch line must be valid JSON");
        assert!(
            v["id"].is_string(),
            "JSONL line carries the notification id"
        );
    }

    // Aynı yoklamada yeni olan iki id de seen'e işlenmiş olmalı.
    assert_eq!(seen.len(), 2);
    assert!(seen.contains("n_1") && seen.contains("n_2"));
}

#[tokio::test]
async fn test_watch_dedupes_same_notification_across_polls() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me/inbox"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(inbox_body(serde_json::json!([notif("n_same"),]))),
        )
        .mount(&mock_server)
        .await;

    let client = client_for(&mock_server.uri());
    let lines = scan_polls(&client, &[("limit", "25")], 3)
        .await
        .expect("multiple polls should succeed");

    // Aynı id her yoklamada tekrar gelse de yalnızca ilk yoklama bastırmalı.
    assert_eq!(
        lines.len(),
        1,
        "later polls with no new notifications emit nothing"
    );
    assert_eq!(lines[0].lines().count(), 1);
}

#[tokio::test]
async fn test_watch_streams_new_notification_on_later_poll() {
    let mock_server = MockServer::start().await;

    // wiremock, eşleşen stub'lar arasından mount edilme SIRASI (FIFO) ile dener ve
    // `up_to_n_times` tükenen stub'ı atlayıp bir sonrakine düşer:
    //   1. yoklama -> ilk mount edilen `n_first` (bir kez kullanılır);
    //   2. yoklama -> `n_first` tükendiği için `n_later`'a düşer.
    Mock::given(method("GET"))
        .and(path("/me/inbox"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(inbox_body(serde_json::json!([notif("n_first"),]))),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/me/inbox"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(inbox_body(serde_json::json!([notif("n_later"),]))),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    let client = client_for(&mock_server.uri());
    let lines = scan_polls(&client, &[("limit", "25")], 2)
        .await
        .expect("polls should succeed");

    // 1. yoklama: n_first; 2. yoklama: n_later (yeni). n_first 2. yoklamada
    // tekrar dönmediği için çıktı sırası ikisini de içermeli.
    let emitted_ids: Vec<String> = lines
        .iter()
        .flat_map(|b| b.lines())
        .map(|l| {
            let v: serde_json::Value =
                serde_json::from_str::<serde_json::Value>(l).expect("valid JSONL");
            v.get("id")
                .and_then(serde_json::Value::as_str)
                .expect("id present")
                .to_string()
        })
        .collect();

    assert_eq!(
        emitted_ids,
        vec!["n_first".to_string(), "n_later".to_string()]
    );
    assert_eq!(lines.len(), 2, "watch keeps polling and streaming");
}

#[tokio::test]
async fn test_watch_aborts_on_http_error() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me/inbox"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "type": "about:blank",
            "status": 400,
            "detail": "boom"
        })))
        .mount(&mock_server)
        .await;

    let client = client_for(&mock_server.uri());
    let err = poll_once(
        &client,
        &[("limit", "25")],
        &mut HashSet::new(),
        &mut Vec::<u8>::new(),
    )
    .await
    .expect_err("400 must abort the polling loop");

    // `watch stopped:` sarmalayıcısı döngünün kalıcı bir hatada durduğunu
    // taşır; 4xx yeniden denemez, böylece döngü sonlanır.
    assert!(
        err.to_string().contains("watch stopped"),
        "expected abort message, got: {err}"
    );
}
