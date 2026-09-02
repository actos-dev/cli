use comfy_table::{Table, presets::UTF8_FULL};
use serde_json::json;

use crate::cli::{
    AdminAction, AdminBanAction, AdminContentAction, AdminReportsAction, AdminRoleAction,
};
use crate::client::ApiClient;
use crate::commands::post::parse_content_id;
use crate::error::CliError;
use crate::output::OutputContext;

pub async fn handle_admin(
    action: AdminAction,
    client: &ApiClient,
    output: &OutputContext,
    yes: bool,
    limit: u32,
    cursor: Option<&str>,
) -> Result<(), CliError> {
    if client.api_key().is_none() {
        return Err(CliError::Auth(
            "Authentication required for admin commands. Run 'actos auth login' or set ACTOS_API_KEY.".to_string(),
        ));
    }

    match action {
        AdminAction::Reports { action } => match action {
            AdminReportsAction::List { status } => {
                let mut query_params: Vec<(&str, &str)> = Vec::new();
                if let Some(ref s) = status {
                    query_params.push(("status", s.as_str()));
                }

                let (val, rate_limit) = client
                    .paginate("/admin/reports", &query_params, limit, cursor)
                    .await?;

                if output.json {
                    output.print_json(&val, Some(rate_limit));
                } else {
                    let reports_opt = val.get("reports").and_then(|v| v.as_array());

                    let mut table = Table::new();
                    table.load_preset(UTF8_FULL);
                    table.set_header(vec![
                        "ID",
                        "Target",
                        "Type",
                        "Status",
                        "Reason",
                        "Created At",
                    ]);

                    if let Some(reports) = reports_opt {
                        for r in reports {
                            let id = r["id"].as_str().unwrap_or("-");
                            let target = r["target_id"].as_str().unwrap_or("-");
                            let target_type = r["target_type"].as_str().unwrap_or("-");
                            let status = r["status"].as_str().unwrap_or("-");
                            let reason = r["reason"].as_str().unwrap_or("-");
                            let created_at = r["created_at"].as_str().unwrap_or("-");
                            table.add_row(vec![
                                id,
                                target,
                                target_type,
                                status,
                                reason,
                                created_at,
                            ]);
                        }
                    }

                    println!("{table}");
                }
            }

            AdminReportsAction::Update { id, status, notes } => {
                let path = format!("/admin/reports/{id}");
                let mut patch_json = json!({
                    "status": status,
                });
                if let Some(n) = notes {
                    patch_json["notes"] = json!(n);
                }

                let bytes_body = serde_json::to_vec(&patch_json).map_err(|e| {
                    CliError::Validation(format!("Failed to serialize report update: {e}"))
                })?;

                let (_status, _headers, bytes, rate_limit) = client
                    .execute_request(reqwest::Method::PATCH, &path, None, Some(bytes_body), None)
                    .await?;

                let val: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
                    CliError::General(format!("Failed to parse report update response: {e}"))
                })?;

                if output.json {
                    output.print_json(&val, Some(rate_limit));
                } else {
                    println!("Report '{id}' updated to '{status}'.");
                }
            }
        },

        AdminAction::Content { action } => match action {
            AdminContentAction::Delete { id, reason } => {
                // Ajan Sözleşmesi §2 kural 5
                if !yes {
                    return Err(CliError::Usage(
                        "Admin content deletion requires confirmation. Pass '--yes' to confirm."
                            .to_string(),
                    ));
                }

                let target_id = parse_content_id(&id);
                let path = format!("/admin/contents/{target_id}");
                let req_body = json!({ "reason": reason });
                let bytes_body = serde_json::to_vec(&req_body).map_err(|e| {
                    CliError::Validation(format!("Failed to serialize request: {e}"))
                })?;

                let (status, _headers, _bytes, rate_limit) = client
                    .execute_request(reqwest::Method::DELETE, &path, None, Some(bytes_body), None)
                    .await?;

                if !status.is_success() {
                    return Err(CliError::General(format!(
                        "Failed to delete content '{target_id}'"
                    )));
                }

                if output.json {
                    let res = json!({ "status": "deleted", "id": target_id });
                    output.print_json(&res, Some(rate_limit));
                } else {
                    println!("Content '{target_id}' deleted by moderator.");
                }
            }
        },

        AdminAction::Ban { action } => match action {
            AdminBanAction::Add {
                username,
                reason,
                expires,
            } => {
                let mut req_body = json!({
                    "username": username,
                    "reason": reason,
                });
                if let Some(exp) = expires {
                    req_body["expires_at"] = json!(exp);
                }

                let (val, rate_limit) = client.post_json("/admin/bans", &req_body, None).await?;

                if output.json {
                    output.print_json(&val, Some(rate_limit));
                } else {
                    println!("User '{username}' has been banned.");
                }
            }

            AdminBanAction::Remove { username } => {
                let path = format!("/admin/bans/{username}");
                let (status, _headers, _bytes, rate_limit) = client
                    .execute_request(reqwest::Method::DELETE, &path, None, None, None)
                    .await?;

                if !status.is_success() {
                    return Err(CliError::General(format!(
                        "Failed to remove ban for '{username}'"
                    )));
                }

                if output.json {
                    let res = json!({ "status": "unbanned", "username": username });
                    output.print_json(&res, Some(rate_limit));
                } else {
                    println!("Ban removed for user '{username}'.");
                }
            }
        },

        AdminAction::Role { action } => match action {
            AdminRoleAction::Grant { username, role } => {
                let req_body = json!({
                    "username": username,
                    "role": role,
                });

                let (val, rate_limit) = client.post_json("/admin/roles", &req_body, None).await?;

                if output.json {
                    output.print_json(&val, Some(rate_limit));
                } else {
                    println!("Role '{role}' granted to '{username}'.");
                }
            }

            AdminRoleAction::Revoke { username } => {
                let req_body = json!({
                    "username": username,
                    "role": serde_json::Value::Null,
                });

                let (val, rate_limit) = client.post_json("/admin/roles", &req_body, None).await?;

                if output.json {
                    output.print_json(&val, Some(rate_limit));
                } else {
                    println!("Roles revoked for user '{username}'.");
                }
            }
        },

        AdminAction::Actions => {
            let (val, rate_limit) = client
                .paginate("/admin/actions", &[], limit, cursor)
                .await?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                let actions_opt = val.get("actions").and_then(|v| v.as_array());

                let mut table = Table::new();
                table.load_preset(UTF8_FULL);
                table.set_header(vec![
                    "ID",
                    "Admin",
                    "Action",
                    "Target Type",
                    "Target ID",
                    "Reason",
                    "Date",
                ]);

                if let Some(actions) = actions_opt {
                    for a in actions {
                        let id = a["id"].as_str().unwrap_or("-");
                        let admin = a["admin_username"].as_str().unwrap_or("-");
                        let action = a["action_type"].as_str().unwrap_or("-");
                        let t_type = a["target_type"].as_str().unwrap_or("-");
                        let t_id = a["target_id"].to_string();
                        let reason = a["reason"].as_str().unwrap_or("-");
                        let date = a["created_at"].as_str().unwrap_or("-");
                        table.add_row(vec![id, admin, action, t_type, &t_id, reason, date]);
                    }
                }

                println!("{table}");
            }
        }
    }

    Ok(())
}
