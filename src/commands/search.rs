use comfy_table::{Table, presets::UTF8_FULL};

use crate::cli::SearchArgs;
use crate::client::ApiClient;
use crate::error::CliError;
use crate::output::OutputContext;

/// Arama tipini çözer (SIKAYETLER #1).
///
/// `--type` verildiyse o kazanır. Verilmediyse sorgunun ilk kelimesi
/// `post|comment|actor` ise tip sayılır, kalanı sorgu olur:
/// `search post captcha` → `(post, captcha)`. Aksi halde kullanım hatası.
pub fn resolve_search_type(
    flag: Option<&str>,
    query: &str,
) -> Result<(String, String), CliError> {
    if let Some(t) = flag {
        return Ok((t.to_string(), query.to_string()));
    }
    let mut words = query.split_whitespace();
    match (words.next(), words.next()) {
        (Some(first), Some(_)) if is_search_type(first) => Ok((
            first.to_ascii_lowercase(),
            query.split_whitespace().skip(1).collect::<Vec<_>>().join(" "),
        )),
        _ => Err(CliError::Usage(
            "Search type is missing. Use '--type <post|comment|actor> <query>' or put the type first: 'actos search post captcha'.".to_string(),
        )),
    }
}

fn is_search_type(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "post" | "comment" | "actor"
    )
}

pub async fn handle_search(
    args: SearchArgs,
    client: &ApiClient,
    output: &OutputContext,
    limit: u32,
    cursor: Option<&str>,
) -> Result<(), CliError> {
    let raw_query = args.query.join(" ");
    let (search_type, query) = resolve_search_type(args.r#type.as_deref(), &raw_query)?;
    let query_params = [("q", query.as_str()), ("type", search_type.as_str())];

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

                match search_type.as_str() {
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
                println!("No results found for query '{query}'.");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_search_type_flag_wins() {
        assert_eq!(
            resolve_search_type(Some("actor"), "post captcha").unwrap(),
            ("actor".to_string(), "post captcha".to_string())
        );
    }

    #[test]
    fn test_resolve_search_type_inferred() {
        assert_eq!(
            resolve_search_type(None, "post captcha").unwrap(),
            ("post".to_string(), "captcha".to_string())
        );
        assert_eq!(
            resolve_search_type(None, "Actor lassie").unwrap(),
            ("actor".to_string(), "lassie".to_string())
        );
    }

    #[test]
    fn test_resolve_search_type_missing() {
        assert!(resolve_search_type(None, "captcha").is_err());
        assert!(resolve_search_type(None, "  ").is_err());
    }
}
