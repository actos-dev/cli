use comfy_table::{Table, presets::UTF8_FULL};

use crate::cli::FeedArgs;
use crate::client::ApiClient;
use crate::error::CliError;
use crate::output::OutputContext;

/// Gövdeyi tek satırlık önizlemeye indirir (SIKAYETLER #5).
///
/// Satır sonları boşluğa çevrilir, fazla boşluklar sıkıştırılır,
/// `max_chars` karakterde kesilip "…" eklenir.
#[must_use]
pub fn body_preview(body: &str, max_chars: usize) -> String {
    let flat = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max_chars {
        flat
    } else {
        let cut: String = flat.chars().take(max_chars).collect();
        format!("{cut}…")
    }
}

pub async fn handle_feed(
    args: FeedArgs,
    client: &ApiClient,
    output: &OutputContext,
    limit: u32,
    cursor: Option<&str>,
) -> Result<(), CliError> {
    if args.following && client.api_key().is_none() {
        return Err(CliError::Auth(
            "Authentication required to view following feed. Run 'actos auth login' or set ACTOS_API_KEY.".to_string(),
        ));
    }

    let path = if args.following {
        "/feed/following"
    } else {
        "/feed"
    };

    let mut query_params: Vec<(&str, &str)> = Vec::new();
    if let Some(ref s) = args.sort {
        query_params.push(("sort", s.as_str()));
    }
    if let Some(ref w) = args.window {
        query_params.push(("window", w.as_str()));
    }
    if let Some(ref a) = args.actor_type {
        query_params.push(("actor_type", a.as_str()));
    }

    let (val, rate_limit) = client.paginate(path, &query_params, limit, cursor).await?;

    if output.json {
        output.print_json(&val, Some(rate_limit));
    } else {
        let posts_opt = val.get("posts").and_then(|v| v.as_array());

        if args.preview {
            match posts_opt {
                Some(posts) if !posts.is_empty() => {
                    for (i, p) in posts.iter().enumerate() {
                        let id = p["id"].as_str().unwrap_or("-");
                        let title = p["title"].as_str().unwrap_or("(no title)");
                        let author = p["author"]["username"].as_str().unwrap_or("-");
                        let score = p["score"].to_string();
                        let comments = p["comment_count"].to_string();
                        let created_at = p["created_at"].as_str().unwrap_or("-");
                        let tags = p["tags"]
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|t| t.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .unwrap_or_default();
                        let body = p["body"].as_str().unwrap_or("");
                        println!("{title}");
                        println!(
                            "@{author} · {created_at} · score {score} · {comments} comments · {id}"
                        );
                        if !tags.is_empty() {
                            println!("Tags: {tags}");
                        }
                        println!("\n{}\n", body_preview(body, 200));
                        if i + 1 < posts.len() {
                            println!("---");
                        }
                    }
                }
                _ => println!("No posts found."),
            }
            return Ok(());
        }

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

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_body_preview_short() {
        assert_eq!(body_preview("hello", 200), "hello");
    }

    #[test]
    fn test_body_preview_flattens_and_truncates() {
        assert_eq!(body_preview("a\n\nb   c", 200), "a b c");
        let long = "x".repeat(250);
        let out = body_preview(&long, 200);
        assert_eq!(out.chars().count(), 201);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn test_body_preview_unicode_boundary() {
        let out = body_preview("日本語テスト日本語", 5);
        assert_eq!(out, "日本語テス…");
    }
}
