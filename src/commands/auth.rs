use comfy_table::{Table, presets::UTF8_FULL};
use serde_json::json;
use std::io::Read;

use crate::cli::{AuthAction, KeysAction, RecoveryAction};
use crate::client::ApiClient;
use crate::config::Config;
use crate::error::CliError;
use crate::output::OutputContext;

pub async fn handle_auth(
    action: AuthAction,
    client: &ApiClient,
    config: &mut Config,
    output: &OutputContext,
    target_profile: &str,
) -> Result<(), CliError> {
    match action {
        AuthAction::Register {
            username,
            r#type,
            display_name,
            save,
        } => {
            let req_body = json!({
                "username": username,
                "actor_type": r#type,
                "display_name": display_name,
            });

            let (val, rate_limit) = client.post_json("/auth/register", &req_body, None).await?;

            let res: actos_types::auth::RegisterResponse = serde_json::from_value(val.clone())
                .map_err(|e| CliError::General(format!("Invalid register response: {e}")))?;

            if save {
                let _ = config.set(target_profile, "api_key", &res.api_key);
                let _ = config.set(target_profile, "username", &res.actor.username);
                let _ = config.set(target_profile, "actor_type", &res.actor.actor_type);
                config.save()?;
            }

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                println!("Account registered successfully!");
                println!("Username:     {}", res.actor.username);
                println!("Actor ID:     {}", res.actor.id);
                println!("Actor Type:   {}", res.actor.actor_type);
                if let Some(dn) = &res.actor.display_name {
                    println!("Display Name: {dn}");
                }
                println!("\nAPI Key (save this now!):");
                println!("{}", res.api_key);

                println!(
                    "\nRecovery Codes (IMPORTANT: save these now, they will NEVER be shown again!):"
                );
                for (i, code) in res.recovery_codes.iter().enumerate() {
                    println!("{:2}. {code}", i + 1);
                }

                if save {
                    println!("\nCredentials saved to profile '{target_profile}'.");
                }
            }
        }

        AuthAction::Login { key, stdin } => {
            let api_key = if stdin {
                let mut buffer = String::new();
                std::io::stdin()
                    .read_to_string(&mut buffer)
                    .map_err(|e| CliError::Io(format!("Failed to read key from stdin: {e}")))?;
                buffer.trim().to_string()
            } else if let Some(k) = key {
                if !output.json {
                    eprintln!(
                        "warning: passing API key via CLI argument is visible in process monitors (e.g. ps aux). Use '--stdin' instead."
                    );
                }
                k.trim().to_string()
            } else {
                return Err(CliError::Usage(
                    "Please provide an API key using '--stdin' or '--key <key>'.".to_string(),
                ));
            };

            if api_key.is_empty() {
                return Err(CliError::Auth("Provided API key is empty.".to_string()));
            }

            // Doğrulamak için geçici bir ApiClient ile whoami çağrısı yap
            let test_client = ApiClient::new(
                client.base_url().to_string(),
                Some(api_key.clone()),
                30,
                false,
                false,
            )?;

            let (val, rate_limit) = test_client.get_json("/auth/whoami", None).await?;
            let whoami: actos_types::auth::WhoamiResponse = serde_json::from_value(val)
                .map_err(|e| CliError::General(format!("Invalid whoami response: {e}")))?;

            let _ = config.set(target_profile, "api_key", &api_key);
            let _ = config.set(target_profile, "username", &whoami.actor.username);
            let _ = config.set(target_profile, "actor_type", &whoami.actor.actor_type);
            config.save()?;

            if output.json {
                let json_val = json!({
                    "status": "logged_in",
                    "profile": target_profile,
                    "actor": whoami.actor,
                });
                output.print_json(&json_val, Some(rate_limit));
            } else {
                println!(
                    "Logged in as {} ({}) to profile '{}'.",
                    whoami.actor.username, whoami.actor.actor_type, target_profile
                );
            }
        }

        AuthAction::Whoami => {
            if client.api_key().is_none() {
                return Err(CliError::Auth(
                    "Authentication required: no API key found. Run 'actos auth login' or set ACTOS_API_KEY.".to_string(),
                ));
            }

            let (val, rate_limit) = client.get_json("/auth/whoami", None).await?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                let whoami: actos_types::auth::WhoamiResponse = serde_json::from_value(val)
                    .map_err(|e| CliError::General(format!("Invalid whoami response: {e}")))?;

                println!("Actor ID:     {}", whoami.actor.id);
                println!("Username:     {}", whoami.actor.username);
                println!("Actor Type:   {}", whoami.actor.actor_type);
                if let Some(dn) = &whoami.actor.display_name {
                    println!("Display Name: {dn}");
                }
                let roles_str = if whoami.roles.is_empty() {
                    "none".to_string()
                } else {
                    whoami.roles.join(", ")
                };
                println!("Roles:        {roles_str}");
                println!("\nActive Key:");
                println!("Key ID:       {}", whoami.key.id);
                println!(
                    "Label:        {}",
                    whoami.key.label.as_deref().unwrap_or("-")
                );
                println!("Created At:   {}", whoami.key.created_at);
            }
        }

        AuthAction::Keys { action } => match action {
            KeysAction::List => {
                if client.api_key().is_none() {
                    return Err(CliError::Auth(
                        "Authentication required: no API key found.".to_string(),
                    ));
                }

                let (val, rate_limit) = client.get_json("/auth/keys", None).await?;

                if output.json {
                    output.print_json(&val, Some(rate_limit));
                } else {
                    let list: actos_types::auth::ListKeysResponse = serde_json::from_value(val)
                        .map_err(|e| {
                            CliError::General(format!("Invalid keys list response: {e}"))
                        })?;

                    let mut table = Table::new();
                    table.load_preset(UTF8_FULL);
                    table.set_header(vec![
                        "Key ID",
                        "Label",
                        "Created At",
                        "Last Used At",
                        "Revoked At",
                    ]);

                    for k in &list.keys {
                        table.add_row(vec![
                            k.id.clone(),
                            k.label.as_deref().unwrap_or("-").to_string(),
                            k.created_at.clone(),
                            k.last_used_at.as_deref().unwrap_or("-").to_string(),
                            k.revoked_at.as_deref().unwrap_or("-").to_string(),
                        ]);
                    }

                    println!("{table}");
                }
            }

            KeysAction::Create { label } => {
                if client.api_key().is_none() {
                    return Err(CliError::Auth(
                        "Authentication required: no API key found.".to_string(),
                    ));
                }

                let req = json!({ "label": label });
                let (val, rate_limit) = client.post_json("/auth/keys", &req, None).await?;

                if output.json {
                    output.print_json(&val, Some(rate_limit));
                } else {
                    let res: actos_types::auth::CreateKeyResponse = serde_json::from_value(val)
                        .map_err(|e| {
                            CliError::General(format!("Invalid key creation response: {e}"))
                        })?;

                    println!("API Key created successfully!");
                    println!("Key ID:   {}", res.key.id);
                    println!("Label:    {}", res.key.label.as_deref().unwrap_or("-"));
                    println!("\nSecret Key (only displayed once!):");
                    println!("{}", res.api_key);
                }
            }

            KeysAction::Revoke { key_id } => {
                if client.api_key().is_none() {
                    return Err(CliError::Auth(
                        "Authentication required: no API key found.".to_string(),
                    ));
                }

                let path = format!("/auth/keys/{key_id}");
                let (status, _headers, _bytes, rate_limit) = client
                    .execute_request(reqwest::Method::DELETE, &path, None, None, None)
                    .await?;

                if !status.is_success() {
                    return Err(CliError::General(format!(
                        "Failed to revoke key '{key_id}'"
                    )));
                }

                if output.json {
                    let res = json!({ "status": "revoked", "key_id": key_id });
                    output.print_json(&res, Some(rate_limit));
                } else {
                    println!("API key '{key_id}' revoked.");
                }
            }
        },

        AuthAction::Recover {
            username,
            code,
            save,
        } => {
            let req = json!({
                "username": username,
                "recovery_code": code,
            });

            let (val, rate_limit) = client.post_json("/auth/recover", &req, None).await?;

            let res: actos_types::auth::RecoverResponse = serde_json::from_value(val.clone())
                .map_err(|e| CliError::General(format!("Invalid recovery response: {e}")))?;

            if save {
                let _ = config.set(target_profile, "api_key", &res.api_key);
                let _ = config.set(target_profile, "username", &username);
                config.save()?;
            }

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                println!("Account recovered successfully!");
                println!("New API Key (displayed once):");
                println!("{}", res.api_key);
                println!("Remaining recovery codes: {}", res.remaining_recovery_codes);
                if save {
                    println!("\nNew key saved to profile '{target_profile}'.");
                }
            }
        }

        AuthAction::Recovery { action } => match action {
            RecoveryAction::Regenerate => {
                if client.api_key().is_none() {
                    return Err(CliError::Auth(
                        "Authentication required: no API key found.".to_string(),
                    ));
                }

                let (val, rate_limit) = client
                    .post_json("/auth/recovery-codes/regenerate", &json!({}), None)
                    .await?;

                if output.json {
                    output.print_json(&val, Some(rate_limit));
                } else {
                    let res: actos_types::auth::RegenerateRecoveryCodesResponse =
                        serde_json::from_value(val).map_err(|e| {
                            CliError::General(format!("Invalid regenerate recovery response: {e}"))
                        })?;

                    println!("10 new recovery codes generated (previous codes are now invalid!):");
                    for (i, code) in res.recovery_codes.iter().enumerate() {
                        println!("{:2}. {code}", i + 1);
                    }
                }
            }
        },

        AuthAction::Logout => {
            let _ = config.set(target_profile, "api_key", "");
            config.save()?;

            if output.json {
                let res = json!({ "status": "logged_out", "profile": target_profile });
                output.print_json(&res, None);
            } else {
                println!("Logged out from profile '{target_profile}'.");
            }
        }
    }

    Ok(())
}
