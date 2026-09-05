//! CLI istemci katmanı.
//!
//! CLI'ın kendi HTTP/retry/rate-limit taşıma katmanı kaldırıldı (`src/client/`
//! silindi) ve tamamen resmi [`actos_sdk::Actos`] istemcisine geçildi. Bu modül
//! yalnızca SDK'yı saran ince bir adaptördür — CLI'a argüman ayrıştırma,
//! çıktı biçimlendirme ve TUI kalır.
//!
//! Tüm istekler SDK'nın `Transport`'u üzerinden gider; retry/backoff, 429
//! `Retry-After` uyumu ve `X-RateLimit-*` takibi SDK tarafından yönetilir.

use std::ops::Deref;
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::header::HeaderMap;
use serde_json::Value;

use crate::error::{CliError, RateLimitInfo};

/// Sunucu tarafı sayfa başına azami öğe sınırı (`PLAN.md` Faz 2).
pub const MAX_PAGE_SIZE: u32 = 100;

/// Resmi `actos` SDK istemcisini saran adaptör.
///
/// `Deref<Target = actos_sdk::Actos>` sayesinde komutlar doğrudan SDK'nın typed
/// resource builder'larına erişir (`client.posts().create(...)`, `client.auth()`,
/// vb.). `api_key()`, `base_url()`, `rate_limit()` SDK'ya delege eder.
#[derive(Clone, Debug)]
pub struct ApiClient {
    actos: actos_sdk::Actos,
    verbose: bool,
}

impl ApiClient {
    /// SDK istemcisini yapılandırılmış değerlerle oluşturur.
    ///
    /// `wait` ve `verbose` CLI uyumluluğu için kabuklanır: SDK 429'u
    /// `Retry-After` uyarınca zaten otomatik yeniden dener (`--wait` artık her
    /// zaman etkindi), `verbose` yalnızca istek satırını stderr'e basar.
    pub fn new(
        base_url: String,
        api_key: Option<String>,
        timeout_secs: u64,
        wait: bool,
        verbose: bool,
    ) -> Result<Self, CliError> {
        let builder = actos_sdk::Actos::builder()
            .base_url(base_url)
            .timeout(Duration::from_secs(timeout_secs));
        let builder = if let Some(key) = api_key {
            builder.api_key(key)
        } else {
            builder
        };
        let actos = builder.build().map_err(CliError::from)?;
        let _ = wait;
        Ok(Self { actos, verbose })
    }

    /// Yapılandırılmış API anahtarı varsa döner.
    #[must_use]
    pub fn api_key(&self) -> Option<&str> {
        self.actos.api_key()
    }

    /// Trailing slash'siz taban URL'yi döner.
    #[must_use]
    pub fn base_url(&self) -> String {
        self.actos
            .base_url()
            .as_str()
            .trim_end_matches('/')
            .to_string()
    }

    /// SDK'nın transport anlık görüntüsünden hız limiti bilgisini döner.
    #[must_use]
    pub fn rate_limit_info(&self) -> Option<RateLimitInfo> {
        self.actos.rate_limit().map(Into::into)
    }

