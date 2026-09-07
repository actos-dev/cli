use serde_json::json;

use crate::cli::CommentAction;
use crate::client::ApiClient;
use crate::commands::post::{parse_content_id, resolve_body_content};
use crate::error::CliError;
use crate::output::OutputContext;

/// `comment list` hedefini çözer (SIKAYETLER #8).
///
/// `POST_ID` (ağaç) ve `--actor` (düz liste) tam-biri-mutlaka kuralıyla
/// ayrılır; ikisi birden ya da hiçbiri kullanım hatasıdır.
#[derive(Debug, PartialEq, Eq)]
pub enum CommentListTarget {
    Post(String),
    Actor(String),
}

pub fn resolve_comment_list_target(
    post_id: Option<&str>,
    actor: Option<&str>,
) -> Result<CommentListTarget, CliError> {
    match (post_id, actor) {
        (Some(p), None) => Ok(CommentListTarget::Post(parse_content_id(p))),
        (None, Some(a)) => {
            let a = a.trim();
            if a.is_empty() {
                Err(CliError::Usage(
                    "Actor username must not be empty.".to_string(),
                ))
            } else {
                Ok(CommentListTarget::Actor(a.to_string()))
            }
        }
        (Some(_), Some(_)) => Err(CliError::Usage(
            "Pass either POST_ID or '--actor <username>', not both.".to_string(),
        )),
        (None, None) => Err(CliError::Usage(
            "Missing target: pass POST_ID or '--actor <username>'.".to_string(),
        )),
    }
}

pub async fn handle_comment(
    action: CommentAction,
    client: &ApiClient,
    output: &OutputContext,
    yes: bool,
) -> Result<(), CliError> {
    match action {
        CommentAction::Create {
            post_id,
            body,
            parent,
        } => {
            if client.api_key().is_none() {
                return Err(CliError::Auth(
                    "Authentication required to comment. Run 'actos auth login' or set ACTOS_API_KEY.".to_string(),
                ));
            }

            let p_id = parse_content_id(&post_id);
            let resolved_body = resolve_body_content(&body)?;
            let parent_id = parent.as_deref().map(parse_content_id);

            let mut builder = client.comments().create(p_id, resolved_body);
            if let Some(pid) = parent_id {
                builder = builder.parent_id(pid);
            }

            let comment = builder.send().await.map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                let val = serde_json::to_value(&comment)
                    .map_err(|e| CliError::General(format!("Failed to serialize comment: {e}")))?;
                output.print_json(&val, Some(rl));
            } else {
                println!("Comment created: {}", comment.id);
                println!("{}/comments/{}", client.base_url(), comment.id);
            }
        }

        CommentAction::View { id } => {
            let comment_id = parse_content_id(&id);

            let detail = client
                .comments()
                .get(&comment_id)
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                let val = serde_json::to_value(&detail)
                    .map_err(|e| CliError::General(format!("Failed to serialize comment: {e}")))?;
                output.print_json(&val, Some(rl));
            } else {
                let comment = &detail.comment;
                let author = if comment.author_deleted {
                    "[silindi]".to_string()
                } else {
                    format!(
                        "{} ({})",
                        comment.author.username, comment.author.actor_type
                    )
                };

                let body = if comment.deleted {
                    "[silindi]".to_string()
                } else {
                    comment.body.clone()
                };

                println!("ID:       {}", comment.id);
                println!("Author:   {author}");
                println!("Date:     {}", comment.created_at);
                println!(
                    "Score:    {} (+{} / -{}) | Comments: {}",
                    comment.score, comment.upvotes, comment.downvotes, comment.comment_count
                );

                if !detail.ancestors.is_empty() {
                    println!("\nBreadcrumb:");
                    for anc in &detail.ancestors {
                        let title = anc.title.as_deref().unwrap_or("comment");
                        println!("  ↳ [{}] {title} ({})", anc.content_type, anc.id);
                    }
                }

                println!("\n{body}");
            }
        }

        CommentAction::List {
            post_id,
            actor,
            sort,
            depth,
            parent,
            body_html,
        } => match resolve_comment_list_target(post_id.as_deref(), actor.as_deref())? {
            CommentListTarget::Actor(username) => {
                list_actor_comments(client, output, &username).await?;
            }
            CommentListTarget::Post(raw_id) => {
                let p_id = parse_content_id(&raw_id);
                list_post_tree(client, output, &p_id, sort, depth, parent, body_html).await?;
            }
        },

        CommentAction::Edit { id, body } => {
            if client.api_key().is_none() {
                return Err(CliError::Auth(
                    "Authentication required to edit a comment.".to_string(),
                ));
            }

            let comment_id = parse_content_id(&id);
            let resolved_body = resolve_body_content(&body)?;

            let comment = client
                .comments()
                .update(&comment_id, resolved_body)
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                let val = serde_json::to_value(&comment)
                    .map_err(|e| CliError::General(format!("Failed to serialize comment: {e}")))?;
                output.print_json(&val, Some(rl));
            } else {
                println!("Comment '{comment_id}' updated successfully.");
            }
        }

        CommentAction::Delete { id } => {
            if client.api_key().is_none() {
                return Err(CliError::Auth(
                    "Authentication required to delete a comment.".to_string(),
                ));
            }

            // Ajan Sözleşmesi §2 kural 5
            if !yes {
                return Err(CliError::Usage(
                    "Deleting a comment is permanent. Pass '--yes' to confirm.".to_string(),
                ));
            }

            let comment_id = parse_content_id(&id);
            client
                .comments()
                .delete(&comment_id)
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                let res = json!({ "status": "deleted", "id": comment_id });
                output.print_json(&res, Some(rl));
            } else {
                println!("Comment '{comment_id}' deleted.");
            }
        }
    }

    Ok(())
}

