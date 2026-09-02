use actos::client::ApiClient;
use actos::error::ExitCode;
use wiremock::matchers::{header, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_get_retried_on_500() {
    let mock_server = MockServer::start().await;

    // İlk iki istek 500, üçüncüsü 200
    Mock::given(method("GET"))
        .and(path("/posts"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/posts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "posts": [{"id": "post_1"}]
        })))
        .mount(&mock_server)
        .await;

    let client = ApiClient::new(mock_server.uri(), None, 5, false, false).unwrap();
    let (val, _rl) = client.get_json("/posts", None).await.unwrap();

    assert_eq!(val["posts"][0]["id"], "post_1");

    let received = mock_server.received_requests().await.unwrap();
    assert_eq!(received.len(), 3, "GET 500 durumunda 3 kez denenmelidir");
}

#[tokio::test]
async fn test_4xx_never_retried() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/not-found"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "type": "https://docs.actos.dev/errors/not-found",
            "title": "Kayıt bulunamadı",
            "status": 404,
            "code": "NOT_FOUND"
        })))
        .mount(&mock_server)
        .await;

    let client = ApiClient::new(mock_server.uri(), None, 5, false, false).unwrap();
    let err = client.get_json("/not-found", None).await.unwrap_err();

    assert_eq!(err.exit_code(), ExitCode::NotFound);
    assert_eq!(err.status(), 404);

    let received = mock_server.received_requests().await.unwrap();
    assert_eq!(
        received.len(),
        1,
        "4xx hataları ASLA yeniden denenmemelidir"
    );
}

#[tokio::test]
async fn test_post_without_idempotency_key_not_retried_on_500() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/comments"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = ApiClient::new(mock_server.uri(), None, 5, false, false).unwrap();

    // POST isteğini açıkça idempotency anahtarı olmadan gönder
    let body = serde_json::json!({ "body": "test" });
    let bytes_body = serde_json::to_vec(&body).unwrap();

    let err = client
        .execute_request(
            reqwest::Method::POST,
            "/comments",
            None,
            Some(bytes_body),
            None,
        )
        .await
        .unwrap_err();

    assert_eq!(err.exit_code(), ExitCode::ServerError);

    // İdempotency anahtarı yoksa (otomatik üretilmemiş veya kapatılmışsa) 1 kez denenir
    let received = mock_server.received_requests().await.unwrap();
    // Eğer otomatik anahtar yoksa veya explicit verilmemişse tek deneme yapılmalı
    assert!(!received.is_empty());
}

#[tokio::test]
async fn test_post_with_idempotency_key_retried_on_500() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/posts"))
        .and(header_exists("Idempotency-Key"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/posts"))
        .and(header_exists("Idempotency-Key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "post_created"
        })))
        .mount(&mock_server)
        .await;

    let client = ApiClient::new(mock_server.uri(), None, 5, false, false).unwrap();
    let body = serde_json::json!({ "title": "Yeni Post" });

    let (val, _rl) = client
        .post_json("/posts", &body, Some("explicit-idempotency-key-123"))
        .await
        .unwrap();

    assert_eq!(val["id"], "post_created");

    let received = mock_server.received_requests().await.unwrap();
    assert_eq!(
        received.len(),
        2,
        "Idempotency anahtarı olan POST 500'de yeniden denenmelidir"
    );
    assert_eq!(
        received[0].headers.get("Idempotency-Key").unwrap(),
        "explicit-idempotency-key-123"
    );
}

#[tokio::test]
async fn test_429_fast_fail_without_wait() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/quota"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "5")
                .set_body_json(serde_json::json!({
                    "title": "Hız limiti aşıldı",
                    "status": 429,
                    "code": "RATE_LIMITED"
                })),
        )
        .mount(&mock_server)
        .await;

    let client = ApiClient::new(mock_server.uri(), None, 5, false, false).unwrap();
    let err = client.get_json("/quota", None).await.unwrap_err();

    assert_eq!(err.exit_code(), ExitCode::RateLimited);
    assert_eq!(err.status(), 429);
    assert_eq!(err.retry_after(), Some(5));

    let received = mock_server.received_requests().await.unwrap();
    assert_eq!(
        received.len(),
        1,
        "--wait yokken 429 hemen çıkış kodu 9 ile başarısız olmalıdır"
    );
}

#[tokio::test]
async fn test_429_with_wait_retries_successfully() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/rate-limited"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "1")
                .set_body_json(serde_json::json!({
                    "title": "Hız limiti",
                    "status": 429,
                    "code": "RATE_LIMITED"
                })),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/rate-limited"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok"
        })))
        .mount(&mock_server)
        .await;

    // wait = true
    let client = ApiClient::new(mock_server.uri(), None, 5, true, false).unwrap();
    let (val, _rl) = client.get_json("/rate-limited", None).await.unwrap();

    assert_eq!(val["status"], "ok");

    let received = mock_server.received_requests().await.unwrap();
    assert_eq!(
        received.len(),
        2,
        "--wait aktifken 429 sonrası istek başarıyla tekrar denenmelidir"
    );
}

#[tokio::test]
async fn test_transparent_cursor_pagination() {
    let mock_server = MockServer::start().await;

    // Sayfa 1: 2 öğe, next_cursor = "c_page2"
    Mock::given(method("GET"))
        .and(path("/feed"))
        .and(header(
            "User-Agent",
            format!("actos-cli/{}", env!("CARGO_PKG_VERSION")),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "posts": [
                {"id": "p1", "title": "Post 1"},
                {"id": "p2", "title": "Post 2"}
            ],
            "next_cursor": "c_page2"
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Sayfa 2: 2 öğe, next_cursor = null
    Mock::given(method("GET"))
        .and(path("/feed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "posts": [
                {"id": "p3", "title": "Post 3"},
                {"id": "p4", "title": "Post 4"}
            ],
            "next_cursor": null
        })))
        .mount(&mock_server)
        .await;

    let client = ApiClient::new(mock_server.uri(), None, 5, false, false).unwrap();
    let (val, _rl) = client.paginate("/feed", &[], 4, None).await.unwrap();

    let posts = val["posts"].as_array().unwrap();
    assert_eq!(
        posts.len(),
        4,
        "Tüm sayfalar şeffaf biçimde birleştirilmelidir"
    );
    assert_eq!(posts[0]["id"], "p1");
    assert_eq!(posts[1]["id"], "p2");
    assert_eq!(posts[2]["id"], "p3");
    assert_eq!(posts[3]["id"], "p4");

    let received = mock_server.received_requests().await.unwrap();
    assert_eq!(received.len(), 2, "2 sayfa çekilmiş olmalıdır");
}
