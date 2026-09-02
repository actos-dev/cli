use comfy_table::{Table, presets::UTF8_FULL};
use reqwest::Method;
use serde_json::json;

use crate::client::ApiClient;
use crate::error::CliError;
use crate::output::OutputContext;

pub async fn handle_docs(
    open: bool,
    client: &ApiClient,
    output: &OutputContext,
) -> Result<(), CliError> {
    let docs_url = format!("{}/docs", client.base_url().trim_end_matches('/'));

    if open {
        #[cfg(target_os = "linux")]
        let _ = std::process::Command::new("xdg-open")
            .arg(&docs_url)
            .spawn();

        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(&docs_url).spawn();

        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("explorer")
            .arg(&docs_url)
            .spawn();

        if output.json {
            let res = json!({ "status": "opened", "url": docs_url });
            output.print_json(&res, None);
        } else {
            println!("Opened {docs_url} in browser.");
        }
        return Ok(());
    }

    let (_status, _headers, bytes, rate_limit) = client
        .execute_request(Method::GET, "/docs/agent", None, None, None)
        .await?;

    let text = String::from_utf8_lossy(&bytes).to_string();

    if output.json {
        let res = json!({ "content": text });
        output.print_json(&res, Some(rate_limit));
    } else {
        print!("{text}");
    }

    Ok(())
}

pub async fn handle_quota(client: &ApiClient, output: &OutputContext) -> Result<(), CliError> {
    let path = if client.api_key().is_some() {
        "/auth/whoami"
    } else {
        "/health"
    };

    let (_status, _headers, _bytes, rate_limit) = client
        .execute_request(Method::GET, path, None, None, None)
        .await?;

    if output.json {
        let res = json!({
            "limit": rate_limit.limit,
            "remaining": rate_limit.remaining,
            "reset_timestamp": rate_limit.reset,
        });
        output.print_json(&res, Some(rate_limit));
    } else {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(vec!["Quota Metric", "Value"]);

        let auth_status = if client.api_key().is_some() {
            "Authenticated"
        } else {
            "Anonymous (IP-based)"
        };

        table.add_row(vec!["Tier", auth_status]);
        table.add_row(vec![
            "Limit",
            &rate_limit
                .limit
                .map_or_else(|| "unlimited".to_string(), |v| v.to_string()),
        ]);
        table.add_row(vec![
            "Remaining",
            &rate_limit
                .remaining
                .map_or_else(|| "unknown".to_string(), |v| v.to_string()),
        ]);
        table.add_row(vec![
            "Reset Timestamp",
            &rate_limit
                .reset
                .map_or_else(|| "none".to_string(), |v| v.to_string()),
        ]);

        println!("{table}");
    }

    Ok(())
}

pub async fn handle_version(client: &ApiClient, output: &OutputContext) -> Result<(), CliError> {
    let cli_version = env!("CARGO_PKG_VERSION");
    let target_api = "v1";

    let server_info = match client.get_json("/version", None).await {
        Ok((val, _rl)) => Some(val),
        Err(_) => None,
    };

    if output.json {
        let res = json!({
            "cli": {
                "name": "actos",
                "version": cli_version,
                "target_api": target_api,
            },
            "server": server_info,
        });
        output.print_json(&res, None);
    } else {
        println!("Actos CLI v{cli_version} (Target API: {target_api})");
        if let Some(srv) = server_info {
            let name = srv["name"].as_str().unwrap_or("actos-api");
            let ver = srv["version"].as_str().unwrap_or("unknown");
            let sha = srv["git_sha"].as_str().unwrap_or("-");
            let api_ver = srv["api_version"].as_str().unwrap_or(target_api);
            println!("Server:    {name} v{ver} (commit: {sha}, API: {api_ver})");
        } else {
            println!("Server:    unreachable or offline");
        }
    }

    Ok(())
}
