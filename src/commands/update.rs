//! `actos update` — sürüm denetimi ve cargo ile kendini güncelleme.
use std::process::Stdio;

use serde_json::json;

use crate::cli::UpdateArgs;
use crate::error::CliError;
use crate::output::OutputContext;

const CRATE_NAME: &str = "actos-cli";
/// crates.io sparse index girdisi (satır başına bir JSON sürüm kaydı).
const INDEX_URL: &str = "https://index.crates.io/ac/to/actos-cli";

/// "1.2.3[-ön-ek]" ayrıştırır. Bozuk girdi `None` döner (o zaman
/// karşılaştırma "bilinmiyor" sayılır, asla "yeni" sayılmaz).
fn parse_version(s: &str) -> Option<(Vec<u64>, Option<String>)> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let (nums, pre) = match s.split_once('-') {
        Some((n, p)) => (n, Some(p.to_string())),
        None => (s, None),
    };
    if nums.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    for seg in nums.split('.') {
        parts.push(seg.parse::<u64>().ok()?);
    }
    Some((parts, pre))
}

/// `latest`, `installed`'dan gerçekten yeniyse `true`.
///
/// Ayrıştırılamayan tarafta güvenli yöne yatılır (`false`): bilinmeyen
/// sürümü "yeni" sayıp kurulum tetiklenmez.
#[must_use]
pub fn is_newer(installed: &str, latest: &str) -> bool {
    let (Some((mut a, a_pre)), Some((mut b, b_pre))) =
        (parse_version(installed), parse_version(latest))
    else {
        return false;
    };
    let width = a.len().max(b.len());
    a.resize(width, 0);
    b.resize(width, 0);
    if a != b {
        return a < b;
    }
    // Sayısal eşitlikte ön-ekli olan eskidir ("1.0-beta" < "1.0").
    a_pre.is_some() && b_pre.is_none()
}

/// Index satırlarından yanked olmayan en yeni sürümü seçer.
fn pick_latest(body: &str) -> Option<String> {
    let mut best: Option<(Vec<u64>, Option<String>, String)> = None;
    for line in body.lines() {
        let val: serde_json::Value = serde_json::from_str(line).ok()?;
        if val.get("yanked").and_then(|v| v.as_bool()).unwrap_or(false) {
            continue;
        }
        let vers = val.get("vers")?.as_str()?;
        let (nums, pre) = parse_version(vers)?;
        let candidate = (nums, pre, vers.to_string());
        let take = match &best {
            None => true,
            Some((bn, bp, _)) => {
                let width = bn.len().max(candidate.0.len());
                let mut a = bn.clone();
                let mut b = candidate.0.clone();
                a.resize(width, 0);
                b.resize(width, 0);
                (b.clone(), candidate.1.clone()) > (a, bp.clone())
            }
        };
        if take {
            best = Some(candidate);
        }
    }
    best.map(|(_, _, vers)| vers)
}

/// crates.io sparse index'ten yayınlanmış en yeni sürümü okur.
pub async fn fetch_latest() -> Result<String, CliError> {
    let body = reqwest::get(INDEX_URL)
        .await
        .map_err(|e| CliError::Network(format!("Could not reach crates.io index: {e}")))?
        .text()
        .await
        .map_err(|e| CliError::Network(format!("Could not read crates.io index: {e}")))?;
    pick_latest(&body).ok_or_else(|| {
        CliError::Network("crates.io index has no usable actos-cli release.".to_string())
    })
}

pub async fn handle_update(args: UpdateArgs, output: &OutputContext) -> Result<(), CliError> {
    let installed = env!("CARGO_PKG_VERSION");
    let latest = match args.version {
        Some(v) => v,
        None => fetch_latest().await?,
    };
    let available = is_newer(installed, &latest);

    if output.json {
        if args.check || !available {
            output.print_json(
                &json!({
                    "installed": installed,
                    "latest": latest,
                    "update_available": available,
                }),
                None,
            );
            return Ok(());
        }
    } else if args.check || !available {
        if available {
            println!("Update available: {installed} -> {latest}");
            println!("Run 'actos update' to install it.");
        } else {
            println!("Already up to date ({installed}).");
        }
        return Ok(());
    }

    if !output.json {
        println!("Updating actos {installed} -> {latest} ...");
    }
    let mut cmd = std::process::Command::new("cargo");
    cmd.args(["install", CRATE_NAME, "--locked", "--version", &latest]);
    if args.force {
        cmd.arg("--force");
    }
    if output.json {
        // stdout saf JSON kalmalı: cargo çıktısı stderr'e, stdout yutlur.
        cmd.stdout(Stdio::null()).stderr(Stdio::inherit());
    }
    let status = cmd.status().map_err(|e| {
        CliError::Usage(format!(
            "Could not run 'cargo' ({e}). Install Rust first: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        ))
    })?;
    if !status.success() {
        return Err(CliError::General(
            "cargo install failed. On Windows, close any running actos first (the OS locks the binary), then retry.".to_string(),
        ));
    }

    if output.json {
        output.print_json(
            &json!({
                "status": "updated",
                "installed": latest,
                "latest": latest,
                "update_available": false,
            }),
            None,
        );
    } else {
        println!("Updated to {latest}. Verify with: actos --version");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer() {
        assert!(is_newer("0.1.1", "0.2.0"));
        assert!(is_newer("0.2.0", "0.2.1"));
        assert!(is_newer("0.2", "0.2.1"));
        assert!(is_newer("1.0-beta", "1.0.0"));
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("0.2.0", "0.1.1"));
        assert!(!is_newer("garbage", "9.9.9"));
        assert!(!is_newer("0.2.0", "garbage"));
        assert!(!is_newer("1.0.0", "1.0-beta"));
    }

    #[test]
    fn test_pick_latest_skips_yanked() {
        let body = [
            r#"{"name":"actos-cli","vers":"0.1.0","yanked":false}"#,
            r#"{"name":"actos-cli","vers":"0.3.0","yanked":true}"#,
            r#"{"name":"actos-cli","vers":"0.2.0","yanked":false}"#,
        ]
        .join("\n");
        assert_eq!(pick_latest(&body), Some("0.2.0".to_string()));
        assert_eq!(pick_latest("not json\n"), None);
    }
}
