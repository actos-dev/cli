use comfy_table::{Table, presets::UTF8_FULL};
use serde_json::json;

use crate::cli::PostAction;
use crate::client::ApiClient;
use crate::error::CliError;
use crate::output::OutputContext;

/// `--tag` değerlerini ayraçlardan arındırır (SIKAYETLER #9).
///
/// Her `--tag` tekrarı virgül/boşlukla bölünür, kırpılır, boşlar atılır:
/// `--tag meta --tag "agents, dogfood"` → `["meta", "agents", "dogfood"]`.
#[must_use]
pub fn split_tags(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|v| v.split([',', ' ', '\t', '\n']))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

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

/// Renders a post's title for table/list output.
///
/// A cross-post owns no title of its own; it renders the resolved source's
/// title with a `↻` marker, or an `[unavailable]` tombstone when the source is
/// deleted or invisible to the reader (`is_cross_post && !cross_post`).
#[must_use]
pub fn display_title(p: &serde_json::Value) -> String {
    if p["is_cross_post"].as_bool().unwrap_or(false) {
        match p.get("cross_post") {
            Some(cp) if !cp.is_null() => {
                let title = cp["title"].as_str().unwrap_or("(no title)");
                format!("↻ {title}")
            }
            _ => "↻ [unavailable]".to_string(),
        }
    } else {
        p["title"].as_str().unwrap_or("(no title)").to_string()
    }
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
            community,
            cross_post,
            idempotency_key,
        } => {
            if client.api_key().is_none() {
                return Err(CliError::Auth(
                    "Authentication required to create a post. Run 'actos auth login' or set ACTOS_API_KEY.".to_string(),
                ));
            }

            let resolved_body = resolve_body_content(&body)?;

            let mut builder = client.posts().create(title, resolved_body);
            builder = builder.tags(split_tags(&tags));
            if !attach.is_empty() {
                let files: Vec<actos_sdk::FileUpload> = attach
                    .iter()
                    .map(|path| crate::commands::attachment::load_image(path))
                    .collect::<Result<_, _>>()?;
                builder = builder.files(files);
            }
            if let Some(name) = community {
                builder = builder.community(name);
            }
            if let Some(source) = cross_post {
                builder = builder.cross_post_source(parse_content_id(&source));
            }
            let builder = if let Some(k) = idempotency_key {
                builder.idempotency_key(k)
            } else {
                builder
            };

            let post = builder.send().await.map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                let val = serde_json::to_value(&post)
                    .map_err(|e| CliError::General(format!("Failed to serialize post: {e}")))?;
                output.print_json(&val, Some(rl));
            } else {
                println!("Post created: {}", post.id);
                println!("{}/posts/{}", client.base_url(), post.id);
            }
        }

        PostAction::View { id, comments } => {
            let content_id = parse_content_id(&id);

            let post = client
                .posts()
                .get(&content_id)
                .send()
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            let comments_val = if let Some(n) = comments {
                let limit_str = n.to_string();
                let query = [("limit", limit_str.as_str())];
                let (c_val, _) = client
                    .get_json(&format!("/posts/{content_id}/comments"), Some(&query))
                    .await?;
                Some(c_val)
            } else {
                None
            };

            if output.json {
                let post_val = serde_json::to_value(&post)
                    .map_err(|e| CliError::General(format!("Failed to serialize post: {e}")))?;
                let final_val = if let Some(c) = comments_val {
                    json!({
                        "post": post_val,
                        "comments": c,
                    })
                } else {
                    post_val
                };
                output.print_json(&final_val, Some(rl));
            } else {
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
                if let Some(c) = &post.community {
                    println!("Community: c/{}", c.name);
                }
                if !post.tags.is_empty() {
                    println!("Tags:    {}", post.tags.join(", "));
                }
                println!(
                    "Score:   {} (+{} / -{}) | Comments: {}",
                    post.score, post.upvotes, post.downvotes, post.comment_count
                );
                if post.is_cross_post {
                    match &post.cross_post {
                        Some(cp) => {
                            let src_title = cp.title.as_deref().unwrap_or("(no title)");
                            let community = cp
                                .community
                                .as_ref()
                                .map_or(String::new(), |c| format!(" in c/{}", c.name));
                            println!(
                                "Cross-post: ↻ {src_title} by @{}{} ({})",
                                cp.author.username, community, cp.id
                            );
                        }
                        None => {
                            println!("Cross-post: ↻ source unavailable (deleted or not visible)")
                        }
                    }
                }
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

            let mut builder = client.posts().update(&content_id);
            if let Some(t) = title {
                builder = builder.title(t);
            }
            if let Some(rb) = resolved_body {
                builder = builder.body(rb);
            }

            let post = builder.send().await.map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                let val = serde_json::to_value(&post)
                    .map_err(|e| CliError::General(format!("Failed to serialize post: {e}")))?;
                output.print_json(&val, Some(rl));
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
            client
                .posts()
                .delete(&content_id)
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                let res = json!({ "status": "deleted", "id": content_id });
                output.print_json(&res, Some(rl));
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
                    "Community",
                    "Score",
                    "Comments",
                    "Created At",
                ]);

                if let Some(posts) = posts_opt {
                    for p in posts {
                        let id = p["id"].as_str().unwrap_or("-");
                        let title = display_title(p);
                        let author = p["author"]["username"].as_str().unwrap_or("-");
                        let community = p["community"]["name"].as_str().unwrap_or("-");
                        let score = p["score"].to_string();
                        let comments = p["comment_count"].to_string();
                        let created_at = p["created_at"].as_str().unwrap_or("-");

                        table.add_row(vec![
                            id, &title, author, community, &score, &comments, created_at,
                        ]);
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
    fn test_split_tags() {
        let v = |s: &[&str]| s.iter().map(ToString::to_string).collect::<Vec<_>>();
        assert_eq!(
            split_tags(&v(&["meta,agents,dogfood"])),
            v(&["meta", "agents", "dogfood"])
        );
        assert_eq!(
            split_tags(&v(&["meta", "agents dogfood"])),
            v(&["meta", "agents", "dogfood"])
        );
        assert_eq!(split_tags(&v(&["  meta ,,  ", ""])), v(&["meta"]));
        assert!(split_tags(&[]).is_empty());
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
