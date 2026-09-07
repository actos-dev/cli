//! `actos user` — hesap listeleme ve aktif hesap değiştirme (SIKAYETLER #10).
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

fn two_profile_config() -> NamedTempFile {
    #![allow(clippy::unwrap_used)]
    let tmp = NamedTempFile::new().unwrap();
    let config_toml = r#"
default_profile = "default"

[profiles.default]
api_url = "https://api.actos.test"
username = "alice"
actor_type = "human"

[profiles.work]
api_url = "https://api.actos.test"
username = "bob"
actor_type = "ai_agent"
"#;
    std::fs::write(tmp.path(), config_toml).unwrap();
    tmp
}

#[test]
fn test_user_list_shows_active_and_hides_keys() {
    let tmp = two_profile_config();

    Command::cargo_bin("actos")
        .unwrap()
        .env("ACTOS_CONFIG", tmp.path())
        .args(["user"])
        .assert()
        .success()
        .stdout(predicate::str::contains("default *"))
        .stdout(predicate::str::contains("@alice").not())
        .stdout(predicate::str::contains("alice"))
        .stdout(predicate::str::contains("api_key").not());
}

#[test]
fn test_user_switch_persists_and_confirms_identity() {
    let tmp = two_profile_config();

    Command::cargo_bin("actos")
        .unwrap()
        .env("ACTOS_CONFIG", tmp.path())
        .args(["user", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Active account: 'work'"))
        .stdout(predicate::str::contains("@bob"));

    let content = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(content.contains(r#"default_profile = "work""#));
}

#[test]
fn test_user_switch_unknown_fails() {
    let tmp = two_profile_config();

    Command::cargo_bin("actos")
        .unwrap()
        .env("ACTOS_CONFIG", tmp.path())
        .args(["user", "ghost"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("ghost"));
}
