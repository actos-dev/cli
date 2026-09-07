//! `actos update` — ağa çıkmadan test edilebilen yollar (--version).
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_update_check_same_version_json() {
    let assert = Command::cargo_bin("actos")
        .unwrap()
        .args(["--json", "update", "--check", "--version", "0.2.0"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let val: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(val["installed"], "0.2.0");
    assert_eq!(val["latest"], "0.2.0");
    assert_eq!(val["update_available"], false);
}

#[test]
fn test_update_check_newer_human() {
    Command::cargo_bin("actos")
        .unwrap()
        .args(["update", "--check", "--version", "9.9.9"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Update available: 0.2.0 -> 9.9.9"));
}
