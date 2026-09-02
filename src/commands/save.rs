use comfy_table::{Table, presets::UTF8_FULL};
use serde_json::json;

use crate::cli::SaveAction;
use crate::client::ApiClient;
use crate::commands::post::parse_content_id;
use crate::error::CliError;
use crate::output::OutputContext;

pub async fn handle_save(
    action: SaveAction,
    client: &ApiClient,
    output: &OutputContext,
    limit: u32,
    cursor: Option<&str>,
) -> Result<(), CliError> {
    if client.api_key().is_none() {
        return Err(CliError::Auth(
            "Authentication required for saved items. Run 'actos auth login' or set ACTOS_API_KEY."
                .to_string(),
        ));
    }

    match action {
        SaveAction::Add { id } => {
            let content_id = parse_content_id(&id);
            let path = format!("/contents/{content_id}/save");

            let (status, _headers, _bytes, rate_limit) = client
                .execute_request(reqwest::Method::PUT, &path, None, None, None)
                .await?;

            if !status.is_success() {
                return Err(CliError::General(format!("Failed to save '{content_id}'")));
            }

            if output.json {
                let res = json!({ "status": "saved", "id": content_id });
                output.print_json(&res, Some(rate_limit));
            } else {
                println!("Saved '{content_id}'.");
            }
        }

        SaveAction::Remove { id } => {
            let content_id = parse_content_id(&id);
            let path = format!("/contents/{content_id}/save");

            let (status, _headers, _bytes, rate_limit) = client
                .execute_request(reqwest::Method::DELETE, &path, None, None, None)
                .await?;

            if !status.is_success() {
                return Err(CliError::General(format!(
                    "Failed to remove '{content_id}' from saved items"
                )));
            }

            if output.json {
                let res = json!({ "status": "removed", "id": content_id });
                output.print_json(&res, Some(rate_limit));
            } else {
                println!("Removed '{content_id}' from saved items.");
            }
        }

        SaveAction::List => {
            let (val, rate_limit) = client.paginate("/me/saves", &[], limit, cursor).await?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                let saves_opt = val.get("saves").and_then(|v| v.as_array());

                let mut table = Table::new();
                table.load_preset(UTF8_FULL);
                table.set_header(vec![
                    "ID",
                    "Type",
                    "Title / Preview",
                    "Author",
                    "Score",
                    "Created At",
                ]);

                if let Some(saves) = saves_opt {
                    for s in saves {
                        let id = s["id"].as_str().unwrap_or("-");
                        let ctype = s["content_type"].as_str().unwrap_or("-");
                        let title_or_body = s["title"]
                            .as_str()
                            .unwrap_or_else(|| s["body"].as_str().unwrap_or("-"));
                        let preview = if title_or_body.chars().count() > 50 {
                            let truncated: String = title_or_body.chars().take(47).collect();
                            format!("{truncated}...")
                        } else {
                            title_or_body.replace('\n', " ")
                        };
                        let author = s["author"]["username"].as_str().unwrap_or("-");
                        let score = s["score"].to_string();
                        let created = s["created_at"].as_str().unwrap_or("-");

                        table.add_row(vec![id, ctype, &preview, author, &score, created]);
                    }
                }

                println!("{table}");
            }
        }
    }

    Ok(())
}
