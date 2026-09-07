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
            let mut builder = client.auth().register(username, r#type);
            if let Some(dn) = display_name {
                builder = builder.display_name(dn);
            }

            let res = builder.send().await.map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if save {
                let _ = config.set(target_profile, "api_key", &res.api_key);
                let _ = config.set(target_profile, "username", &res.actor.username);
                let _ = config.set(target_profile, "actor_type", &res.actor.actor_type);
                config.save()?;
            }

            if output.json {
                let val = serde_json::to_value(&res)
                    .map_err(|e| CliError::General(format!("Failed to serialize register: {e}")))?;
                output.print_json(&val, Some(rl));
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

            // Doğrulamak için geçici bir SDK istemcisiyle whoami çağrısı yap
            let test_client = actos_sdk::Actos::builder()
                .base_url(client.base_url())
                .api_key(api_key.clone())
                .build()
                .map_err(CliError::from)?;
            let whoami = test_client.auth().whoami().await.map_err(CliError::from)?;
            let rate_limit = test_client.rate_limit().map(Into::into).unwrap_or_default();

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

            let whoami = client.auth().whoami().await.map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                let val = serde_json::to_value(&whoami)
                    .map_err(|e| CliError::General(format!("Failed to serialize whoami: {e}")))?;
                output.print_json(&val, Some(rl));
            } else {
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

                let keys = client.auth().list_keys().await.map_err(CliError::from)?;
                let rl = client.rate_limit_info().unwrap_or_default();

                if output.json {
                    let val = json!({ "keys": keys });
                    output.print_json(&val, Some(rl));
                } else {
                    let mut table = Table::new();
                    table.load_preset(UTF8_FULL);
                    table.set_header(vec![
                        "Key ID",
                        "Label",
                        "Created At",
                        "Last Used At",
                        "Revoked At",
                    ]);

                    for k in &keys {
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

                let mut builder = client.auth().create_key();
                if let Some(l) = label {
                    builder = builder.label(l);
                }
                let res = builder.send().await.map_err(CliError::from)?;
                let rl = client.rate_limit_info().unwrap_or_default();

                if output.json {
                    let val = serde_json::to_value(&res).map_err(|e| {
                        CliError::General(format!("Failed to serialize key creation: {e}"))
                    })?;
                    output.print_json(&val, Some(rl));
                } else {
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

                client
                    .auth()
                    .revoke_key(&key_id)
                    .await
                    .map_err(CliError::from)?;
                let rl = client.rate_limit_info().unwrap_or_default();

                if output.json {
                    let res = json!({ "status": "revoked", "key_id": key_id });
                    output.print_json(&res, Some(rl));
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
            let res = client
                .auth()
                .recover(&username, &code)
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if save {
                let _ = config.set(target_profile, "api_key", &res.api_key);
                let _ = config.set(target_profile, "username", &username);
                config.save()?;
            }

            if output.json {
                let val = serde_json::to_value(&res)
                    .map_err(|e| CliError::General(format!("Failed to serialize recovery: {e}")))?;
                output.print_json(&val, Some(rl));
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

                let res = client
                    .auth()
                    .regenerate_recovery_codes()
                    .await
                    .map_err(CliError::from)?;
                let rl = client.rate_limit_info().unwrap_or_default();

                if output.json {
                    let val = serde_json::to_value(&res).map_err(|e| {
                        CliError::General(format!("Failed to serialize recovery codes: {e}"))
                    })?;
                    output.print_json(&val, Some(rl));
                } else {
                    println!("10 new recovery codes generated (previous codes are now invalid!):");
                    for (i, code) in res.recovery_codes.iter().enumerate() {
                        println!("{:2}. {code}", i + 1);
                    }
                }
            }
        },

        AuthAction::Logout => {
            // SIKAYETLER #3: yalnız anahtar değil, kimlik de gider.
            let _ = config.clear_key(target_profile, "api_key");
            let _ = config.clear_key(target_profile, "username");
            let _ = config.clear_key(target_profile, "actor_type");
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
