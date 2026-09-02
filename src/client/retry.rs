use reqwest::{Method, StatusCode};
use std::time::Duration;

/// En fazla deneme sayısı (ilk istek + 3 yeniden deneme = 4 istek).
pub const MAX_RETRIES: u32 = 3;

/// İsteğin idempotent olup olmadığını belirler.
///
/// - `GET`, `HEAD`, `OPTIONS` istekleri doğası gereği idempotenttir.
/// - `POST`, `PUT`, `PATCH`, `DELETE` istekleri YALNIZCA `Idempotency-Key` başlığı varsa idempotenttir.
#[must_use]
pub fn is_request_idempotent(method: &Method, has_idempotency_key: bool) -> bool {
    if method == Method::GET || method == Method::HEAD || method == Method::OPTIONS {
        true
    } else {
        has_idempotency_key
    }
}

/// Verilen durum ve hata türüne göre isteğin yeniden denenip denenmeyeceğini belirler.
///
/// Kurallar (`PLAN.md` Faz 2 ve Ajan Sözleşmesi §2 kural 8):
/// - 4xx hataları **ASLA** yeniden denenmez.
/// - 5xx sunucu hataları ve ağ hataları yeniden denenir; **ANCAK** idempotency anahtarı
///   olmayan yazma istekleri 5xx durumunda **asla yeniden denenmez** (sunucuya ulaşmış olabilir).
#[must_use]
pub fn should_retry(
    method: &Method,
    has_idempotency_key: bool,
    status: Option<StatusCode>,
    is_network_error: bool,
) -> bool {
    let idempotent = is_request_idempotent(method, has_idempotency_key);

    if is_network_error {
        return idempotent;
    }

    if let Some(status_code) = status {
        if status_code.is_client_error() {
            // 4xx asla yeniden denenmez
            return false;
        }

        if status_code.is_server_error() {
            // 5xx yalnızca istek idempotent ise yeniden denenir
            return idempotent;
        }
    }

    false
}

/// Jitter eklenmiş üstel geri çekilme (exponential backoff) süresini hesaplar.
#[must_use]
pub fn calculate_backoff(attempt: u32) -> Duration {
    let base_ms = 100u64.saturating_mul(1u64 << attempt.min(5));
    // 0..50 ms arası pseudo-jitter
    let jitter_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos() as u64)
        % 50;
    Duration::from_millis(base_ms + jitter_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idempotence_rules() {
        assert!(is_request_idempotent(&Method::GET, false));
        assert!(is_request_idempotent(&Method::HEAD, false));
        assert!(!is_request_idempotent(&Method::POST, false));
        assert!(is_request_idempotent(&Method::POST, true));
        assert!(!is_request_idempotent(&Method::PUT, false));
        assert!(is_request_idempotent(&Method::PUT, true));
        assert!(!is_request_idempotent(&Method::DELETE, false));
        assert!(is_request_idempotent(&Method::DELETE, true));
    }

    #[test]
    fn test_retry_rules() {
        // GET 500 -> retry
        assert!(should_retry(
            &Method::GET,
            false,
            Some(StatusCode::INTERNAL_SERVER_ERROR),
            false
        ));

        // GET 502 -> retry
        assert!(should_retry(
            &Method::GET,
            false,
            Some(StatusCode::BAD_GATEWAY),
            false
        ));

        // POST without key on 500 -> DO NOT retry
        assert!(!should_retry(
            &Method::POST,
            false,
            Some(StatusCode::INTERNAL_SERVER_ERROR),
            false
        ));

        // POST with key on 500 -> retry
        assert!(should_retry(
            &Method::POST,
            true,
            Some(StatusCode::INTERNAL_SERVER_ERROR),
            false
        ));

        // POST with key on 400/401/404 -> NEVER retry
        assert!(!should_retry(
            &Method::POST,
            true,
            Some(StatusCode::BAD_REQUEST),
            false
        ));
        assert!(!should_retry(
            &Method::POST,
            true,
            Some(StatusCode::UNAUTHORIZED),
            false
        ));
        assert!(!should_retry(
            &Method::POST,
            true,
            Some(StatusCode::NOT_FOUND),
            false
        ));

        // Network error on idempotent request -> retry
        assert!(should_retry(&Method::GET, false, None, true));
        assert!(should_retry(&Method::POST, true, None, true));

        // Network error on non-idempotent write -> do not retry
        assert!(!should_retry(&Method::POST, false, None, true));
    }

    #[test]
    fn test_backoff_calculation() {
        let b0 = calculate_backoff(0);
        let b1 = calculate_backoff(1);
        assert!(b0.as_millis() >= 100);
        assert!(b1.as_millis() >= 200);
    }
}
