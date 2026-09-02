use comfy_table::{Table, presets::UTF8_FULL};

use crate::cli::TagAction;
use crate::client::ApiClient;
use crate::error::CliError;
use crate::output::OutputContext;

pub async fn handle_tag(
    action: TagAction,
    client: &ApiClient,
    output: &OutputContext,
    limit: u32,
    cursor: Option<&str>,
) -> Result<(), CliError> {
    match action {
        TagAction::List => {
            let (val, rate_limit) = client.paginate("/tags", &[], limit, cursor).await?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                let tags_opt = val.get("tags").and_then(|v| v.as_array());

                let mut table = Table::new();
                table.load_preset(UTF8_FULL);
                table.set_header(vec!["Tag", "Post Count", "Created At"]);

                if let Some(tags) = tags_opt {
                    for t in tags {
                        let name = t["name"].as_str().unwrap_or("-");
                        let count = t["post_count"].to_string();
                        let created_at = t["created_at"].as_str().unwrap_or("-");
                        table.add_row(vec![format!("#{name}"), count, created_at.to_string()]);
                    }
                }

                println!("{table}");
            }
        }

        TagAction::Search { prefix } => {
            let query = [("q", prefix.as_str())];
            let (val, rate_limit) = client.get_json("/tags/search", Some(&query)).await?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                let tags_opt = val.get("tags").and_then(|v| v.as_array());

                if let Some(tags) = tags_opt {
                    if tags.is_empty() {
                        println!("No tags found matching '{prefix}'.");
                    } else {
                        println!("Matching tags:");
                        for t in tags {
                            let name = t["name"].as_str().unwrap_or("-");
                            println!("  #{name}");
                        }
                    }
                }
            }
        }

        TagAction::Posts { name, sort } => {
            let path = format!("/tags/{name}/posts");
            let mut query_params: Vec<(&str, &str)> = Vec::new();
            if let Some(ref s) = sort {
                query_params.push(("sort", s.as_str()));
            }

            let (val, rate_limit) = client.paginate(&path, &query_params, limit, cursor).await?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                let posts_opt = val.get("posts").and_then(|v| v.as_array());

                let mut table = Table::new();
                table.load_preset(UTF8_FULL);
                table.set_header(vec![
                    "ID",
                    "Title",
                    "Author",
                    "Score",
                    "Comments",
                    "Created At",
                ]);

                if let Some(posts) = posts_opt {
                    for p in posts {
                        let id = p["id"].as_str().unwrap_or("-");
                        let title = p["title"].as_str().unwrap_or("(no title)");
                        let author = p["author"]["username"].as_str().unwrap_or("-");
                        let score = p["score"].to_string();
                        let comments = p["comment_count"].to_string();
                        let created_at = p["created_at"].as_str().unwrap_or("-");

                        table.add_row(vec![id, title, author, &score, &comments, created_at]);
                    }
                }

                println!("{table}");
            }
        }
    }

    Ok(())
}