    /// Ham GET isteği — yanıtı `Value` olarak döner (sayfalama/özet uçlarında
    /// raw JSON'u muhafaza etmek için kullanılır).
    pub async fn get_json(
        &self,
        path: &str,
        query: Option<&[(&str, &str)]>,
    ) -> Result<(Value, RateLimitInfo), CliError> {
        let (_status, _headers, bytes, rate_limit) = self
            .execute_request(reqwest::Method::GET, path, query, None, None)
            .await?;
        let val = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .map_err(|e| CliError::General(format!("Failed to parse JSON response: {e}")))?
        };
        Ok((val, rate_limit))
    }

    /// JSON gövdesiyle POST isteği — yanıtı `Value` olarak döner.
    pub async fn post_json(
        &self,
        path: &str,
        body: &Value,
        idempotency_key: Option<&str>,
    ) -> Result<(Value, RateLimitInfo), CliError> {
        let bytes_body = serde_json::to_vec(body)
            .map_err(|e| CliError::Validation(format!("Failed to serialize request JSON: {e}")))?;
        let (_status, _headers, bytes, rate_limit) = self
            .execute_request(
                reqwest::Method::POST,
                path,
                None,
                Some(bytes_body),
                idempotency_key,
            )
            .await?;
        let val = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .map_err(|e| CliError::General(format!("Failed to parse JSON response: {e}")))?
        };
        Ok((val, rate_limit))
    }

    /// SDK transport'u üzerinden ham bir istek çalıştırır (kaçış kapağı).
    ///
    /// HTTP metodolojisi, sorgu, gövde ve idempotency anahtarını SDK'nın
    /// `Actos::request` + `Transport::execute` zincirine bağlar; retry/rate-limit
    /// SDK tarafından uygulanır.
    pub async fn execute_request(
        &self,
        method: reqwest::Method,
        path: &str,
        query: Option<&[(&str, &str)]>,
        body: Option<Vec<u8>>,
        explicit_idempotency_key: Option<&str>,
    ) -> Result<(StatusCode, HeaderMap, Vec<u8>, RateLimitInfo), CliError> {
        let mut builder = self.actos.request(method.clone(), path);
        if let Some(q) = query {
            builder = builder.query(q);
        }
        if method == reqwest::Method::POST {
            let key = explicit_idempotency_key
                .map(ToString::to_string)
                .or_else(|| Some(uuid::Uuid::new_v4().to_string()));
            if let Some(ref k) = key {
                builder = builder.header("idempotency-key", k.clone());
            }
        }
        if let Some(bytes_body) = body {
            builder = builder
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(bytes_body);
        }

        if self.verbose {
            eprintln!("> {} {}", method, path);
            if self.actos.api_key().is_some() {
                eprintln!("> Authorization: Bearer ***");
            }
        }

        let res = self
            .actos
            .transport()
            .execute(builder)
            .await
            .map_err(CliError::from)?;

        let status = res.status();
        let headers = res.headers().clone();
        let bytes = res
            .bytes()
            .await
            .map_err(|e| CliError::Network(e.to_string()))?
            .to_vec();
        let rate_limit = self.rate_limit_info().unwrap_or_default();

        Ok((status, headers, bytes, rate_limit))
    }

    /// Şeffaf cursor takibiyle sayfalanmış istek yapar (`PLAN.md` Faz 2).
    ///
    /// Çok sayfalı uçlarda (`/feed`, `/tags`, `/actors`, ...) aynı eski
    /// davranışı korur: ham JSON yanıtlarını birleştirir, `next_cursor`'ı
    /// korur ve hedef limit aşılınca durur.
    pub async fn paginate(
        &self,
        path: &str,
        base_query: &[(&str, &str)],
        target_limit: u32,
        explicit_cursor: Option<&str>,
    ) -> Result<(Value, RateLimitInfo), CliError> {
        let mut accumulated = Value::Null;
        let mut current_cursor = explicit_cursor.map(ToString::to_string);
        let mut total_fetched: usize = 0;
        let limit_usize = target_limit as usize;

        loop {
            let page_chunk =
                (target_limit.saturating_sub(total_fetched as u32)).clamp(1, MAX_PAGE_SIZE);

            let mut query_params: Vec<(&str, String)> = base_query
                .iter()
                .map(|(k, v)| (*k, v.to_string()))
                .collect();
            query_params.push(("limit", page_chunk.to_string()));
            if let Some(ref c) = current_cursor {
                query_params.push(("cursor", c.clone()));
            }

            let query_refs: Vec<(&str, &str)> =
                query_params.iter().map(|(k, v)| (*k, v.as_str())).collect();

            let (page_val, rate_limit) = self.get_json(path, Some(&query_refs)).await?;

            // Kullanıcı tek sayfa (--cursor) istediyse döngüye devam etme
            if explicit_cursor.is_some() {
                return Ok((page_val, rate_limit));
            }

            let (new_count, next_cursor) = merge_page(&mut accumulated, page_val, limit_usize);

            total_fetched = new_count;

            if total_fetched >= limit_usize || next_cursor.is_none() {
                return Ok((accumulated, rate_limit));
            }

            current_cursor = next_cursor;
        }
    }
}

