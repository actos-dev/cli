use std::fmt;

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

impl From<actos_types::ErrorCode> for ExitCode {
    fn from(code: actos_types::ErrorCode) -> Self {
        match code {
            actos_types::ErrorCode::ValidationFailed
            | actos_types::ErrorCode::InvalidCursor
            | actos_types::ErrorCode::UnsupportedMedia => Self::ValidationError,
            actos_types::ErrorCode::MissingCredentials | actos_types::ErrorCode::InvalidKey => {
                Self::AuthFailed
            }
            actos_types::ErrorCode::Forbidden | actos_types::ErrorCode::Banned => Self::Forbidden,
            actos_types::ErrorCode::NotFound => Self::NotFound,
            actos_types::ErrorCode::Gone => Self::Gone,
            actos_types::ErrorCode::Conflict => Self::Conflict,
            actos_types::ErrorCode::RateLimited => Self::RateLimited,
            actos_types::ErrorCode::Internal => Self::ServerError,
        }
    }
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
    },
    Server(String),
    Network(String),
    General(String),
    Io(String),
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
            Self::General(_) => ExitCode::GeneralError,
            Self::Io(_) => ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub const fn error_code(&self) -> actos_types::ErrorCode {
        match self {
            Self::Usage(_) | Self::Validation(_) => actos_types::ErrorCode::ValidationFailed,
            Self::Auth(_) => actos_types::ErrorCode::MissingCredentials,
            Self::Forbidden(_) => actos_types::ErrorCode::Forbidden,
            Self::NotFound(_) => actos_types::ErrorCode::NotFound,
            Self::Gone(_) => actos_types::ErrorCode::Gone,
            Self::Conflict(_) => actos_types::ErrorCode::Conflict,
            Self::RateLimited { .. } => actos_types::ErrorCode::RateLimited,
            Self::Server(_) | Self::Network(_) | Self::General(_) | Self::Io(_) => {
                actos_types::ErrorCode::Internal
            }
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
            Self::RateLimited { message, .. } => message.as_str(),
        }
    }

    #[must_use]
    pub const fn retry_after(&self) -> Option<u64> {
        match self {
            Self::RateLimited { retry_after, .. } => *retry_after,
            _ => None,
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
            let status = error_code.http_status();
            let mut val = serde_json::json!({
                "error": {
                    "code": code_str,
                    "message": self.message(),
                    "status": status,
                }
            });

            if let Some(retry) = self.retry_after()
                && let Some(map) = val.get_mut("error").and_then(|v| v.as_object_mut())
            {
                map.insert("retry_after".to_string(), serde_json::json!(retry));
            }

            eprintln!("{}", serde_json::to_string(&val).unwrap_or_default());
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
