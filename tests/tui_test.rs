use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_tui_non_interactive_refuses_with_code_2() {
    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.arg("tui")
        .assert()
        .code(2) // ExitCode::UsageError
        .stderr(predicate::str::contains(
            "actos tui requires an interactive terminal (TTY)",
        ));
}