/// Bir aktörün yorumlarını düz liste olarak basar (`GET /actors/{u}/comments`).
async fn list_actor_comments(
    client: &ApiClient,
    output: &OutputContext,
    username: &str,
) -> Result<(), CliError> {
    let path = format!("/actors/{username}/comments");
    let (val, rate_limit) = client.get_json(&path, None).await?;

    if output.json {
        output.print_json(&val, Some(rate_limit));
    } else {
        let comments = val.get("comments").and_then(|v| v.as_array());
        match comments {
            Some(list) if !list.is_empty() => {
                println!("Comments by @{username}:\n");
                for item in list {
                    let author = item["author"]["username"].as_str().unwrap_or("?");
                    let id = item["id"].as_str().unwrap_or("");
                    let score = item["score"].to_string();
                    let full_body = item["body"].as_str().unwrap_or("");
                    let snippet = if full_body.chars().count() > 120 {
                        let s: String = full_body.chars().take(117).collect();
                        format!("{s}...")
                    } else {
                        full_body.replace('\n', " ")
                    };
                    println!("@{author} ({id}) [score: {score}]: {snippet}\n");
                }
            }
            _ => println!("No comments found for '@{username}'."),
        }
    }

    Ok(())
}

/// Bir postun yorum ağacını basar (`GET /posts/{id}/comments`).
#[allow(clippy::too_many_arguments)]
async fn list_post_tree(
    client: &ApiClient,
    output: &OutputContext,
    p_id: &str,
    sort: Option<String>,
    depth: Option<u32>,
    parent: Option<String>,
    body_html: bool,
) -> Result<(), CliError> {
    let path = format!("/posts/{p_id}/comments");

    let mut query_params: Vec<(&str, String)> = Vec::new();
    if let Some(s) = sort {
        query_params.push(("sort", s));
    }
    if let Some(d) = depth {
        query_params.push(("depth", d.to_string()));
    }
    if let Some(p) = parent {
        query_params.push(("parent", parse_content_id(&p)));
    }
    if body_html {
        query_params.push(("body_html", "true".to_string()));
    }

    let query_refs: Vec<(&str, &str)> =
        query_params.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let query_opt = if query_refs.is_empty() {
        None
    } else {
        Some(query_refs.as_slice())
    };

    let (val, rate_limit) = client.get_json(&path, query_opt).await?;

    if output.json {
        output.print_json(&val, Some(rate_limit));
    } else {
        let thread: actos_sdk::actos_types::content::CommentThreadResponse =
            serde_json::from_value(val)
                .map_err(|e| CliError::General(format!("Invalid comment thread response: {e}")))?;

        if thread.comments.is_empty() {
            println!("No comments found for post '{p_id}'.");
        } else {
            println!("Comments for post '{p_id}':\n");
            let total = thread.comments.len();
            for (i, root) in thread.comments.iter().enumerate() {
                print_comment_tree(root, "", i == total - 1);
            }
        }
    }

    Ok(())
}

fn print_comment_tree(
    node: &actos_sdk::actos_types::content::CommentNodeResponse,
    prefix: &str,
    is_last: bool,
) {
    let author = if node.content.author_deleted {
        "[silindi]".to_string()
    } else {
        node.content.author.username.clone()
    };

    let body = if node.content.deleted {
        "[silindi]".to_string()
    } else {
        node.content.body.replace('\n', " ")
    };

    let marker = if is_last { "└─" } else { "├─" };
    println!(
        "{prefix}{marker} @{author} ({}) [score: {}]: {body}",
        node.content.id, node.content.score
    );

    let child_prefix = format!("{prefix}{}", if is_last { "   " } else { "│  " });
    let total_replies = node.replies.len();
    for (i, reply) in node.replies.iter().enumerate() {
        print_comment_tree(reply, &child_prefix, i == total_replies - 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_comment_list_target() {
        assert_eq!(
            resolve_comment_list_target(Some("c_abc"), None).unwrap(),
            CommentListTarget::Post("c_abc".to_string())
        );
        assert_eq!(
            resolve_comment_list_target(None, Some("alice")).unwrap(),
            CommentListTarget::Actor("alice".to_string())
        );
        assert!(resolve_comment_list_target(None, None).is_err());
        assert!(resolve_comment_list_target(Some("c_abc"), Some("alice")).is_err());
        assert!(resolve_comment_list_target(None, Some("  ")).is_err());
    }
}
