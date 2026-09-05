use reqwest::StatusCode;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Unix 0anından sonraki saniye cinsinden hız sınırı penceresi sıfırlanma bilgisi.
///
/// Alanlar `Option`'dur çünkü sunucu `X-RateLimit-*` başlıklarını her zaman
/// göndermeyebilir; SDK bu bilgiyi [`actos_sdk::RateLimit`] olarak taşır ve
/// bu türe eşlenir.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimitInfo {
    /// Pencerede izin verilen toplam istek sayısı.
    pub limit: Option<u32>,
    /// Pencerede kalan istek sayısı.
    pub remaining: Option<u32>,
    /// Pencere sıfırlanana kadar geçen saniye (epoch-reset zaman damgası).
    pub reset: Option<u64>,
}

impl From<actos_sdk::RateLimit> for RateLimitInfo {
    fn from(rl: actos_sdk::RateLimit) -> Self {
        Self {
            limit: Some(rl.limit),
            remaining: Some(rl.remaining),
            reset: Some(rl.reset),
        }
    }
}

/// CLI çıkış kodları (`PLAN.md` §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ExitCode {
    /// Başarı
    Success = 0,
    /// Sınıflandırılamayan genel hata
    GeneralError = 1,
    /// Kullanım hatası (bayrak, eksik argüman, TTY'siz onay)
    UsageError = 2,
    /// Kimlik doğrulama başarısız (401, INVALID_KEY, MISSING_CREDENTIALS)
    AuthFailed = 3,
    /// Yetki yok (403, FORBIDDEN, BANNED)
    Forbidden = 4,
    /// Bulunamadı (404, NOT_FOUND)
    NotFound = 5,
    /// Silinmiş içerik (410, GONE)
    Gone = 6,
    /// Çakışma (409, CONFLICT)
    Conflict = 7,
    /// Doğrulama hatası (400, 415, VALIDATION_FAILED, INVALID_CURSOR, UNSUPPORTED_MEDIA)
    ValidationError = 8,
    /// Hız sınırı (429, RATE_LIMITED)
    RateLimited = 9,
    /// Sunucu hatası (5xx, INTERNAL)
    ServerError = 10,
    /// Ağ / bağlanılamadı (taşıma katmanı)
    NetworkError = 11,
}

impl ExitCode {
    #[must_use]
    pub const fn as_i32(self) -> i32 {
        self as i32
    }
}

impl From<actos_sdk::actos_types::ErrorCode> for ExitCode {
    fn from(code: actos_sdk::actos_types::ErrorCode) -> Self {
        match code {
            actos_sdk::actos_types::ErrorCode::ValidationFailed
            | actos_sdk::actos_types::ErrorCode::InvalidCursor
            | actos_sdk::actos_types::ErrorCode::UnsupportedMedia => Self::ValidationError,
            actos_sdk::actos_types::ErrorCode::MissingCredentials
            | actos_sdk::actos_types::ErrorCode::InvalidKey => Self::AuthFailed,
            actos_sdk::actos_types::ErrorCode::Forbidden
            | actos_sdk::actos_types::ErrorCode::Banned => Self::Forbidden,
            actos_sdk::actos_types::ErrorCode::NotFound => Self::NotFound,
            actos_sdk::actos_types::ErrorCode::Gone => Self::Gone,
            actos_sdk::actos_types::ErrorCode::Conflict => Self::Conflict,
            actos_sdk::actos_types::ErrorCode::RateLimited => Self::RateLimited,
            actos_sdk::actos_types::ErrorCode::Internal => Self::ServerError,
        }
    }
}

/// RFC 9457 Problem Details gövdesi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProblemDetails {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub type_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<actos_sdk::actos_types::ErrorCode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// CLI çalışma zamanı hataları.
#[derive(Debug)]
pub enum CliError {
    Usage(String),
    Auth(String),
    Forbidden(String),
    NotFound(String),
    Gone(String),
    Conflict(String),
    Validation(String),
    RateLimited {
        message: String,
        retry_after: Option<u64>,
        request_id: Option<String>,
    },
    Server(String),
    Network(String),
    General(String),
    Io(String),
    Api {
        code: actos_sdk::actos_types::ErrorCode,
        message: String,
        status: u16,
        request_id: Option<String>,
        retry_after: Option<u64>,
    },
}

