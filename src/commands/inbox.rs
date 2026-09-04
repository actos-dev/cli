use comfy_table::{Table, presets::UTF8_FULL};

use crate::cli::InboxAction;
use crate::client::ApiClient;
use crate::error::CliError;
use crate::output::OutputContext;

/// `actos inbox` — bildirimleri listeler ya da okundu işaretler (Faz 17).
///
/// Backend sözleşmesi (openapi Faz 18.A):
/// - `GET /me/inbox?unread=true&limit=&cursor=` liste ucu.
/// - `PATCH /me/inbox/{id}/read` tek bildirimi okundu işaretler (204).
/// - `POST /me/inbox/read` tüm bildirimleri okundu işaretler; `{marked}` döner
///   (zaten okunmuşlar sayılmaz — idempotent).
/// - `unread_count` **toplam** okunmamış sayıdır, bu sayfadaki öğe sayısı değil.
///   Bu ayrım özet satırında korunur.
/// - `target_type` post ve yorum için ikisi de `"content"`; ayrım `kind` alanıyla
///   yapılır (tabloda ayrı bir sütun olarak gösterilir).
pub async fn handle_inbox(
    action: InboxAction,
    client: &ApiClient,
    output: &OutputContext,
    limit: u32,
    cursor: Option<&str>,
) -> Result<(), CliError> {
    match action {
        InboxAction::List { unread } => {
            if client.api_key().is_none() {
                return Err(CliError::Auth(
                    "Authentication required to view your inbox. Run 'actos auth login' or set ACTOS_API_KEY."
                        .to_string(),
                ));
            }

            let mut query_params: Vec<(&str, &str)> = Vec::new();
            if unread {
                query_params.push(("unread", "true"));
            }

            let (val, rate_limit) = client
                .paginate("/me/inbox", &query_params, limit, cursor)
                .await?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
                return Ok(());
            }

            let resp: actos_types::notification::InboxResponse =
                serde_json::from_value(val.clone())
                    .map_err(|e| CliError::General(format!("Invalid inbox response: {e}")))?;

            if resp.notifications.is_empty() {
                println!("No notifications in your inbox.");
            } else {
                let mut table = Table::new();
                table.load_preset(UTF8_FULL);
                table.set_header(vec!["ID", "Kind", "From", "Target", "Read", "Created At"]);

                for n in &resp.notifications {
                    let id = n.id.as_str();
                    let kind = n.kind.as_str();
                    let from = n
                        .actor
                        .as_ref()
                        .map(|a| a.username.as_str())
                        .unwrap_or("system");
                    let target = format!("{}:{}", n.target_type, n.target_id);
                    let read = if n.read_at.is_some() { "yes" } else { "no" };
                    table.add_row(vec![
                        id,
                        kind,
                        from,
                        target.as_str(),
                        read,
                        n.created_at.as_str(),
                    ]);
                }

                println!("{table}");
            }

            // unread_count toplamdır — sayfadaki öğe sayısından ayrı gösterilir.
            println!(
                "Total unread: {} (this page shows {} notification(s))",
                resp.unread_count,
                resp.notifications.len()
            );
            Ok(())
        }

        InboxAction::Read { id, all } => {
            if client.api_key().is_none() {
                return Err(CliError::Auth(
                    "Authentication required to update your inbox. Run 'actos auth login' or set ACTOS_API_KEY."
                        .to_string(),
                ));
            }

            if all && id.is_some() {
                return Err(CliError::Usage(
                    "Provide either a single notification <id> or --all, not both.".to_string(),
                ));
            }
            if !all && id.is_none() {
                return Err(CliError::Usage(
                    "Provide a notification <id> to read, or --all to mark everything read."
                        .to_string(),
                ));
            }

            if all {
                let (val, rate_limit) = client
                    .post_json("/me/inbox/read", &serde_json::json!({}), None)
                    .await?;

                let resp: actos_types::notification::MarkAllReadResponse =
                    serde_json::from_value(val.clone()).map_err(|e| {
                        CliError::General(format!("Invalid mark-all-read response: {e}"))
                    })?;

                if output.json {
                    output.print_json(&val, Some(rate_limit));
                } else {
                    println!("Marked {} notification(s) as read.", resp.marked);
                }
            } else if let Some(notification_id) = id {
                let path = format!("/me/inbox/{notification_id}/read");
                let (status, _headers, _bytes, rate_limit) = client
                    .execute_request(reqwest::Method::PATCH, &path, None, None, None)
                    .await?;

                if !status.is_success() {
                    return Err(CliError::General(format!(
                        "Failed to mark notification '{notification_id}' as read"
                    )));
                }

                if output.json {
                    let res = serde_json::json!({ "status": "read", "id": notification_id });
                    output.print_json(&res, Some(rate_limit));
                } else {
                    println!("Notification '{notification_id}' marked as read.");
                }
            }

            Ok(())
        }
    }
}
