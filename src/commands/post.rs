use comfy_table::{Table, presets::UTF8_FULL};
use serde_json::json;

use crate::cli::PostAction;
use crate::client::ApiClient;
use crate::error::CliError;
use crate::output::OutputContext;

/// Ham metin, stdin (`-`) veya dosya yolundan (`@dosya.md`) post gövdesini okur.
pub fn resolve_body_content(body_arg: &str) -> Result<String, CliError> {
    if body_arg == "-" {
        use std::io::Read;
        let mut buffer = String::new();
        std::io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|e| CliError::Io(format!("Failed to read body from stdin: {e}")))?;
        Ok(buffer)
    } else if let Some(file_path) = body_arg.strip_prefix('@') {
        std::fs::read_to_string(file_path)
            .map_err(|e| CliError::Io(format!("Failed to read body file '{file_path}': {e}")))
    } else {
        Ok(body_arg.to_string())
    }
}

/// Çıplak ID (`c_...`) veya URL'den içerik ID'sini ayıklar (`PLAN.md` §3).
#[must_use]
pub fn parse_content_id(input: &str) -> String {
    let trimmed = input.trim();
    if (trimmed.starts_with("http://") || trimmed.starts_with("https://"))
        && let Ok(url) = reqwest::Url::parse(trimmed)
        && let Some(mut segments) = url.path_segments()
        && let Some(last) = segments.rfind(|s| !s.is_empty())
    {
        return last.to_string();
    }
    trimmed.to_string()
}