impl Deref for ApiClient {
    type Target = actos_sdk::Actos;

    fn deref(&self) -> &Self::Target {
        &self.actos
    }
}

/// Sayfalanmış JSON yanıtındaki ana dizi alan adını bulur.
#[must_use]
pub fn find_array_field_name(val: &Value) -> Option<String> {
    if let Some(map) = val.as_object() {
        let candidate_keys = [
            "items",
            "results",
            "posts",
            "comments",
            "actors",
            "tags",
            "reports",
            "keys",
            "votes",
            "saved",
            "notifications",
            "actions",
        ];

        for key in candidate_keys {
            if let Some(Value::Array(_)) = map.get(key) {
                return Some(key.to_string());
            }
        }

        for (k, v) in map {
            if let Value::Array(_) = v {
                return Some(k.clone());
            }
        }
    }
    None
}

/// Sayfalanmış yanıttan bir sonraki sayfa imlecini okur.
#[must_use]
pub fn extract_next_cursor(val: &Value) -> Option<String> {
    val.get("next_cursor")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(ToString::to_string)
}

/// İki sayfalanmış yanıtı şeffaf biçimde birleştirir.
///
/// `target_limit` öğe sayısına ulaşıldığında eklemeyi durdurur.
pub fn merge_page(
    accumulated: &mut Value,
    page: Value,
    target_limit: usize,
) -> (usize, Option<String>) {
    let next_cursor = extract_next_cursor(&page);

    if accumulated.is_null() {
        *accumulated = page;
        if let Some(key) = find_array_field_name(accumulated)
            && let Some(Value::Array(arr)) = accumulated.get_mut(&key)
        {
            if arr.len() > target_limit {
                arr.truncate(target_limit);
            }
            return (arr.len(), next_cursor);
        }
        return (0, next_cursor);
    }

    // İkinci veya sonraki sayfaları birleştir
    let page_items = if let Some(key) = find_array_field_name(&page)
        && let Some(Value::Array(arr)) = page.get(&key)
    {
        Some(arr.clone())
    } else {
        None
    };

    if let Some(key) = find_array_field_name(accumulated)
        && let Some(new_items) = page_items
        && let Some(Value::Array(acc_arr)) = accumulated.get_mut(&key)
    {
        let remaining_needed = target_limit.saturating_sub(acc_arr.len());
        let to_add = new_items.into_iter().take(remaining_needed);
        acc_arr.extend(to_add);
    }

    if let Some(map) = accumulated.as_object_mut() {
        if let Some(cur) = &next_cursor {
            map.insert("next_cursor".to_string(), Value::String(cur.clone()));
        } else {
            map.insert("next_cursor".to_string(), Value::Null);
        }
    }

    let current_count = find_array_field_name(accumulated)
        .and_then(|k| accumulated.get(&k))
        .and_then(|v| v.as_array())
        .map_or(0, Vec::len);

    (current_count, next_cursor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_next_cursor() {
        let val = json!({ "next_cursor": "cursor_123" });
        assert_eq!(extract_next_cursor(&val), Some("cursor_123".to_string()));

        let empty = json!({ "next_cursor": null });
        assert_eq!(extract_next_cursor(&empty), None);
    }

    #[test]
    fn test_merge_page() {
        let page1 = json!({
            "posts": [{"id": 1}, {"id": 2}],
            "next_cursor": "c2"
        });
        let page2 = json!({
            "posts": [{"id": 3}, {"id": 4}],
            "next_cursor": null
        });

        let mut acc = Value::Null;
        let (count1, cursor1) = merge_page(&mut acc, page1, 3);
        assert_eq!(count1, 2);
        assert_eq!(cursor1, Some("c2".to_string()));

        let (count2, cursor2) = merge_page(&mut acc, page2, 3);
        assert_eq!(count2, 3);
        assert_eq!(cursor2, None);

        let posts = acc["posts"].as_array().unwrap();
        assert_eq!(posts.len(), 3);
        assert_eq!(posts[0]["id"], 1);
        assert_eq!(posts[1]["id"], 2);
        assert_eq!(posts[2]["id"], 3);
    }
}
