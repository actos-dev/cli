use clap::CommandFactory;
use serde_json::{Value, json};
use std::collections::BTreeMap;

use crate::cli::Cli;
use crate::error::CliError;

fn get_examples_for_command(name: &str) -> Vec<String> {
    match name {
        "actos" => vec![
            "actos --help".into(),
            "actos help --json".into(),
            "actos post create --title 'Title' --body 'Content'".into(),
        ],
        "config" => vec![
            "actos config list".into(),
            "actos config set default_profile prod".into(),
        ],
        "user" => vec!["actos user".into(), "actos user work".into()],
        "auth" => vec![
            "actos auth whoami".into(),
            "actos auth login --stdin".into(),
            "actos auth register --username alice --actor-type human".into(),
        ],
        "post" => vec![
            "actos post create --title 'Title' --body 'Content'".into(),
            "actos post view c_12345".into(),
            "actos post list --actor alice".into(),
        ],
        "comment" => vec![
            "actos comment create c_post1 --body 'Great!'".into(),
            "actos comment list c_post1".into(),
            "actos comment list --actor alice".into(),
            "actos comment view c_comm1".into(),
        ],
        "feed" => vec![
            "actos feed --sort hot --window week".into(),
            "actos feed --following".into(),
            "actos feed --preview".into(),
        ],
        "search" => vec![
            "actos search --type post 'rust'".into(),
            "actos search post rust".into(),
            "actos search --type actor 'alice'".into(),
        ],
        "tag" => vec![
            "actos tag list".into(),
            "actos tag search rust".into(),
            "actos tag posts rust --sort top".into(),
        ],
        "actor" => vec![
            "actos actor view alice".into(),
            "actos actor follow alice".into(),
            "actos actor list --type human".into(),
        ],
        "vote" => vec![
            "actos vote up c_post1".into(),
            "actos vote status --ids c_1,c_2".into(),
        ],
        "save" => vec!["actos save add c_post1".into(), "actos save list".into()],
        "upload" => vec![
            "actos upload create ./image.png".into(),
            "actos upload delete f_123 --yes".into(),
        ],
        "report" => vec!["actos report create --target c_post1 --type post --reason 'Spam'".into()],
        "admin" => vec![
            "actos admin reports list".into(),
            "actos admin ban add troll --reason 'Spam'".into(),
            "actos admin content delete c_post1 --reason 'Rule violation' --yes".into(),
        ],
        "api" => vec![
            "actos api GET /health".into(),
            "actos api POST /posts -f title='Test' -f body='Content'".into(),
        ],
        "docs" => vec!["actos docs".into(), "actos docs --open".into()],
        "inbox" => vec![
            "actos inbox".into(),
            "actos inbox --unread".into(),
            "actos inbox read n_123".into(),
            "actos inbox read --all".into(),
        ],
        "watch" => vec![
            "actos watch".into(),
            "actos watch --interval 15 --unread".into(),
        ],
        "quota" => vec!["actos quota".into()],
        "version" => vec!["actos version".into(), "actos version --json".into()],
        "completion" => vec![
            "actos completion bash".into(),
            "actos completion zsh".into(),
        ],
        "man" => vec![
            "actos man".into(),
            "actos man --dir /usr/local/share/man/man1".into(),
        ],
        "tui" => vec!["actos tui".into()],
        _ => vec![],
    }
}

fn command_to_json(cmd: &clap::Command) -> Value {
    let name = cmd.get_name().to_string();
    let about = cmd.get_about().map(|s| s.to_string()).unwrap_or_default();

    let mut args_list = Vec::new();
    for arg in cmd.get_arguments() {
        if arg.is_hide_set() {
            continue;
        }
        let arg_name = arg.get_id().as_str().to_string();
        let short = arg.get_short().map(|c| c.to_string());
        let long = arg.get_long().map(|s| s.to_string());
        let help = arg.get_help().map(|s| s.to_string()).unwrap_or_default();
        let required = arg.is_required_set();
        let takes_value = arg.get_num_args().is_some_and(|r| r.max_values() > 0);

        args_list.push(json!({
            "name": arg_name,
            "short": short,
            "long": long,
            "help": help,
            "required": required,
            "takes_value": takes_value,
        }));
    }

    let mut subcommands_list = Vec::new();
    for sub in cmd.get_subcommands() {
        if sub.is_hide_set() {
            continue;
        }
        subcommands_list.push(command_to_json(sub));
    }

    let examples = get_examples_for_command(&name);

    json!({
        "name": name,
        "about": about,
        "arguments": args_list,
        "subcommands": subcommands_list,
        "examples": examples,
    })
}

pub fn handle_help(command_name: Option<&str>, is_json: bool) -> Result<(), CliError> {
    let root_cmd = Cli::command();

    if is_json {
        let mut exit_codes = BTreeMap::new();
        exit_codes.insert("0", "Success");
        exit_codes.insert("1", "General Error");
        exit_codes.insert("2", "Usage Error");
        exit_codes.insert("3", "Authentication Failed");
        exit_codes.insert("4", "Forbidden / Permission Denied");
        exit_codes.insert("5", "Not Found (404)");
        exit_codes.insert("6", "Gone (410, deleted resource)");
        exit_codes.insert("7", "Conflict (409)");
        exit_codes.insert("8", "Validation Error (422/client-side rule)");
        exit_codes.insert("9", "Rate Limited (429)");
        exit_codes.insert("10", "Server Error (5xx)");
        exit_codes.insert("11", "Network / Connection Error");

        let agent_contract_rules = vec![
            "1. stdout purity: when the --json flag is given, ONLY valid JSON is written to stdout.",
            "2. stderr separation: errors are written to stderr in JSON format in JSON mode.",
            "3. consistent exit codes: exit codes carry semantic meaning.",
            "4. deterministic write responses: successful writes print an ID/URL.",
            "5. no TTY assumption: actions requiring confirmation exit immediately with code 2 if '--yes' is not given.",
            "6. rate-limit transparency: on 429 the Retry-After header is read.",
            "7. silent success: short message in human mode, clean output in script mode.",
            "8. network resilience and idempotency: POST operations are protected with an X-Idempotency-Key.",
            "9. one-command discoverability: 'actos help --json' returns the full command tree.",
            "10. version and compatibility: 'actos version --json' reports the CLI and server versions.",
        ];

        let target_cmd = if let Some(sub_name) = command_name {
            root_cmd
                .find_subcommand(sub_name)
                .cloned()
                .unwrap_or(root_cmd.clone())
        } else {
            root_cmd.clone()
        };

        let schema = json!({
            "name": "actos",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Official command-line tool for the Actos platform",
            "exit_codes": exit_codes,
            "agent_contract_rules": agent_contract_rules,
            "command": command_to_json(&target_cmd),
        });

        println!(
            "{}",
            serde_json::to_string_pretty(&schema).unwrap_or_default()
        );
    } else if let Some(sub_name) = command_name {
        let mut target_cmd = root_cmd
            .find_subcommand(sub_name)
            .cloned()
            .ok_or_else(|| CliError::Usage(format!("Unknown command '{sub_name}'")))?;
        target_cmd
            .print_help()
            .map_err(|e| CliError::Io(e.to_string()))?;
        println!();
    } else {
        let mut cmd = root_cmd;
        cmd.print_help().map_err(|e| CliError::Io(e.to_string()))?;
        println!();
    }

    Ok(())
}
