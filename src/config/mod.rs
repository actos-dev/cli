pub mod permissions;

use crate::error::CliError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The default Actos API base URL when no explicit value (CLI flag, env,
/// or profile) is provided. Change this single spot when the domain moves
/// (e.g. `actos.me`) — it feeds the default profile, `resolve` fallback,
/// and the unit-test fixtures alike.
pub const DEFAULT_API_URL: &str = "https://api.actos.com.tr";

fn default_profile_name() -> String {
    "default".to_string()
}

/// Tek bir profilin ayarları (`PLAN.md` Faz 1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Profile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_type: Option<String>,
}

/// Tüm CLI yapılandırması.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_profile_name")]
    pub default_profile: String,
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
}

impl Default for Config {
    fn default() -> Self {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "default".to_string(),
            Profile {
                api_url: Some(DEFAULT_API_URL.to_string()),
                api_key: None,
                username: None,
                actor_type: None,
            },
        );
        Self {
            default_profile: "default".to_string(),
            profiles,
        }
    }
}

/// Komut çalıştırılırken çözümlenmiş nihai yapılandırma.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfig {
    pub profile_name: String,
    pub api_url: String,
    pub api_key: Option<String>,
    pub username: Option<String>,
    pub actor_type: Option<String>,
}

impl ResolvedConfig {
    /// Kimlik doğrulaması gereken komutlar için API anahtarını zorunlu tutar.
    ///
    /// Eksikse `ExitCode::AuthFailed` (3) ile dönen `CliError::Auth` üretir.
    pub fn require_api_key(&self) -> Result<&str, CliError> {
        match self.api_key.as_deref() {
            Some(key) if !key.trim().is_empty() => Ok(key),
            _ => Err(CliError::Auth(
                "Authentication required: no API key found. Set ACTOS_API_KEY or configure with 'actos config set api_key <key>' or 'actos auth login'.".to_string(),
            )),
        }
    }
}

/// API anahtarını maskeler (`PLAN.md` Faz 1: ilk 10 karakter + "...").
///
/// Tam anahtar asla stdout/stderr'e açık basılmaz.
#[must_use]
pub fn mask_api_key(key: &str) -> String {
    if key.is_empty() {
        return String::new();
    }
    let char_count = key.chars().count();
    let visible = if char_count > 10 {
        10
    } else if char_count > 4 {
        char_count / 2
    } else {
        1
    };
    let prefix: String = key.chars().take(visible).collect();
    format!("{prefix}...")
}

/// Yapılandırma dosyası yolunu belirler.
///
/// Öncelik: `$ACTOS_CONFIG` > `$XDG_CONFIG_HOME/actos/config.toml` > `~/.config/actos/config.toml`
#[must_use]
pub fn config_path() -> PathBuf {
    if let Ok(path) = std::env::var("ACTOS_CONFIG")
        && !path.trim().is_empty()
    {
        return PathBuf::from(path);
    }

    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
        && !xdg.trim().is_empty()
    {
        return PathBuf::from(xdg).join("actos").join("config.toml");
    }

    if let Some(config_dir) = dirs::config_dir() {
        return config_dir.join("actos").join("config.toml");
    }

    if let Some(home) = dirs::home_dir() {
        return home.join(".config").join("actos").join("config.toml");
    }

    PathBuf::from("config.toml")
}

impl Config {
    /// Maskelenmiş API anahtarlarına sahip bir kopyasını döner.
    #[must_use]
    pub fn masked(&self) -> Self {
        let mut cloned = self.clone();
        for profile in cloned.profiles.values_mut() {
            if let Some(ref key) = profile.api_key {
                profile.api_key = Some(mask_api_key(key));
            }
        }
        cloned
    }

    /// Belirtilen dosyadan yapılandırmayı okur.
    /// Dosya yoksa varsayılan boş/default yapılandırma döner.
    pub fn load_from_path(path: &Path) -> Result<Self, CliError> {
        if !path.exists() {
            return Ok(Self::default());
        }

        permissions::check_permissions(path);

        let content = std::fs::read_to_string(path).map_err(|e| {
            CliError::Io(format!(
                "Failed to read config file '{}': {e}",
                path.display()
            ))
        })?;

        let config: Self = toml::from_str(&content).map_err(|e| {
            CliError::Validation(format!("Corrupted config file '{}': {e}", path.display()))
        })?;

        Ok(config)
    }

