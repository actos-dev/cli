//! Bilinmeyen subcommand yerelde reddedilir, şebekeye çıkmaz (SIKAYETLER #2).
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_unknown_subcommand_fails_locally() {
    Command::cargo_bin("actos")
        .unwrap()
        .args(["tag", "meta"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_search_extra_positional_fails_locally() {
    Command::cargo_bin("actos")
        .unwrap()
        .args(["post", "view"])
        .assert()
        .failure()
        .code(2);
}
