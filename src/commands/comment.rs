use serde_json::json;

use crate::cli::CommentAction;
use crate::client::ApiClient;
use crate::commands::post::{parse_content_id, resolve_body_content};
use crate::error::CliError;
use crate::output::OutputContext;

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

            let mut req_body = json!({
                "body": resolved_body,
            });

            if let Some(pid) = parent_id {
                req_body["parent_id"] = json!(pid);
            }

            let path = format!("/posts/{p_id}/comments");
            let (val, rate_limit) = client.post_json(&path, &req_body, None).await?;

            let comment: actos_types::content::ContentSummary = serde_json::from_value(val.clone())
                .map_err(|e| {
                    CliError::General(format!("Invalid comment creation response: {e}"))
                })?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                println!("Comment created: {}", comment.id);
                println!("{}/comments/{}", client.base_url(), comment.id);
            }
        }

        CommentAction::View { id } => {
            let comment_id = parse_content_id(&id);
            let path = format!("/comments/{comment_id}");

            let (val, rate_limit) = client.get_json(&path, None).await?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                let detail: actos_types::content::CommentDetailResponse =
                    serde_json::from_value(val).map_err(|e| {
                        CliError::General(format!("Invalid comment detail response: {e}"))
                    })?;

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
            sort,
            depth,
            parent,
            body_html,
        } => {
            let p_id = parse_content_id(&post_id);
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

            let (val, rate_limit) = client.get_json(&path, Some(&query_refs)).await?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                let thread: actos_types::content::CommentThreadResponse =
                    serde_json::from_value(val).map_err(|e| {
                        CliError::General(format!("Invalid comment thread response: {e}"))
                    })?;

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
        }

        CommentAction::Edit { id, body } => {
            if client.api_key().is_none() {
                return Err(CliError::Auth(
                    "Authentication required to edit a comment.".to_string(),
                ));
            }

            let comment_id = parse_content_id(&id);
            let resolved_body = resolve_body_content(&body)?;
            let patch_json = json!({ "body": resolved_body });

            let path = format!("/comments/{comment_id}");
            let bytes_body = serde_json::to_vec(&patch_json).map_err(|e| {
                CliError::Validation(format!("Failed to serialize comment edit JSON: {e}"))
            })?;

            let (_status, _headers, bytes, rate_limit) = client
                .execute_request(reqwest::Method::PATCH, &path, None, Some(bytes_body), None)
                .await?;

            let val: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
                CliError::General(format!("Failed to parse comment edit response: {e}"))
            })?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
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
            let path = format!("/comments/{comment_id}");

            let (status, _headers, _bytes, rate_limit) = client
                .execute_request(reqwest::Method::DELETE, &path, None, None, None)
                .await?;

            if !status.is_success() {
                return Err(CliError::General(format!(
                    "Failed to delete comment '{comment_id}'"
                )));
            }

            if output.json {
                let res = json!({ "status": "deleted", "id": comment_id });
                output.print_json(&res, Some(rate_limit));
            } else {
                println!("Comment '{comment_id}' deleted.");
            }
        }
    }

    Ok(())
}

fn print_comment_tree(
    node: &actos_types::content::CommentNodeResponse,
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
