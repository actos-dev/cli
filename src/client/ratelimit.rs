use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const RATE_LIMIT_HEADER: &str = "x-ratelimit-limit";
pub const RATE_REMAINING_HEADER: &str = "x-ratelimit-remaining";
pub const RATE_RESET_HEADER: &str = "x-ratelimit-reset";

/// API yanıtlarından toplanan hız sınırı bilgileri.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RateLimitInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset: Option<u64>,
}

impl RateLimitInfo {
    /// HTTP başlıklarından hız sınırı bilgilerini okur.
    #[must_use]
    pub fn from_headers(headers: &HeaderMap) -> Self {
        let limit = headers
            .get(RATE_LIMIT_HEADER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        let remaining = headers
            .get(RATE_REMAINING_HEADER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        let reset = headers
            .get(RATE_RESET_HEADER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        Self {
            limit,
            remaining,
            reset,
        }
    }
}

/// `Retry-After` başlığını saniye cinsinden ayrıştırır (varsayılan: 1 saniye).
#[must_use]
pub fn extract_retry_after(headers: &HeaderMap) -> u64 {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1)
        .max(1)
}

/// 429 yanıtında `--wait` bayrağı aktifse stderr'e bilgi vererek bekler.
pub async fn wait_for_retry(retry_after_secs: u64) {
    eprintln!("Rate limit reached (429). Waiting {retry_after_secs}s before retrying...");
    tokio::time::sleep(Duration::from_secs(retry_after_secs)).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    #[test]
    fn test_rate_limit_info_from_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(RATE_LIMIT_HEADER, HeaderValue::from_static("100"));
        headers.insert(RATE_REMAINING_HEADER, HeaderValue::from_static("42"));
        headers.insert(RATE_RESET_HEADER, HeaderValue::from_static("1700000000"));

        let info = RateLimitInfo::from_headers(&headers);
        assert_eq!(info.limit, Some(100));
        assert_eq!(info.remaining, Some(42));
        assert_eq!(info.reset, Some(1700000000));
    }

    #[test]
    fn test_extract_retry_after() {
        let mut headers = HeaderMap::new();
        headers.insert(reqwest::header::RETRY_AFTER, HeaderValue::from_static("5"));
        assert_eq!(extract_retry_after(&headers), 5);

        let empty = HeaderMap::new();
        assert_eq!(extract_retry_after(&empty), 1);
    }
}
