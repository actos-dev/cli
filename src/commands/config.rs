use crate::cli::ConfigAction;
use crate::config::Config;
use crate::error::CliError;
use comfy_table::{Table, presets::UTF8_FULL};

/// `actos user` — aktif hesabı listeler/değiştirir (SIKAYETLER #10).
///
/// İsimsiz çağrı hesapları listeler (anahtar göstermez, ağa çıkmaz);
/// isimli çağrı `default_profile`'ı değiştirir. Kim kiminle konuşuyor
/// her zaman açıkça basılır.
pub fn handle_user(name: Option<&str>, is_json: bool) -> Result<(), CliError> {
    let mut config = Config::load()?;

    match name {
        None => {
            if is_json {
                let profiles: Vec<serde_json::Value> = config
                    .profiles
                    .iter()
                    .map(|(pname, prof)| {
                        serde_json::json!({
                            "name": pname,
                            "username": prof.username,
                            "actor_type": prof.actor_type,
                            "active": pname == &config.default_profile,
                        })
                    })
                    .collect();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "default_profile": config.default_profile,
                        "profiles": profiles,
                    }))
                    .unwrap_or_default()
                );
            } else {
                let mut table = Table::new();
                table.load_preset(UTF8_FULL);
                table.set_header(vec!["Account", "Username", "Actor Type"]);

                for (pname, prof) in &config.profiles {
                    let display = if pname == &config.default_profile {
                        format!("{pname} *")
                    } else {
                        pname.clone()
                    };
                    table.add_row(vec![
                        display,
                        prof.username
                            .as_deref()
                            .unwrap_or("(no identity)")
                            .to_string(),
                        prof.actor_type.as_deref().unwrap_or("-").to_string(),
                    ]);
                }

                println!("{table}");
                println!("\nActive account: '{}'.", config.default_profile);
                println!("Switch with: actos user <account>");
            }
        }
        Some(target) => {
            if !config.profiles.contains_key(target) {
                let mut known: Vec<&String> = config.profiles.keys().collect();
                known.sort();
                let known = known
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(CliError::NotFound(format!(
                    "Account '{target}' not found. Known accounts: {known}"
                )));
            }
            config.default_profile = target.to_string();
            config.save()?;

            let who = config
                .profiles
                .get(target)
                .and_then(|p| p.username.clone())
                .map(|u| format!("@{u}"))
                .unwrap_or_else(|| "(no identity saved yet)".to_string());
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "switched",
                        "active": target,
                        "username": who,
                    }))
                    .unwrap_or_default()
                );
            } else {
                println!("Active account: '{target}' ({who}).");
                println!("Subsequent commands run as this identity.");
            }
        }
    }

    Ok(())
}

/// `actos config` alt komutlarını çalıştırır.
pub fn handle_config(
    action: ConfigAction,
    cli_profile: Option<&str>,
    is_json: bool,
) -> Result<(), CliError> {
    let mut config = Config::load()?;

    match action {
        ConfigAction::List { profile } => {
            let filter_profile = profile.as_deref().or(cli_profile);
            let masked = config.masked();

            if is_json {
                let output = if let Some(prof_name) = filter_profile {
                    let prof = masked.profiles.get(prof_name).ok_or_else(|| {
                        CliError::NotFound(format!("Profile '{prof_name}' not found"))
                    })?;
                    serde_json::json!({
                        "default_profile": masked.default_profile,
                        "profile": prof_name,
                        "config": prof
                    })
                } else {
                    serde_json::to_value(&masked).map_err(|e| {
                        CliError::General(format!("Failed to serialize config to JSON: {e}"))
                    })?
                };
                println!(
                    "{}",
                    serde_json::to_string_pretty(&output).unwrap_or_default()
                );
            } else if let Some(prof_name) = filter_profile {
                let prof = masked.profiles.get(prof_name).ok_or_else(|| {
                    CliError::NotFound(format!("Profile '{prof_name}' not found"))
                })?;

                println!("Profile: {prof_name}");
                println!(
                    "API URL:    {}",
                    prof.api_url.as_deref().unwrap_or("(not set)")
                );
                println!(
                    "API Key:    {}",
                    prof.api_key.as_deref().unwrap_or("(not set)")
                );
                println!(
                    "Username:   {}",
                    prof.username.as_deref().unwrap_or("(not set)")
                );
                println!(
                    "Actor Type: {}",
                    prof.actor_type.as_deref().unwrap_or("(not set)")
                );
            } else {
                println!("Default profile: {}\n", masked.default_profile);

                let mut table = Table::new();
                table.load_preset(UTF8_FULL);
                table.set_header(vec![
                    "Profile",
                    "API URL",
                    "API Key",
                    "Username",
                    "Actor Type",
                ]);

                for (name, prof) in &masked.profiles {
                    let display_name = if name == &masked.default_profile {
                        format!("{name} *")
                    } else {
                        name.clone()
                    };

                    table.add_row(vec![
                        display_name,
                        prof.api_url.as_deref().unwrap_or("-").to_string(),
                        prof.api_key.as_deref().unwrap_or("-").to_string(),
                        prof.username.as_deref().unwrap_or("-").to_string(),
                        prof.actor_type.as_deref().unwrap_or("-").to_string(),
                    ]);
                }

                println!("{table}");
            }
        }

        ConfigAction::Get { key, profile } => {
            let target_profile = profile
                .as_deref()
                .or(cli_profile)
                .unwrap_or(config.default_profile.as_str());

            let val = config.get(target_profile, &key)?;

            if is_json {
                let json = serde_json::json!({
                    "key": key,
                    "value": val,
                    "profile": target_profile,
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json).unwrap_or_default()
                );
            } else {
                println!("{val}");
            }
        }

        ConfigAction::Set {
            key,
            value,
            profile,
        } => {
            let target_profile = profile
                .as_deref()
                .or(cli_profile)
                .unwrap_or(&config.default_profile)
                .to_string();

            config.set(&target_profile, &key, &value)?;
            config.save()?;

            if is_json {
                let json = serde_json::json!({
                    "key": key,
                    "value": value,
                    "profile": target_profile,
                    "updated": true,
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json).unwrap_or_default()
                );
            } else if key == "default_profile" {
                println!("Default profile set to '{value}'.");
            } else {
                println!("Config '{key}' set to '{value}' (profile: {target_profile}).");
            }
        }
    }

    Ok(())
}
