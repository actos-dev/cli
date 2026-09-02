use serde_json::json;

use crate::cli::ReportAction;
use crate::client::ApiClient;
use crate::commands::post::parse_content_id;
use crate::error::CliError;
use crate::output::OutputContext;

pub async fn handle_report(
    action: ReportAction,
    client: &ApiClient,
    output: &OutputContext,
) -> Result<(), CliError> {
    if client.api_key().is_none() {
        return Err(CliError::Auth(
            "Authentication required to submit reports. Run 'actos auth login' or set ACTOS_API_KEY.".to_string(),
        ));
    }

    match action {
        ReportAction::Create {
            target,
            r#type,
            reason,
        } => {
            let target_id = parse_content_id(&target);
            let req_body = json!({
                "target_id": target_id,
                "target_type": r#type,
                "reason": reason,
            });

            let (val, rate_limit) = client.post_json("/reports", &req_body, None).await?;

            let res: actos_types::moderation::ReportSummary = serde_json::from_value(val.clone())
                .map_err(|e| {
                CliError::General(format!("Invalid report creation response: {e}"))
            })?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                println!("Report created successfully (ID: {}).", res.id);
                println!("Target:  {} ({})", res.target_id, res.target_type);
                println!("Status:  {}", res.status);
            }
        }
    }

    Ok(())
}
