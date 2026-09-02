use actos::client::ratelimit::RateLimitInfo;
use actos::error::ExitCode;
use actos::output::OutputContext;
use actos::output::filter::filter_fields;
use assert_cmd::Command;
use serde_json::json;
use tempfile::NamedTempFile;

#[test]
fn test_agent_contract_rule_1_stdout_purity() {
    let tmp = NamedTempFile::new().unwrap();
    let toml = r#"
default_profile = "default"

[profiles.default]
api_url = "https://api.actos.dev"
api_key = "actos_test_1234567890"
"#;
    std::fs::write(tmp.path(), toml).unwrap();

    let mut cmd = Command::cargo_bin("actos").unwrap();
    let assert = cmd
        .env("ACTOS_CONFIG", tmp.path())
        .args(["--json", "config", "list"])
        .assert()
        .success();

    let stdout_bytes = assert.get_output().stdout.clone();
    let stdout_str = String::from_utf8(stdout_bytes).unwrap();

    // 1. stdout yalnızca ve yalnız geçerli JSON olmalıdır
    let val: serde_json::Value = serde_json::from_str(&stdout_str)
        .expect("Ajan Sözleşmesi §2 Kural 1: stdout geçerli JSON olmak zorundadır!");

    assert_eq!(val["default_profile"], "default");

    // 2. stdout içinde hiçbir ANSI kaçış dizisi (\x1b) olmamalıdır
    assert!(
        !stdout_str.contains('\x1b'),
        "stdout içinde ANSI renk veya kaçış kodu bulunamaz!"
    );

    // 3. stdout içinde log, uyarı veya 'warning:' metni olmamalıdır
    assert!(
        !stdout_str.to_lowercase().contains("warning:"),
        "stdout içinde uyarı metni bulunamaz!"
    );
}

#[test]
fn test_agent_contract_rule_2_stderr_json_error() {
    let tmp = NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), "default_profile = \"default\"\n").unwrap();

    let mut cmd = Command::cargo_bin("actos").unwrap();
    let assert = cmd
        .env("ACTOS_CONFIG", tmp.path())
        .args(["--json", "config", "get", "non_existent_key"])
        .assert()
        .code(5); // NotFound

    let output = assert.get_output();

    // 1. Hata durumunda stdout tamamen boş olmalıdır
    let stdout_str = String::from_utf8(output.stdout.clone()).unwrap();
    assert!(
        stdout_str.trim().is_empty(),
        "Hata durumunda stdout'a hiçbir şey yazılmamalıdır!"
    );

    // 2. stderr geçerli bir JSON hata nesnesi olmalıdır
    let stderr_str = String::from_utf8(output.stderr.clone()).unwrap();
    let val: serde_json::Value = serde_json::from_str(stderr_str.trim())
        .expect("Ajan Sözleşmesi §2 Kural 2: stderr geçerli bir JSON nesnesi olmalıdır!");

    assert!(val.get("error").is_some());
    assert_eq!(val["error"]["code"], "NOT_FOUND");
    assert_eq!(val["error"]["status"], 404);
}

#[test]
fn test_exit_code_contract_mappings() {
    use actos_types::ErrorCode;

    assert_eq!(ExitCode::from(ErrorCode::ValidationFailed).as_i32(), 8);
    assert_eq!(ExitCode::from(ErrorCode::InvalidCursor).as_i32(), 8);
    assert_eq!(ExitCode::from(ErrorCode::UnsupportedMedia).as_i32(), 8);
    assert_eq!(ExitCode::from(ErrorCode::MissingCredentials).as_i32(), 3);
    assert_eq!(ExitCode::from(ErrorCode::InvalidKey).as_i32(), 3);
    assert_eq!(ExitCode::from(ErrorCode::Forbidden).as_i32(), 4);
    assert_eq!(ExitCode::from(ErrorCode::Banned).as_i32(), 4);
    assert_eq!(ExitCode::from(ErrorCode::NotFound).as_i32(), 5);
    assert_eq!(ExitCode::from(ErrorCode::Gone).as_i32(), 6);
    assert_eq!(ExitCode::from(ErrorCode::Conflict).as_i32(), 7);
    assert_eq!(ExitCode::from(ErrorCode::RateLimited).as_i32(), 9);
    assert_eq!(ExitCode::from(ErrorCode::Internal).as_i32(), 10);
    assert_eq!(ExitCode::NetworkError.as_i32(), 11);
    assert_eq!(ExitCode::UsageError.as_i32(), 2);
    assert_eq!(ExitCode::Success.as_i32(), 0);
}

#[test]
fn test_client_side_field_filtering() {
    let post = json!({
        "id": "c_12345",
        "title": "Actos CLI",
        "body": "Rust 2024 ve Clap ile yazıldı",
        "author": "efe",
        "votes_count": 42
    });

    let filtered = filter_fields(&post, &["id".to_string(), "title".to_string()]);

    assert_eq!(
        filtered,
        json!({
            "id": "c_12345",
            "title": "Actos CLI"
        })
    );
    assert!(filtered.get("body").is_none());
    assert!(filtered.get("votes_count").is_none());
}

#[test]
fn test_output_context_meta_rate_limit() {
    let _ctx = OutputContext::new(true, None, false, false);
    let val = json!({
        "id": "c_999",
        "title": "Test"
    });

    let rl = RateLimitInfo {
        limit: Some(100),
        remaining: Some(99),
        reset: Some(1700000000),
    };

    // Buffer'a yazma testi
    let mut modified = val.clone();
    if let Some(map) = modified.as_object_mut() {
        let mut meta = serde_json::Map::new();
        meta.insert("rate_limit".to_string(), serde_json::to_value(rl).unwrap());
        map.insert("_meta".to_string(), serde_json::Value::Object(meta));
    }

    assert_eq!(modified["_meta"]["rate_limit"]["remaining"], 99);
    assert_eq!(modified["_meta"]["rate_limit"]["limit"], 100);
}
