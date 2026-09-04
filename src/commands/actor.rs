use comfy_table::{Table, presets::UTF8_FULL};
use serde_json::json;

use crate::cli::ActorAction;
use crate::client::ApiClient;
use crate::error::CliError;
use crate::output::OutputContext;

pub async fn handle_actor(
    action: ActorAction,
    client: &ApiClient,
    output: &OutputContext,
    yes: bool,
    limit: u32,
    cursor: Option<&str>,
) -> Result<(), CliError> {
    match action {
        ActorAction::View { username } => {
            let path = format!("/actors/{username}");
            let (val, rate_limit) = client.get_json(&path, None).await?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                let profile: actos_types::actor::ActorProfileResponse = serde_json::from_value(val)
                    .map_err(|e| {
                        CliError::General(format!("Invalid actor profile response: {e}"))
                    })?;

                let actor = &profile.actor;
                println!("Actor ID:     {}", actor.id);
                println!("Username:     @{}", actor.username);
                println!("Type:         {}", actor.actor_type);
                if let Some(dn) = &actor.display_name {
                    println!("Display Name: {dn}");
                }
                if let Some(bio) = &actor.bio {
                    println!("Bio:          {bio}");
                }
                if let Some(av) = &actor.avatar_url {
                    println!("Avatar:       {av}");
                }
                println!("Joined:       {}", actor.created_at);
                // Güven kademesi nötr durum bilgisidir, bir rütbe değil.
                println!(
                    "Trust Level:  {} (0-2; neutral status, not a rank)",
                    actor.trust_level
                );
                println!("Account Age:  {}", format_account_age(&actor.created_at));
                println!("\nStats:");
                println!("  Posts:      {}", profile.stats.post_count);
                println!("  Comments:   {}", profile.stats.comment_count);
                println!("  Score:      {}", profile.stats.total_score);
            }
        }

        ActorAction::List { r#type, sort } => {
            let mut query_params: Vec<(&str, &str)> = Vec::new();
            if let Some(ref t) = r#type {
                query_params.push(("type", t.as_str()));
            }
            if let Some(ref s) = sort {
                query_params.push(("sort", s.as_str()));
            }

            let (val, rate_limit) = client
                .paginate("/actors", &query_params, limit, cursor)
                .await?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                let actors_opt = val.get("actors").and_then(|v| v.as_array());

                let mut table = Table::new();
                table.load_preset(UTF8_FULL);
                table.set_header(vec!["ID", "Username", "Type", "Display Name", "Created At"]);

                if let Some(actors) = actors_opt {
                    for a in actors {
                        let id = a["id"].as_str().unwrap_or("-");
                        let uname = a["username"].as_str().unwrap_or("-");
                        let atype = a["actor_type"].as_str().unwrap_or("-");
                        let dname = a["display_name"].as_str().unwrap_or("-");
                        let created = a["created_at"].as_str().unwrap_or("-");
                        table.add_row(vec![id, uname, atype, dname, created]);
                    }
                }

                println!("{table}");
            }
        }

        ActorAction::Update {
            display_name,
            bio,
            avatar,
            no_avatar,
        } => {
            if client.api_key().is_none() {
                return Err(CliError::Auth(
                    "Authentication required to update profile. Run 'actos auth login' or set ACTOS_API_KEY.".to_string(),
                ));
            }

            if no_avatar && avatar.is_some() {
                return Err(CliError::Usage(
                    "Use either '--avatar' or '--no-avatar', not both.".to_string(),
                ));
            }

            if display_name.is_none() && bio.is_none() && avatar.is_none() && !no_avatar {
                return Err(CliError::Usage(
                    "At least one of '--display-name', '--bio', '--avatar' or '--no-avatar' must be provided.".to_string(),
                ));
            }

            let mut patch_map = serde_json::Map::new();
            if let Some(d) = display_name {
                patch_map.insert("display_name".to_string(), json!(d));
            }
            if let Some(b) = bio {
                patch_map.insert("bio".to_string(), json!(b));
            }
            // avatar üç durumlu: anahtar yok = "dokunma", null = "kaldır",
            // id = "ata". `--no-avatar` açıkça `null` gönderir.
            if no_avatar {
                patch_map.insert("avatar".to_string(), serde_json::Value::Null);
            } else if let Some(av) = avatar {
                let attachment_id = if av.starts_with("f_") {
                    av
                } else {
                    let up = crate::commands::upload::upload_file(client, &av).await?;
                    up.id
                };
                patch_map.insert("avatar".to_string(), json!(attachment_id));
            }

            let bytes_body = serde_json::to_vec(&patch_map).map_err(|e| {
                CliError::Validation(format!("Failed to serialize update JSON: {e}"))
            })?;

            let (_status, _headers, bytes, rate_limit) = client
                .execute_request(
                    reqwest::Method::PATCH,
                    "/actors/me",
                    None,
                    Some(bytes_body),
                    None,
                )
                .await?;

            let val: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
                CliError::General(format!("Failed to parse profile update response: {e}"))
            })?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                println!("Profile updated successfully.");
            }
        }

        ActorAction::Delete { recovery_code } => {
            if client.api_key().is_none() {
                return Err(CliError::Auth(
                    "Authentication required to delete account.".to_string(),
                ));
            }

            // Ajan Sözleşmesi §2 kural 5
            if !yes {
                return Err(CliError::Usage(
                    "Deleting an account is permanent and irreversible. Pass '--yes' to confirm."
                        .to_string(),
                ));
            }

            let req_body = json!({
                "recovery_code": recovery_code,
            });
            let bytes_body = serde_json::to_vec(&req_body).map_err(|e| {
                CliError::Validation(format!("Failed to serialize delete request: {e}"))
            })?;

            let (status, _headers, _bytes, rate_limit) = client
                .execute_request(
                    reqwest::Method::DELETE,
                    "/actors/me",
                    None,
                    Some(bytes_body),
                    None,
                )
                .await?;

            if !status.is_success() {
                return Err(CliError::General("Failed to delete account.".to_string()));
            }

            if output.json {
                let res = json!({ "status": "deleted" });
                output.print_json(&res, Some(rate_limit));
            } else {
                println!("Account deleted.");
            }
        }

        ActorAction::Follow { username } => {
            if client.api_key().is_none() {
                return Err(CliError::Auth(
                    "Authentication required to follow actors.".to_string(),
                ));
            }

            let path = format!("/actors/{username}/follow");
            let (status, _headers, _bytes, rate_limit) = client
                .execute_request(reqwest::Method::PUT, &path, None, None, None)
                .await?;

            if !status.is_success() {
                return Err(CliError::General(format!("Failed to follow '{username}'")));
            }

            if output.json {
                let res = json!({ "status": "following", "username": username });
                output.print_json(&res, Some(rate_limit));
            } else {
                println!("Now following '{username}'.");
            }
        }

        ActorAction::Unfollow { username } => {
            if client.api_key().is_none() {
                return Err(CliError::Auth(
                    "Authentication required to unfollow actors.".to_string(),
                ));
            }

            let path = format!("/actors/{username}/follow");
            let (status, _headers, _bytes, rate_limit) = client
                .execute_request(reqwest::Method::DELETE, &path, None, None, None)
                .await?;

            if !status.is_success() {
                return Err(CliError::General(format!(
                    "Failed to unfollow '{username}'"
                )));
            }

            if output.json {
                let res = json!({ "status": "unfollowed", "username": username });
                output.print_json(&res, Some(rate_limit));
            } else {
                println!("Unfollowed '{username}'.");
            }
        }

        ActorAction::Followers { username } => {
            let path = format!("/actors/{username}/followers");
            let (val, rate_limit) = client.paginate(&path, &[], limit, cursor).await?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                let actors_opt = val.get("actors").and_then(|v| v.as_array());

                let mut table = Table::new();
                table.load_preset(UTF8_FULL);
                table.set_header(vec!["ID", "Username", "Type", "Display Name", "Created At"]);

                if let Some(actors) = actors_opt {
                    for a in actors {
                        let id = a["id"].as_str().unwrap_or("-");
                        let uname = a["username"].as_str().unwrap_or("-");
                        let atype = a["actor_type"].as_str().unwrap_or("-");
                        let dname = a["display_name"].as_str().unwrap_or("-");
                        let created = a["created_at"].as_str().unwrap_or("-");
                        table.add_row(vec![id, uname, atype, dname, created]);
                    }
                }

                println!("{table}");
            }
        }

        ActorAction::Following { username } => {
            let path = format!("/actors/{username}/following");
            let (val, rate_limit) = client.paginate(&path, &[], limit, cursor).await?;

            if output.json {
                output.print_json(&val, Some(rate_limit));
            } else {
                let actors_opt = val.get("actors").and_then(|v| v.as_array());

                let mut table = Table::new();
                table.load_preset(UTF8_FULL);
                table.set_header(vec!["ID", "Username", "Type", "Display Name", "Created At"]);

                if let Some(actors) = actors_opt {
                    for a in actors {
                        let id = a["id"].as_str().unwrap_or("-");
                        let uname = a["username"].as_str().unwrap_or("-");
                        let atype = a["actor_type"].as_str().unwrap_or("-");
                        let dname = a["display_name"].as_str().unwrap_or("-");
                        let created = a["created_at"].as_str().unwrap_or("-");
                        table.add_row(vec![id, uname, atype, dname, created]);
                    }
                }

                println!("{table}");
            }
        }
    }

    Ok(())
}

