#![forbid(unsafe_code)]

use actos::cli::{Cli, Commands};
use actos::commands::actor::handle_actor;
use actos::commands::admin::handle_admin;
use actos::commands::auth::handle_auth;
use actos::commands::comment::handle_comment;
use actos::commands::config::handle_config;
use actos::commands::feed::handle_feed;
use actos::commands::post::handle_post;
use actos::commands::report::handle_report;
use actos::commands::save::handle_save;
use actos::commands::search::handle_search;
use actos::commands::tag::handle_tag;
use actos::commands::upload::handle_upload;
use actos::commands::vote::handle_vote;
use actos::config::Config;
use actos::error::{CliError, ExitCode};
use clap::Parser;

async fn run(cli: Cli) -> Result<(), CliError> {
    let mut config = Config::load()?;
    let output = cli.output_context();
    let client = cli.build_client(&config)?;
    let profile = cli.profile.clone();
    let yes = cli.yes;
    let json = cli.json;
    let limit = cli.limit;
    let cursor = cli.cursor.clone();

    match cli.command {
        Commands::Config(args) => handle_config(args.action, profile.as_deref(), json),
        Commands::Auth(args) => {
            let target_profile = profile
                .as_deref()
                .unwrap_or(config.default_profile.as_str())
                .to_string();
            handle_auth(args.action, &client, &mut config, &output, &target_profile).await
        }
        Commands::Post(args) => handle_post(args.action, &client, &output, yes).await,
        Commands::Comment(args) => handle_comment(args.action, &client, &output, yes).await,
        Commands::Feed(args) => handle_feed(args, &client, &output, limit, cursor.as_deref()).await,
        Commands::Search(args) => {
            handle_search(args, &client, &output, limit, cursor.as_deref()).await
        }
        Commands::Tag(args) => {
            handle_tag(args.action, &client, &output, limit, cursor.as_deref()).await
        }
        Commands::Actor(args) => {
            handle_actor(args.action, &client, &output, yes, limit, cursor.as_deref()).await
        }
        Commands::Vote(args) => handle_vote(args.action, &client, &output).await,
        Commands::Save(args) => {
            handle_save(args.action, &client, &output, limit, cursor.as_deref()).await
        }
        Commands::Upload(args) => handle_upload(args.action, &client, &output, yes).await,
        Commands::Report(args) => handle_report(args.action, &client, &output).await,
        Commands::Admin(args) => {
            handle_admin(args.action, &client, &output, yes, limit, cursor.as_deref()).await
        }
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_json = args.iter().any(|arg| arg == "--json");

    match Cli::try_parse() {
        Ok(cli) => {
            let json_mode = cli.json;
            if let Err(err) = run(cli).await {
                err.render(json_mode);
                std::process::exit(err.exit_code().as_i32());
            }
        }
        Err(err) => {
            use clap::error::ErrorKind;
            match err.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                    let _ = err.print();
                    std::process::exit(ExitCode::Success.as_i32());
                }
                _ => {
                    if is_json {
                        let cli_err = CliError::Usage(err.to_string());
                        cli_err.render(true);
                    } else {
                        let _ = err.print();
                    }
                    std::process::exit(ExitCode::UsageError.as_i32());
                }
            }
        }
    }
}