impl CliError {
    #[must_use]
    pub const fn exit_code(&self) -> ExitCode {
        match self {
            Self::Usage(_) => ExitCode::UsageError,
            Self::Auth(_) => ExitCode::AuthFailed,
            Self::Forbidden(_) => ExitCode::Forbidden,
            Self::NotFound(_) => ExitCode::NotFound,
            Self::Gone(_) => ExitCode::Gone,
            Self::Conflict(_) => ExitCode::Conflict,
            Self::Validation(_) => ExitCode::ValidationError,
            Self::RateLimited { .. } => ExitCode::RateLimited,
            Self::Server(_) => ExitCode::ServerError,
            Self::Network(_) => ExitCode::NetworkError,
            Self::General(_) | Self::Io(_) => ExitCode::GeneralError,
            Self::Api { code, .. } => match code {
                actos_sdk::actos_types::ErrorCode::ValidationFailed
                | actos_sdk::actos_types::ErrorCode::InvalidCursor
                | actos_sdk::actos_types::ErrorCode::UnsupportedMedia => ExitCode::ValidationError,
                actos_sdk::actos_types::ErrorCode::MissingCredentials
                | actos_sdk::actos_types::ErrorCode::InvalidKey => ExitCode::AuthFailed,
                actos_sdk::actos_types::ErrorCode::Forbidden
                | actos_sdk::actos_types::ErrorCode::Banned => ExitCode::Forbidden,
                actos_sdk::actos_types::ErrorCode::NotFound => ExitCode::NotFound,
                actos_sdk::actos_types::ErrorCode::Gone => ExitCode::Gone,
                actos_sdk::actos_types::ErrorCode::Conflict => ExitCode::Conflict,
                actos_sdk::actos_types::ErrorCode::RateLimited => ExitCode::RateLimited,
                actos_sdk::actos_types::ErrorCode::Internal => ExitCode::ServerError,
            },
        }
    }

    #[must_use]
    pub const fn error_code(&self) -> actos_sdk::actos_types::ErrorCode {
        match self {
            Self::Usage(_) | Self::Validation(_) => {
                actos_sdk::actos_types::ErrorCode::ValidationFailed
            }
            Self::Auth(_) => actos_sdk::actos_types::ErrorCode::MissingCredentials,
            Self::Forbidden(_) => actos_sdk::actos_types::ErrorCode::Forbidden,
            Self::NotFound(_) => actos_sdk::actos_types::ErrorCode::NotFound,
            Self::Gone(_) => actos_sdk::actos_types::ErrorCode::Gone,
            Self::Conflict(_) => actos_sdk::actos_types::ErrorCode::Conflict,
            Self::RateLimited { .. } => actos_sdk::actos_types::ErrorCode::RateLimited,
            Self::Server(_) | Self::Network(_) | Self::General(_) | Self::Io(_) => {
                actos_sdk::actos_types::ErrorCode::Internal
            }
            Self::Api { code, .. } => *code,
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::Usage(m)
            | Self::Auth(m)
            | Self::Forbidden(m)
            | Self::NotFound(m)
            | Self::Gone(m)
            | Self::Conflict(m)
            | Self::Validation(m)
            | Self::Server(m)
            | Self::Network(m)
            | Self::General(m)
            | Self::Io(m) => m.as_str(),
            Self::RateLimited { message, .. } | Self::Api { message, .. } => message.as_str(),
        }
    }

    #[must_use]
    pub const fn status(&self) -> u16 {
        match self {
            Self::Api { status, .. } => *status,
            _ => self.error_code().http_status(),
        }
    }

    #[must_use]
    pub fn request_id(&self) -> Option<&str> {
        match self {
            Self::RateLimited { request_id, .. } | Self::Api { request_id, .. } => {
                request_id.as_deref()
            }
            _ => None,
        }
    }

