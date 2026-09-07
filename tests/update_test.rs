//! `actos update` — ağa çıkmadan test edilebilen yollar (--version).
use assert_cmd::Command;
use predicates::prelude::*;

const INSTALLED: &str = env!("CARGO_PKG_VERSION");

#[test]
fn test_update_check_same_version_json() {
    let assert = Command::cargo_bin("actos")
        .unwrap()
        .args(["--json", "update", "--check", "--version", INSTALLED])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let val: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(val["installed"], INSTALLED);
    assert_eq!(val["latest"], INSTALLED);
    assert_eq!(val["update_available"], false);
}

#[test]
fn test_update_check_newer_human() {
    Command::cargo_bin("actos")
        .unwrap()
        .args(["update", "--check", "--version", "9.9.9"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "Update available: {INSTALLED} -> 9.9.9"
        )));
}
