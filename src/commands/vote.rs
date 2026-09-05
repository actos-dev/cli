use comfy_table::{Table, presets::UTF8_FULL};
use serde_json::json;

use crate::cli::VoteAction;
use crate::client::ApiClient;
use crate::commands::post::parse_content_id;
use crate::error::CliError;
use crate::output::OutputContext;

pub async fn handle_vote(
    action: VoteAction,
    client: &ApiClient,
    output: &OutputContext,
) -> Result<(), CliError> {
    if client.api_key().is_none() {
        return Err(CliError::Auth(
            "Authentication required to vote. Run 'actos auth login' or set ACTOS_API_KEY."
                .to_string(),
        ));
    }

    match action {
        VoteAction::Up { id } => {
            let content_id = parse_content_id(&id);
            let path = format!("/contents/{content_id}/vote");
            let req_body = json!({ "value": 1 });
            let bytes_body = serde_json::to_vec(&req_body)
                .map_err(|e| CliError::Validation(format!("Failed to serialize vote JSON: {e}")))?;

            let (_status, _headers, bytes, rate_limit) = client
                .execute_request(reqwest::Method::PUT, &path, None, Some(bytes_body), None)
                .await?;

            let val: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|e| CliError::General(format!("Failed to parse vote response: {e}")))?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                let res: actos_sdk::actos_types::interaction::VoteResponse =
                    serde_json::from_value(val)
                        .map_err(|e| CliError::General(format!("Invalid vote response: {e}")))?;

                println!(
                    "Upvoted '{content_id}'. Score: {} (+{} / -{})",
                    res.score, res.upvotes, res.downvotes
                );
            }
        }

        VoteAction::Down { id } => {
            let content_id = parse_content_id(&id);
            let path = format!("/contents/{content_id}/vote");
            let req_body = json!({ "value": -1 });
            let bytes_body = serde_json::to_vec(&req_body)
                .map_err(|e| CliError::Validation(format!("Failed to serialize vote JSON: {e}")))?;

            let (_status, _headers, bytes, rate_limit) = client
                .execute_request(reqwest::Method::PUT, &path, None, Some(bytes_body), None)
                .await?;

            let val: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|e| CliError::General(format!("Failed to parse vote response: {e}")))?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                let res: actos_sdk::actos_types::interaction::VoteResponse =
                    serde_json::from_value(val)
                        .map_err(|e| CliError::General(format!("Invalid vote response: {e}")))?;

                println!(
                    "Downvoted '{content_id}'. Score: {} (+{} / -{})",
                    res.score, res.upvotes, res.downvotes
                );
            }
        }

        VoteAction::Clear { id } => {
            let content_id = parse_content_id(&id);
            let path = format!("/contents/{content_id}/vote");
            let req_body = json!({ "value": 0 });
            let bytes_body = serde_json::to_vec(&req_body)
                .map_err(|e| CliError::Validation(format!("Failed to serialize vote JSON: {e}")))?;

            let (_status, _headers, bytes, rate_limit) = client
                .execute_request(reqwest::Method::PUT, &path, None, Some(bytes_body), None)
                .await?;

            let val: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|e| CliError::General(format!("Failed to parse vote response: {e}")))?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                let res: actos_sdk::actos_types::interaction::VoteResponse =
                    serde_json::from_value(val)
                        .map_err(|e| CliError::General(format!("Invalid vote response: {e}")))?;

                println!(
                    "Cleared vote for '{content_id}'. Score: {} (+{} / -{})",
                    res.score, res.upvotes, res.downvotes
                );
            }
        }

        VoteAction::Status { ids } => {
            let parsed_ids: Vec<String> = ids
                .split(',')
                .map(|s| parse_content_id(s.trim()))
                .filter(|s| !s.is_empty())
                .collect();

            let joined_ids = parsed_ids.join(",");
            let query = [("content_ids", joined_ids.as_str())];
            let (val, rate_limit) = client.get_json("/me/votes", Some(&query)).await?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                let vote_map: actos_sdk::actos_types::interaction::VoteMapResponse =
                    serde_json::from_value(val).map_err(|e| {
                        CliError::General(format!("Invalid vote map response: {e}"))
                    })?;

                let mut table = Table::new();
                table.load_preset(UTF8_FULL);
                table.set_header(vec!["Content ID", "Vote"]);

                for id in &parsed_ids {
                    let vote_str = match vote_map.votes.get(id) {
                        Some(1) => "+1 (Up)",
                        Some(-1) => "-1 (Down)",
                        _ => "0 (None)",
                    };
                    table.add_row(vec![id.as_str(), vote_str]);
                }

                println!("{table}");
            }
        }
    }

    Ok(())
}