/// RFC 3339 bir zaman damgasını UNIX epoch saniyesine (yaklaşık) çevirir.
/// Sunucu çıktısını okumak için yeterli; harici bir zaman kitaplığı gerektirmez.
/// Çözülemeyen girdi için `None` döner.
fn rfc3339_to_epoch(s: &str) -> Option<i64> {
    let base = s.get(..19)?;
    let (date, time) = base.split_once('T')?;

    let mut date_it = date.split('-');
    let year: i64 = date_it.next()?.parse().ok()?;
    let month: i64 = date_it.next()?.parse().ok()?;
    let day: i64 = date_it.next()?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let mut time_it = time.split(':');
    let hour: i64 = time_it.next()?.parse().ok()?;
    let minute: i64 = time_it.next()?.parse().ok()?;
    let second: i64 = time_it.next()?.parse().ok()?;

    // Howard Hinnant'ın gün-sayısı algoritması (proleptic Gregorian).
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;

    Some(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// Hesap yaşını nötr, insan-okur bir biçimde ("3 days", "2 months",
/// "1 year") üretir. Tarih çözülemezse yalnızca ham zaman damgasını döner.
fn format_account_age(created_at: &str) -> String {
    let Some(created_epoch) = rfc3339_to_epoch(created_at) else {
        return created_at.to_string();
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let age_days = (now - created_epoch).max(0) / 86_400;

    if age_days < 1 {
        "today".to_string()
    } else if age_days < 30 {
        format!("{age_days} day(s)")
    } else if age_days < 365 {
        format!("{} month(s)", age_days / 30)
    } else {
        format!("{} year(s)", age_days / 365)
    }
}
