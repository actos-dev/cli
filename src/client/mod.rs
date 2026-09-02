pub mod idempotency;
pub mod pagination;
pub mod ratelimit;
pub mod retry;

use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use reqwest::{Method, StatusCode};
use serde_json::Value;
use std::time::Duration;

use crate::error::CliError;
use ratelimit::RateLimitInfo;

/// Actos platformu için HTTP istemcisi (`PLAN.md` Faz 2).
#[derive(Debug, Clone)]
pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
    wait: bool,
    verbose: bool,
}

impl ApiClient {
    /// Yeni bir `ApiClient` oluşturur.
    pub fn new(
        base_url: String,
        api_key: Option<String>,
        timeout_secs: u64,
        wait: bool,
        verbose: bool,
    ) -> Result<Self, CliError> {
        let user_agent = format!("actos-cli/{}", env!("CARGO_PKG_VERSION"));
        let mut default_headers = HeaderMap::new();
        default_headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&user_agent)
                .map_err(|e| CliError::Validation(format!("Invalid User-Agent header: {e}")))?,
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .default_headers(default_headers)
            .build()
            .map_err(|e| CliError::General(format!("Failed to build HTTP client: {e}")))?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            wait,
            verbose,
        })
    }

    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    #[must_use]
    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    /// Bir HTTP isteğini yeniden deneme, hız sınırı ve idempotency kurallarına göre çalıştırır.
    pub async fn execute_request(
        &self,
        method: Method,
        path: &str,
        query: Option<&[(&str, &str)]>,
        body: Option<Vec<u8>>,
        explicit_idempotency_key: Option<&str>,
    ) -> Result<(StatusCode, HeaderMap, Vec<u8>, RateLimitInfo), CliError> {
        let url_str = if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            format!("{}/{}", self.base_url, path.trim_start_matches('/'))
        };

        let mut url = reqwest::Url::parse(&url_str)
            .map_err(|e| CliError::Validation(format!("Invalid URL '{url_str}': {e}")))?;

        if let Some(q) = query {
            let mut query_pairs = url.query_pairs_mut();
            for (k, v) in q {
                query_pairs.append_pair(k, v);
            }
        }

        let has_idempotency_key = explicit_idempotency_key.is_some()
            || (method == Method::POST && !self.is_read_only_path(path));

        let idempotency_key = explicit_idempotency_key
            .map(ToString::to_string)
            .or_else(|| {
                if method == Method::POST && !self.is_read_only_path(path) {
                    Some(idempotency::generate_idempotency_key())
                } else {
                    None
                }
            });

        let mut attempt = 0;

        loop {
            let mut req_builder = self.client.request(method.clone(), url.clone());

            // Kimlik başlığı enjeksiyonu
            if let Some(ref key) = self.api_key {
                req_builder = req_builder.header(AUTHORIZATION, format!("Bearer {key}"));
            }

            // Idempotency anahtarı enjeksiyonu
            if let Some(ref ikey) = idempotency_key {
                req_builder = req_builder.header(idempotency::IDEMPOTENCY_KEY_HEADER, ikey);
            }

            // Gövde ekleme
            if let Some(ref b) = body {
                req_builder = req_builder
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .body(b.clone());
            }

            // Verbose loglama (stderr'e, anahtar maskelenerek)
            if self.verbose {
                eprintln!("> {} {}", method, url);
                if self.api_key.is_some() {
                    eprintln!("> Authorization: Bearer [REDACTED]");
                }
                if let Some(ref ikey) = idempotency_key {
                    eprintln!("> Idempotency-Key: {ikey}");
                }
            }

            match req_builder.send().await {
                Ok(response) => {
                    let status = response.status();
                    let headers = response.headers().clone();
                    let rate_limit = RateLimitInfo::from_headers(&headers);

                    if self.verbose {
                        eprintln!(
                            "< {} {}",
                            status.as_u16(),
                            status.canonical_reason().unwrap_or("")
                        );
                        for (name, val) in &headers {
                            if let Ok(v_str) = val.to_str() {
                                eprintln!("< {name}: {v_str}");
                            }
                        }
                    }

                    // 429 Hız sınırı yönetimi
                    if status == StatusCode::TOO_MANY_REQUESTS {
                        let retry_after = ratelimit::extract_retry_after(&headers);

                        if self.wait {
                            ratelimit::wait_for_retry(retry_after).await;
                            continue;
                        }

                        let bytes = response
                            .bytes()
                            .await
                            .map_or_else(|_| Vec::new(), |b| b.to_vec());
                        return Err(CliError::from_http_response(status, &headers, &bytes));
                    }

                    // 2xx Başarılı yanıt
                    if status.is_success() {
                        let bytes = response
                            .bytes()
                            .await
                            .map_or_else(|_| Vec::new(), |b| b.to_vec());
                        return Ok((status, headers, bytes, rate_limit));
                    }

                    // 4xx / 5xx Hata durumları
                    let bytes = response
                        .bytes()
                        .await
                        .map_or_else(|_| Vec::new(), |b| b.to_vec());
                    let cli_err = CliError::from_http_response(status, &headers, &bytes);

                    // Yeniden deneme kontrolü
                    if retry::should_retry(&method, has_idempotency_key, Some(status), false)
                        && attempt < retry::MAX_RETRIES
                    {
                        attempt += 1;
                        let backoff = retry::calculate_backoff(attempt);
                        if self.verbose {
                            eprintln!(
                                "Retrying after 5xx error in {:?} (attempt {}/{})",
                                backoff,
                                attempt,
                                retry::MAX_RETRIES
                            );
                        }
                        tokio::time::sleep(backoff).await;
                        continue;
                    }

                    return Err(cli_err);
                }

                Err(req_err) => {
                    if self.verbose {
                        eprintln!("! Request failed: {req_err}");
                    }

                    let cli_err = CliError::Network(req_err.to_string());

                    if retry::should_retry(&method, has_idempotency_key, None, true)
                        && attempt < retry::MAX_RETRIES
                    {
                        attempt += 1;
                        let backoff = retry::calculate_backoff(attempt);
                        if self.verbose {
                            eprintln!(
                                "Retrying after network error in {:?} (attempt {}/{})",
                                backoff,
                                attempt,
                                retry::MAX_RETRIES
                            );
                        }
                        tokio::time::sleep(backoff).await;
                        continue;
                    }

                    return Err(cli_err);
                }
            }
        }
    }

    /// JSON yanıtı dönecek GET isteği yapar.
    pub async fn get_json(
        &self,
        path: &str,
        query: Option<&[(&str, &str)]>,
    ) -> Result<(Value, RateLimitInfo), CliError> {
        let (_status, _headers, bytes, rate_limit) = self
            .execute_request(Method::GET, path, query, None, None)
            .await?;

        let val: Value = serde_json::from_slice(&bytes)
            .map_err(|e| CliError::General(format!("Failed to parse JSON response: {e}")))?;

        Ok((val, rate_limit))
    }

    /// JSON gövdesiyle POST isteği yapar.
    pub async fn post_json(
        &self,
        path: &str,
        body: &Value,
        idempotency_key: Option<&str>,
    ) -> Result<(Value, RateLimitInfo), CliError> {
        let bytes_body = serde_json::to_vec(body)
            .map_err(|e| CliError::Validation(format!("Failed to serialize request JSON: {e}")))?;

        let (_status, _headers, bytes, rate_limit) = self
            .execute_request(Method::POST, path, None, Some(bytes_body), idempotency_key)
            .await?;

        let val: Value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .map_err(|e| CliError::General(format!("Failed to parse JSON response: {e}")))?
        };

        Ok((val, rate_limit))
    }

    /// Şeffaf cursor takibiyle sayfalanmış istek yapar (`PLAN.md` Faz 2).
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
            let page_chunk = (target_limit.saturating_sub(total_fetched as u32))
                .clamp(1, pagination::MAX_PAGE_SIZE);

            let page_chunk_str = page_chunk.to_string();
            let mut query_params: Vec<(&str, &str)> = base_query.to_vec();
            query_params.push(("limit", &page_chunk_str));

            let cur_holder;
            if let Some(ref c) = current_cursor {
                cur_holder = c.clone();
                query_params.push(("cursor", &cur_holder));
            }

            let (page_val, rate_limit) = self.get_json(path, Some(&query_params)).await?;

            // Eğer kullanıcı tek sayfa (--cursor) istediyse döngüye devam etme
            if explicit_cursor.is_some() {
                return Ok((page_val, rate_limit));
            }

            let (new_count, next_cursor) =
                pagination::merge_page(&mut accumulated, page_val, limit_usize);

            total_fetched = new_count;

            if total_fetched >= limit_usize || next_cursor.is_none() {
                return Ok((accumulated, rate_limit));
            }

            current_cursor = next_cursor;
        }
    }

    const fn is_read_only_path(&self, _path: &str) -> bool {
        false
    }
}