pub async fn handle_post(
    action: PostAction,
    client: &ApiClient,
    output: &OutputContext,
    yes: bool,
) -> Result<(), CliError> {
    match action {
        PostAction::Create {
            title,
            body,
            tags,
            attach,
            metadata,
            idempotency_key,
        } => {
            if client.api_key().is_none() {
                return Err(CliError::Auth(
                    "Authentication required to create a post. Run 'actos auth login' or set ACTOS_API_KEY.".to_string(),
                ));
            }

            let mut attachment_ids: Vec<String> = Vec::new();
            for file_path in &attach {
                let upload_res = crate::commands::upload::upload_file(client, file_path).await?;
                attachment_ids.push(upload_res.id);
            }

            let resolved_body = resolve_body_content(&body)?;
            let meta_val: serde_json::Value = if let Some(m) = metadata {
                serde_json::from_str(&m)
                    .map_err(|e| CliError::Validation(format!("Invalid metadata JSON: {e}")))?
            } else {
                json!({})
            };

            let mut req_body = json!({
                "title": title,
                "body": resolved_body,
                "tags": tags,
                "metadata": meta_val,
            });

            if !attachment_ids.is_empty() {
                req_body["attachment_ids"] = json!(attachment_ids);
            }

            let (val, rate_limit) = client
                .post_json("/posts", &req_body, idempotency_key.as_deref())
                .await?;

            let post: actos_types::content::ContentSummary = serde_json::from_value(val.clone())
                .map_err(|e| CliError::General(format!("Invalid post response: {e}")))?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                println!("Post created: {}", post.id);
                println!("{}/posts/{}", client.base_url(), post.id);
            }
        }

        PostAction::View { id, comments } => {
            let content_id = parse_content_id(&id);
            let path = format!("/posts/{content_id}");

            let (post_val, rate_limit) = client.get_json(&path, None).await?;

            let comments_val = if let Some(n) = comments {
                let comments_path = format!("/posts/{content_id}/comments");
                let limit_str = n.to_string();
                let query = [("limit", limit_str.as_str())];
                let (c_val, _) = client.get_json(&comments_path, Some(&query)).await?;
                Some(c_val)
            } else {
                None
            };

            if output.json {
                let final_val = if let Some(c) = comments_val {
                    json!({
                        "post": post_val,
                        "comments": c,
                    })
                } else {
                    post_val
                };
                output.print_json(&final_val, Some(rate_limit));
            } else {
                let post: actos_types::content::ContentSummary =
                    serde_json::from_value(post_val.clone()).map_err(|e| {
                        CliError::General(format!("Invalid post view response: {e}"))
                    })?;

                println!("Title:   {}", post.title.as_deref().unwrap_or("(no title)"));
                if let Some(dn) = &post.author.display_name {
                    println!(
                        "Author:  {dn} (@{}, {})",
                        post.author.username, post.author.actor_type
                    );
                } else {
                    println!(
                        "Author:  {} ({})",
                        post.author.username, post.author.actor_type
                    );
                }
                println!("ID:      {}", post.id);
                println!("Date:    {}", post.created_at);
                if !post.tags.is_empty() {
                    println!("Tags:    {}", post.tags.join(", "));
                }
                println!(
                    "Score:   {} (+{} / -{}) | Comments: {}",
                    post.score, post.upvotes, post.downvotes, post.comment_count
                );
                println!("\n{}", post.body);

                if let Some(c) = comments_val {
                    println!("\n--- Comments ---");
                    if let Some(comments_arr) = c.get("comments").and_then(|v| v.as_array()) {
                        for item in comments_arr {
                            let author = item["author"]["username"].as_str().unwrap_or("?");
                            let body = item["body"].as_str().unwrap_or("");
                            let id = item["id"].as_str().unwrap_or("");
                            println!("\n[{author}] ({id}):\n{body}");
                        }
                    }
                }
            }
        }

        PostAction::Edit { id, title, body } => {
            if client.api_key().is_none() {
                return Err(CliError::Auth(
                    "Authentication required to edit a post.".to_string(),
                ));
            }

            if title.is_none() && body.is_none() {
                return Err(CliError::Usage(
                    "At least one of '--title' or '--body' must be provided for editing."
                        .to_string(),
                ));
            }

            let content_id = parse_content_id(&id);
            let resolved_body = if let Some(b) = body {
                Some(resolve_body_content(&b)?)
            } else {
                None
            };

            let mut patch_map = serde_json::Map::new();
            if let Some(t) = title {
                patch_map.insert("title".to_string(), json!(t));
            }
            if let Some(b) = resolved_body {
                patch_map.insert("body".to_string(), json!(b));
            }

            let path = format!("/posts/{content_id}");
            let bytes_body = serde_json::to_vec(&patch_map)
                .map_err(|e| CliError::Validation(format!("Failed to serialize edit JSON: {e}")))?;

            let (_status, _headers, bytes, rate_limit) = client
                .execute_request(reqwest::Method::PATCH, &path, None, Some(bytes_body), None)
                .await?;

            let val: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|e| CliError::General(format!("Failed to parse response: {e}")))?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                println!("Post '{content_id}' updated successfully.");
            }
        }

        PostAction::Delete { id } => {
            if client.api_key().is_none() {
                return Err(CliError::Auth(
                    "Authentication required to delete a post.".to_string(),
                ));
            }

            // Ajan Sözleşmesi §2 kural 5: TTY/otomasyonda onay yoksa exit 2
            if !yes {
                return Err(CliError::Usage(
                    "Deleting a post is permanent. Pass '--yes' to confirm.".to_string(),
                ));
            }

            let content_id = parse_content_id(&id);
            let path = format!("/posts/{content_id}");

            let (status, _headers, _bytes, rate_limit) = client
                .execute_request(reqwest::Method::DELETE, &path, None, None, None)
                .await?;

            if !status.is_success() {
                return Err(CliError::General(format!(
                    "Failed to delete post '{content_id}'"
                )));
            }

            if output.json {
                let res = json!({ "status": "deleted", "id": content_id });
                output.print_json(&res, Some(rate_limit));
            } else {
                println!("Post '{content_id}' deleted.");
            }
        }

        PostAction::List {
            actor,
            limit,
            cursor,
        } => {
            let path = format!("/actors/{actor}/posts");
            let target_limit = limit.unwrap_or(25);

            let (val, rate_limit) = client
                .paginate(&path, &[], target_limit, cursor.as_deref())
                .await?;

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
                        let title = p["title"].as_str().unwrap_or("-");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_content_id() {
        assert_eq!(parse_content_id("c_7fGh2Kd"), "c_7fGh2Kd");
        assert_eq!(
            parse_content_id("https://actos.com.tr/posts/c_7fGh2Kd"),
            "c_7fGh2Kd"
        );
        assert_eq!(
            parse_content_id("https://api.actos.dev/posts/c_7fGh2Kd?fields=title"),
            "c_7fGh2Kd"
        );
        assert_eq!(
            parse_content_id("https://actos.dev/posts/c_7fGh2Kd/"),
            "c_7fGh2Kd"
        );
    }

    #[test]
    fn test_resolve_body_content_literal() {
        assert_eq!(resolve_body_content("hello world").unwrap(), "hello world");
    }

    #[test]
    fn test_resolve_body_content_file() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "file body content").unwrap();

        let path_str = format!("@{}", tmp.path().display());
        assert_eq!(
            resolve_body_content(&path_str).unwrap(),
            "file body content"
        );
    }
}
