use comfy_table::{Table, presets::UTF8_FULL};

use crate::cli::SearchArgs;
use crate::client::ApiClient;
use crate::error::CliError;
use crate::output::OutputContext;

pub async fn handle_search(
    args: SearchArgs,
    client: &ApiClient,
    output: &OutputContext,
    limit: u32,
    cursor: Option<&str>,
) -> Result<(), CliError> {
    let query_params = [("q", args.query.as_str()), ("type", args.r#type.as_str())];

    let (val, rate_limit) = client
        .paginate("/search", &query_params, limit, cursor)
        .await?;

    if output.json {
        output.print_json(&val, Some(rate_limit));
    } else {
        let results_opt = val.get("results").and_then(|v| v.as_array());

        match results_opt {
            Some(results) if !results.is_empty() => {
                let mut table = Table::new();
                table.load_preset(UTF8_FULL);

                match args.r#type.as_str() {
                    "actor" => {
                        table.set_header(vec![
                            "ID",
                            "Username",
                            "Type",
                            "Display Name",
                            "Created At",
                        ]);
                        for r in results {
                            let id = r["id"].as_str().unwrap_or("-");
                            let username = r["username"].as_str().unwrap_or("-");
                            let actor_type = r["actor_type"].as_str().unwrap_or("-");
                            let display_name = r["display_name"].as_str().unwrap_or("-");
                            let created_at = r["created_at"].as_str().unwrap_or("-");
                            table.add_row(vec![id, username, actor_type, display_name, created_at]);
                        }
                    }
                    "comment" => {
                        table.set_header(vec!["ID", "Author", "Snippet", "Score", "Created At"]);
                        for r in results {
                            let id = r["id"].as_str().unwrap_or("-");
                            let author = r["author"]["username"].as_str().unwrap_or("-");
                            let full_body = r["body"].as_str().unwrap_or("-");
                            let snippet = if full_body.chars().count() > 60 {
                                let s: String = full_body.chars().take(57).collect();
                                format!("{s}...")
                            } else {
                                full_body.replace('\n', " ")
                            };
                            let score = r["score"].to_string();
                            let created_at = r["created_at"].as_str().unwrap_or("-");
                            table.add_row(vec![id, author, &snippet, &score, created_at]);
                        }
                    }
                    _ => {
                        // "post" ve varsayılan
                        table.set_header(vec![
                            "ID",
                            "Title",
                            "Author",
                            "Tags",
                            "Score",
                            "Comments",
                            "Created At",
                        ]);
                        for r in results {
                            let id = r["id"].as_str().unwrap_or("-");
                            let title = r["title"].as_str().unwrap_or("(no title)");
                            let author = r["author"]["username"].as_str().unwrap_or("-");
                            let tags = r["tags"]
                                .as_array()
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|t| t.as_str())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                })
                                .unwrap_or_default();
                            let score = r["score"].to_string();
                            let comments = r["comment_count"].to_string();
                            let created_at = r["created_at"].as_str().unwrap_or("-");
                            table.add_row(vec![
                                id, title, author, &tags, &score, &comments, created_at,
                            ]);
                        }
                    }
                }

                println!("{table}");
            }
            _ => {
                println!("No results found for query '{}'.", args.query);
            }
        }
    }

    Ok(())
}