    /// Varsayılan yoldan yapılandırmayı okur.
    pub fn load() -> Result<Self, CliError> {
        Self::load_from_path(&config_path())
    }

    /// Yapılandırmayı belirtilen dosyaya `0600` izinleriyle kaydeder.
    pub fn save_to_path(&self, path: &Path) -> Result<(), CliError> {
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent).map_err(|e| {
                CliError::Io(format!(
                    "Failed to create config directory '{}': {e}",
                    parent.display()
                ))
            })?;
        }

        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| CliError::Validation(format!("Failed to serialize config: {e}")))?;

        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;

            let mut options = std::fs::OpenOptions::new();
            options.write(true).create(true).truncate(true);
            options.mode(0o600);

            let mut file = options.open(path).map_err(|e| {
                CliError::Io(format!(
                    "Failed to open config file '{}': {e}",
                    path.display()
                ))
            })?;

            file.write_all(toml_str.as_bytes()).map_err(|e| {
                CliError::Io(format!(
                    "Failed to write config file '{}': {e}",
                    path.display()
                ))
            })?;

            let _ = permissions::ensure_0600_permissions(path);
        }

        #[cfg(not(unix))]
        {
            std::fs::write(path, toml_str).map_err(|e| {
                CliError::Io(format!(
                    "Failed to write config file '{}': {e}",
                    path.display()
                ))
            })?;
        }

        Ok(())
    }

    /// Varsayılan yola yapılandırmayı kaydeder.
    pub fn save(&self) -> Result<(), CliError> {
        Self::save_to_path(self, &config_path())
    }

    /// CLI argümanları ve ortam değişkenleri ışığında nihai yapılandırmayı çözümler.
    ///
    /// Öncelik:
    /// - API Anahtarı: `ACTOS_API_KEY` env > profile `api_key`
    /// - API URL: CLI `--api-url` > `ACTOS_API_URL` env > profile `api_url` > varsayılan URL
    /// - Profil: CLI `--profile` > `ACTOS_PROFILE` env > `config.default_profile` > `"default"`
    #[must_use]
    pub fn resolve(&self, cli_profile: Option<&str>, cli_api_url: Option<&str>) -> ResolvedConfig {
        let profile_name = cli_profile
            .filter(|p| !p.trim().is_empty())
            .map(ToString::to_string)
            .or_else(|| {
                std::env::var("ACTOS_PROFILE")
                    .ok()
                    .filter(|p| !p.trim().is_empty())
            })
            .unwrap_or_else(|| {
                if self.default_profile.trim().is_empty() {
                    "default".to_string()
                } else {
                    self.default_profile.clone()
                }
            });

        let profile = self
            .profiles
            .get(&profile_name)
            .cloned()
            .unwrap_or_default();

        let api_url = cli_api_url
            .filter(|u| !u.trim().is_empty())
            .map(ToString::to_string)
            .or_else(|| {
                std::env::var("ACTOS_API_URL")
                    .ok()
                    .filter(|u| !u.trim().is_empty())
            })
            .or(profile.api_url)
            .unwrap_or_else(|| DEFAULT_API_URL.to_string());

        let api_key = std::env::var("ACTOS_API_KEY")
            .ok()
            .filter(|k| !k.trim().is_empty())
            .or(profile.api_key);

        ResolvedConfig {
            profile_name,
            api_url,
            api_key,
            username: profile.username,
            actor_type: profile.actor_type,
        }
    }

    /// Ayar anahtarının değerini döner.
    pub fn get(&self, profile_name: &str, key: &str) -> Result<String, CliError> {
        if key == "default_profile" {
            return Ok(self.default_profile.clone());
        }

        let profile = self
            .profiles
            .get(profile_name)
            .ok_or_else(|| CliError::NotFound(format!("Profile '{profile_name}' not found")))?;

        match key {
            "api_url" => profile.api_url.clone().ok_or_else(|| {
                CliError::NotFound(format!(
                    "Key 'api_url' is not set in profile '{profile_name}'"
                ))
            }),
            "api_key" => profile.api_key.clone().ok_or_else(|| {
                CliError::NotFound(format!(
                    "Key 'api_key' is not set in profile '{profile_name}'"
                ))
            }),
            "username" => profile.username.clone().ok_or_else(|| {
                CliError::NotFound(format!(
                    "Key 'username' is not set in profile '{profile_name}'"
                ))
            }),
            "actor_type" => profile.actor_type.clone().ok_or_else(|| {
                CliError::NotFound(format!(
                    "Key 'actor_type' is not set in profile '{profile_name}'"
                ))
            }),
            other => Err(CliError::Usage(format!(
                "Invalid config key '{other}'. Valid keys: api_url, api_key, username, actor_type, default_profile"
            ))),
        }
    }

    /// Ayar anahtarına yeni değer atar.
    pub fn set(&mut self, profile_name: &str, key: &str, value: &str) -> Result<(), CliError> {
        if key == "default_profile" {
            self.default_profile = value.to_string();
            return Ok(());
        }

        match key {
            "api_url" | "api_key" | "username" | "actor_type" => {
                let profile = self.profiles.entry(profile_name.to_string()).or_default();
                match key {
                    "api_url" => profile.api_url = Some(value.to_string()),
                    "api_key" => profile.api_key = Some(value.to_string()),
                    "username" => profile.username = Some(value.to_string()),
                    "actor_type" => profile.actor_type = Some(value.to_string()),
                    _ => unreachable!(),
                }
                Ok(())
            }
            other => Err(CliError::Usage(format!(
                "Invalid config key '{other}'. Valid keys: api_url, api_key, username, actor_type, default_profile"
            ))),
        }
    }

    /// Kimlik anahtarını profilden tamamen kaldırır (SIKAYETLER #3).
    ///
    /// `set(key, "")` boş string bırakır ve `config list` boş gösterir;
    /// bu metot `None` yapar ki liste "(not set)" göstersin.
    pub fn clear_key(&mut self, profile_name: &str, key: &str) -> Result<(), CliError> {
        match key {
            "api_url" | "api_key" | "username" | "actor_type" => {
                if let Some(profile) = self.profiles.get_mut(profile_name) {
                    match key {
                        "api_url" => profile.api_url = None,
                        "api_key" => profile.api_key = None,
                        "username" => profile.username = None,
                        "actor_type" => profile.actor_type = None,
                        _ => unreachable!(),
                    }
                }
                Ok(())
            }
            other => Err(CliError::Usage(format!(
                "Invalid config key '{other}'. Valid keys: api_url, api_key, username, actor_type, default_profile"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ExitCode;
    use tempfile::NamedTempFile;

    #[test]
    fn test_mask_api_key() {
        assert_eq!(mask_api_key(""), "");
        assert_eq!(mask_api_key("actos_7Jw1234567890"), "actos_7Jw1...");
        assert_eq!(mask_api_key("actos_secret_key"), "actos_secr...");
        assert_eq!(mask_api_key("shortkey"), "shor...");
        assert!(!mask_api_key("actos_secret_key").contains("secret_key"));
    }

    #[test]
    fn test_require_api_key_present() {
        let resolved = ResolvedConfig {
            profile_name: "default".to_string(),
            api_url: DEFAULT_API_URL.to_string(),
            api_key: Some("actos_valid_key".to_string()),
            username: None,
            actor_type: None,
        };
        assert_eq!(resolved.require_api_key().unwrap(), "actos_valid_key");
    }

    #[test]
    fn test_require_api_key_missing() {
        let resolved = ResolvedConfig {
            profile_name: "default".to_string(),
            api_url: DEFAULT_API_URL.to_string(),
            api_key: None,
            username: None,
            actor_type: None,
        };
        let err = resolved.require_api_key().unwrap_err();
        assert_eq!(err.exit_code(), ExitCode::AuthFailed);
        assert_eq!(
            err.error_code(),
            actos_sdk::actos_types::ErrorCode::MissingCredentials
        );
    }

    #[test]
    fn test_config_get_set() {
        let mut config = Config::default();
        config
            .set("default", "api_url", "http://127.0.0.1:3100")
            .unwrap();
        assert_eq!(
            config.get("default", "api_url").unwrap(),
            "http://127.0.0.1:3100"
        );

        config.set("default", "api_key", "actos_my_key").unwrap();
        assert_eq!(config.get("default", "api_key").unwrap(), "actos_my_key");

        config.set("default", "username", "alice").unwrap();
        assert_eq!(config.get("default", "username").unwrap(), "alice");

        config.set("default", "actor_type", "human").unwrap();
        assert_eq!(config.get("default", "actor_type").unwrap(), "human");

        config.set("default", "default_profile", "custom").unwrap();
        assert_eq!(config.get("default", "default_profile").unwrap(), "custom");
    }

    #[test]
    fn test_clear_key_removes_identity() {
        let mut config = Config::default();
        config.set("default", "api_key", "actos_k").unwrap();
        config.set("default", "username", "alice").unwrap();
        config.set("default", "actor_type", "human").unwrap();

        config.clear_key("default", "api_key").unwrap();
        config.clear_key("default", "username").unwrap();
        config.clear_key("default", "actor_type").unwrap();

        assert!(config.get("default", "api_key").is_err());
        assert!(config.get("default", "username").is_err());
        assert!(config.get("default", "actor_type").is_err());
        assert!(config.clear_key("default", "nope").is_err());
        // Var olmayan profil sessizce geçilir (logout güvenli).
        assert!(config.clear_key("ghost", "username").is_ok());
    }

    #[test]
    fn test_config_invalid_key() {
        let mut config = Config::default();
        let err = config.set("default", "invalid_key", "value").unwrap_err();
        assert_eq!(err.exit_code(), ExitCode::UsageError);

        let err2 = config.get("default", "invalid_key").unwrap_err();
        assert_eq!(err2.exit_code(), ExitCode::UsageError);
    }

    #[test]
    fn test_masked_config() {
        let mut config = Config::default();
        config
            .set("default", "api_key", "actos_super_secret_key_12345")
            .unwrap();
        let masked = config.masked();

        let masked_key = masked.get("default", "api_key").unwrap();
        assert!(masked_key.ends_with("..."));
        assert!(!masked_key.contains("super_secret_key_12345"));
    }

    #[test]
    fn test_missing_config_returns_default() {
        let non_existent_path = PathBuf::from("/tmp/non_existent_actos_config_123456789.toml");
        let config = Config::load_from_path(&non_existent_path).unwrap();
        assert_eq!(config.default_profile, "default");
    }

    #[test]
    fn test_corrupt_config_returns_validation_error() {
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"invalid [ [ toml syntax === ").unwrap();

        let err = Config::load_from_path(tmp.path()).unwrap_err();
        assert_eq!(err.exit_code(), ExitCode::ValidationError);
        assert!(err.message().contains("Corrupted config file"));
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let tmp = NamedTempFile::new().unwrap();
        let mut config = Config::default();
        config
            .set("default", "api_url", "http://127.0.0.1:3100")
            .unwrap();
        config
            .set("default", "api_key", "actos_secret_key_saved")
            .unwrap();
        config.set("default", "username", "bob").unwrap();
        config.set("default", "actor_type", "ai_agent").unwrap();

        config.save_to_path(tmp.path()).unwrap();

        let loaded = Config::load_from_path(tmp.path()).unwrap();
        assert_eq!(
            loaded.get("default", "api_url").unwrap(),
            "http://127.0.0.1:3100"
        );
        assert_eq!(
            loaded.get("default", "api_key").unwrap(),
            "actos_secret_key_saved"
        );
        assert_eq!(loaded.get("default", "username").unwrap(), "bob");
        assert_eq!(loaded.get("default", "actor_type").unwrap(), "ai_agent");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = std::fs::metadata(tmp.path()).unwrap();
            assert_eq!(meta.permissions().mode() & 0o777, 0o600);
        }
    }
}