    #[must_use]
    pub const fn retry_after(&self) -> Option<u64> {
        match self {
            Self::RateLimited { retry_after, .. } | Self::Api { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// RFC 9457 HTTP yanıtından `CliError::Api` üretir.
    pub fn from_http_response(
        status: reqwest::StatusCode,
        headers: &HeaderMap,
        bytes: &[u8],
    ) -> Self {
        let problem: Option<ProblemDetails> = serde_json::from_slice(bytes).ok();

        let request_id = problem
            .as_ref()
            .and_then(|p| p.request_id.clone())
            .or_else(|| {
                headers
                    .get("x-request-id")
                    .and_then(|v| v.to_str().ok())
                    .map(ToString::to_string)
            });

        let retry_after = headers
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        let code = problem
            .as_ref()
            .and_then(|p| p.code)
            .unwrap_or_else(|| match status.as_u16() {
                400 => actos_sdk::actos_types::ErrorCode::ValidationFailed,
                401 => actos_sdk::actos_types::ErrorCode::MissingCredentials,
                403 => actos_sdk::actos_types::ErrorCode::Forbidden,
                404 => actos_sdk::actos_types::ErrorCode::NotFound,
                409 => actos_sdk::actos_types::ErrorCode::Conflict,
                410 => actos_sdk::actos_types::ErrorCode::Gone,
                415 => actos_sdk::actos_types::ErrorCode::UnsupportedMedia,
                429 => actos_sdk::actos_types::ErrorCode::RateLimited,
                _ => actos_sdk::actos_types::ErrorCode::Internal,
            });

        let message = problem
            .as_ref()
            .and_then(|p| p.detail.clone().or_else(|| p.title.clone()))
            .unwrap_or_else(|| {
                if let Ok(text) = std::str::from_utf8(bytes)
                    && !text.trim().is_empty()
                {
                    text.trim().to_string()
                } else {
                    status
                        .canonical_reason()
                        .unwrap_or("HTTP Error")
                        .to_string()
                }
            });

        if code == actos_sdk::actos_types::ErrorCode::RateLimited {
            Self::RateLimited {
                message,
                retry_after,
                request_id,
            }
        } else {
            Self::Api {
                code,
                message,
                status: status.as_u16(),
                request_id,
                retry_after,
            }
        }
    }

    /// Hatayı stderr'e basar (`PLAN.md` §2 Ajan Sözleşmesi kural 1 ve 2).
    pub fn render(&self, json: bool) {
        if json {
            let error_code = self.error_code();
            let code_str = match serde_json::to_string(&error_code) {
                Ok(s) => s.trim_matches('"').to_string(),
                Err(_) => "INTERNAL".to_string(),
            };
            let status = self.status();
            let mut err_val = serde_json::json!({
                "code": code_str,
                "message": self.message(),
                "status": status,
            });

            if let Some(req_id) = self.request_id()
                && let Some(map) = err_val.as_object_mut()
            {
                map.insert("request_id".to_string(), serde_json::json!(req_id));
            }

            if let Some(retry) = self.retry_after()
                && let Some(map) = err_val.as_object_mut()
            {
                map.insert("retry_after".to_string(), serde_json::json!(retry));
            }

            let val = serde_json::json!({ "error": err_val });
            eprintln!("{}", serde_json::to_string(&val).unwrap_or_default());
        } else if let Some(req_id) = self.request_id() {
            eprintln!("error: {} (request ID: {})", self.message(), req_id);
        } else {
            eprintln!("error: {}", self.message());
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

/// Machine-readable error-code string (e.g. `RATE_LIMITED`), used to keep the
/// user-facing message greppable by agents even when the server omits a detail.
fn code_name(code: actos_sdk::actos_types::ErrorCode) -> String {
    serde_json::to_string(&code)
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_default()
}

impl From<actos_sdk::Error> for CliError {
    fn from(e: actos_sdk::Error) -> Self {
        match e {
            actos_sdk::Error::Api {
                code,
                status,
                detail,
                request_id,
                retry_after,
                ..
            } => {
                // API metni İngilizce kalır; detay yoksa makine-okunur kod eklenir
                // (ajanlar `RATE_LIMITED` gibi kodlara göre karar verebilir).
                // SDK `detail` alanını gerçek detay yokken HTTP canonical reason ile
                // doldurur (örn. "Too Many Requests"); bu durumda kod gömülür ki
                // ajanlar grep'lenebilir kalsın.
                let canonical = StatusCode::from_u16(status)
                    .ok()
                    .and_then(|s| s.canonical_reason());
                let message = match detail {
                    Some(d) if Some(d.as_str()) != canonical => d,
                    _ => format!("[{} {}]", status, code_name(code)),
                };

                if code == actos_sdk::actos_types::ErrorCode::RateLimited {
                    Self::RateLimited {
                        message,
                        retry_after: retry_after.map(|d| d.as_secs()),
                        request_id,
                    }
                } else {
                    Self::Api {
                        code,
                        message,
                        status,
                        request_id,
                        retry_after: retry_after.map(|d| d.as_secs()),
                    }
                }
            }
            actos_sdk::Error::Transport(e) => Self::Network(e.to_string()),
            actos_sdk::Error::Decode(m) => Self::General(format!("could not decode response: {m}")),
            actos_sdk::Error::Config(m) => Self::Validation(m),
            actos_sdk::Error::Io(e) => Self::Io(e.to_string()),
            _ => Self::General("unhandled SDK error".to_string()),
        }
    }
}
