use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

#[test]
fn test_cli_config_list_masking() {
    let tmp = NamedTempFile::new().unwrap();
    let config_toml = r#"
default_profile = "default"

[profiles.default]
api_url = "https://api.actos.dev"
api_key = "actos_7Jw9876543210abcdef"
username = "alice"
actor_type = "human"
"#;
    std::fs::write(tmp.path(), config_toml).unwrap();

    // 1. İnsan-okunur mod
    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp.path())
        .args(["config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("actos_7Jw9..."))
        .stdout(predicate::str::contains("9876543210abcdef").not());

    // 2. --json modu
    let mut json_cmd = Command::cargo_bin("actos").unwrap();
    let assert = json_cmd
        .env("ACTOS_CONFIG", tmp.path())
        .args(["--json", "config", "list"])
        .assert()
        .success();

    let stdout_str = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let val: serde_json::Value = serde_json::from_str(&stdout_str).unwrap();
    assert_eq!(val["profiles"]["default"]["api_key"], "actos_7Jw9...");
    assert!(!stdout_str.contains("9876543210abcdef"));
}

#[test]
fn test_cli_config_get_and_set() {
    let tmp = NamedTempFile::new().unwrap();
    let config_toml = r#"
default_profile = "default"

[profiles.default]
api_url = "https://api.actos.dev"
api_key = "actos_initial_key"
"#;
    std::fs::write(tmp.path(), config_toml).unwrap();

    // Get api_url
    let mut cmd_get = Command::cargo_bin("actos").unwrap();
    cmd_get
        .env("ACTOS_CONFIG", tmp.path())
        .args(["config", "get", "api_url"])
        .assert()
        .success()
        .stdout(predicate::str::contains("https://api.actos.dev"));

    // Set new api_url
    let mut cmd_set = Command::cargo_bin("actos").unwrap();
    cmd_set
        .env("ACTOS_CONFIG", tmp.path())
        .args(["config", "set", "api_url", "http://127.0.0.1:3100"])
        .assert()
        .success();

    // Verify update
    let mut cmd_verify = Command::cargo_bin("actos").unwrap();
    cmd_verify
        .env("ACTOS_CONFIG", tmp.path())
        .args(["config", "get", "api_url"])
        .assert()
        .success()
        .stdout(predicate::str::contains("http://127.0.0.1:3100"));

    // Set in a custom profile
    let mut cmd_set_prof = Command::cargo_bin("actos").unwrap();
    cmd_set_prof
        .env("ACTOS_CONFIG", tmp.path())
        .args([
            "config",
            "set",
            "api_key",
            "actos_work_key",
            "--profile",
            "work",
        ])
        .assert()
        .success();

    let mut cmd_get_prof = Command::cargo_bin("actos").unwrap();
    cmd_get_prof
        .env("ACTOS_CONFIG", tmp.path())
        .args(["config", "get", "api_key", "--profile", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("actos_work_key"));
}

#[test]
fn test_cli_0600_permissions_creation() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("sub").join("config.toml");

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", &config_path)
        .args(["config", "set", "api_url", "https://api.actos.dev"])
        .assert()
        .success();

    assert!(config_path.exists());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(&config_path).unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "Created config file must have 0600 permissions"
        );
    }
}

#[test]
#[cfg(unix)]
fn test_cli_permission_warning_on_open_file() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), "default_profile = \"default\"\n").unwrap();

    // Make permissions too open: 0644
    let mut perms = std::fs::metadata(tmp.path()).unwrap().permissions();
    perms.set_mode(0o644);
    std::fs::set_permissions(tmp.path(), perms).unwrap();

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp.path())
        .args(["config", "list"])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning: config file"))
        .stderr(predicate::str::contains("permissions are too open"));
}

#[test]
fn test_cli_corrupt_config_handling() {
    let tmp = NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), b"broken toml [[[ ==").unwrap();

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp.path())
        .args(["config", "list"])
        .assert()
        .code(8) // ValidationError
        .stderr(predicate::str::contains("Corrupted config file"));
}

#[test]
fn test_cli_json_error_format_on_missing_key() {
    let tmp = NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), "default_profile = \"default\"\n").unwrap();

    let mut cmd = Command::cargo_bin("actos").unwrap();
    let assert = cmd
        .env("ACTOS_CONFIG", tmp.path())
        .args(["--json", "config", "get", "api_key"])
        .assert()
        .code(5); // NotFound

    let stderr_str = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    let val: serde_json::Value = serde_json::from_str(&stderr_str).unwrap();
    assert_eq!(val["error"]["code"], "NOT_FOUND");
    assert_eq!(val["error"]["status"], 404);
}

#[test]
fn test_cli_env_precedence_in_config() {
    let tmp = NamedTempFile::new().unwrap();
    let config_content = r#"
default_profile = "default"

[profiles.default]
api_url = "https://file-default.actos.dev"
api_key = "actos_file_key"

[profiles.work]
api_url = "https://file-work.actos.dev"
api_key = "actos_work_file_key"
"#;
    std::fs::write(tmp.path(), config_content).unwrap();

    let config = actos::config::Config::load_from_path(tmp.path()).unwrap();

    // 1. Varsayılan profil çözümlenmesi
    let resolved = config.resolve(None, None);
    assert_eq!(resolved.profile_name, "default");
    assert_eq!(resolved.api_url, "https://file-default.actos.dev");
    assert_eq!(resolved.api_key.as_deref(), Some("actos_file_key"));

    // 2. CLI argümanlarının profili ve api-url'i ezmesi
    let resolved_cli = config.resolve(Some("work"), Some("https://cli-override.actos.dev"));
    assert_eq!(resolved_cli.profile_name, "work");
    assert_eq!(resolved_cli.api_url, "https://cli-override.actos.dev");
    assert_eq!(resolved_cli.api_key.as_deref(), Some("actos_work_file_key"));
}
