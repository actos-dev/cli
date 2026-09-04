use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

#[test]
fn test_help_json_schema_completeness() {
    let mut cmd = Command::cargo_bin("actos").unwrap();
    let assert = cmd.args(["help", "--json"]).assert().success();

    let stdout_bytes = assert.get_output().stdout.clone();
    let schema: Value =
        serde_json::from_slice(&stdout_bytes).expect("actos help --json must output valid JSON");

    assert_eq!(schema["name"], "actos");
    assert!(schema["version"].is_string());

    // Exit codes check
    let exit_codes = schema["exit_codes"].as_object().unwrap();
    for code_str in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11"] {
        assert!(
            exit_codes.contains_key(code_str),
            "Missing exit code {code_str}"
        );
    }

    // Agent contract rules
    let rules = schema["agent_contract_rules"].as_array().unwrap();
    assert_eq!(rules.len(), 10, "Must contain all 10 agent contract rules");

    // Check all expected subcommands are present in tree
    let subcommands = schema["command"]["subcommands"].as_array().unwrap();
    let subcommand_names: Vec<&str> = subcommands
        .iter()
        .filter_map(|s| s["name"].as_str())
        .collect();

    let expected = [
        "config",
        "auth",
        "post",
        "comment",
        "feed",
        "search",
        "tag",
        "actor",
        "vote",
        "save",
        "upload",
        "report",
        "admin",
        "api",
        "docs",
        "inbox",
        "watch",
        "quota",
        "version",
        "completion",
        "man",
        "help",
        "tui",
    ];

    for exp in expected {
        assert!(
            subcommand_names.contains(&exp),
            "Command tree missing subcommand '{exp}'"
        );
    }
}

#[test]
fn test_completion_bash() {
    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.args(["completion", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_actos"));
}

#[test]
fn test_man_page() {
    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.args(["man"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ACTOS"));
}
