use std::collections::HashSet;
use std::io::Write;
use std::time::Duration;

use crate::cli::WatchArgs;
use crate::client::ApiClient;
use crate::error::CliError;
use crate::output::OutputContext;

/// `actos watch` — bildirimleri JSONL akışı olarak izler (Faz 17).
///
/// Sunucuda **push/SSE yoktur**; bu bir **yoklama döngüsüdür**: her
/// `--interval` saniyede `GET /me/inbox`'ı yeniden sorgular. Hız sınırı
/// (`Retry-After` / `X-RateLimit-*` başlıkları) ve 5xx/network geri çekilmesi
/// paylaşılan `ApiClient::execute_request` tarafından yönetilir — kendi
/// döngüsünü yeniden yazmaz (bkz. `src/client/ratelimit.rs`, `retry.rs`).
///
/// Çıktı: her yeni (daha önce görülmemiş) bildirim için stdout'a tek satır
/// JSON (JSONL). Aynı id iki yoklamada da görünürse bir kez basılır.
///
/// Döngünün gövdesi `poll_once`'a ayrılmıştır; döngünün kendisi sonsuzdur
/// (Ctrl-C ile durur). Sınırlı, ölçeklenebilir yoklama testleri için
/// `scan_polls`'a bakın.
pub async fn handle_watch(
    args: WatchArgs,
    client: &ApiClient,
    output: &OutputContext,
    limit: u32,
) -> Result<(), CliError> {
    if client.api_key().is_none() {
        return Err(CliError::Auth(
            "Authentication required to watch your inbox. Run 'actos auth login' or set ACTOS_API_KEY."
                .to_string(),
        ));
    }

    eprintln!(
        "Watching inbox — polling every {}s (server has no push/SSE; this command polls). Press Ctrl-C to stop.",
        args.interval
    );

    let limit_str = limit.to_string();
    let mut seen: HashSet<String> = HashSet::new();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    loop {
        let mut query: Vec<(&str, &str)> = vec![("limit", limit_str.as_str())];
        if args.unread {
            query.push(("unread", "true"));
        }

        poll_once(client, &query, &mut seen, &mut out).await?;

        if output.verbose {
            eprintln!("Polled inbox (seen {} unique notification(s)).", seen.len());
        }

        tokio::time::sleep(Duration::from_secs(args.interval)).await;
    }
}

/// Tek bir `GET /me/inbox` yoklaması yapar.
///
/// Yanıttaki her **yeni** (henüz `seen` içinde olmayan) bildirimi tek bir JSON
/// satırı (JSONL) olarak `out`'a yazar. `seen` yoklamalar arasında
/// paylaşıldığı için aynı id iki kez basılmaz. Bu fonksiyon hem sonsuz
/// `handle_watch` döngüsü hem de sınırlı test senaryoları tarafından kullanılır.
pub async fn poll_once<W: Write>(
    client: &ApiClient,
    query: &[(&str, &str)],
    seen: &mut HashSet<String>,
    out: &mut W,
) -> Result<(), CliError> {
    let (val, _rate_limit) = client
        .get_json("/me/inbox", Some(query))
        .await
        .map_err(|e| CliError::General(format!("watch stopped: {e}")))?;

    let notifications = val
        .get("notifications")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for n in notifications {
        let id = n
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if id.is_empty() || !seen.insert(id) {
            continue;
        }
        let line = serde_json::to_string(&n)
            .map_err(|e| CliError::General(format!("Failed to serialize notification: {e}")))?;
        writeln!(out, "{line}").map_err(|e| CliError::Io(e.to_string()))?;
        out.flush().map_err(|e| CliError::Io(e.to_string()))?;
    }

    Ok(())
}

/// Sınırlı sayıda yoklama yapar ve bastığı tüm JSONL satırlarını döndürür.
///
/// `handle_watch` sonsuz döngü olduğu için doğrudan test edilemez; bu fonksiyon
/// döngü gövdesini **uçu dönmeden** test edilebilir kılar: aynı `seen` kümesi
/// yoklamalar arasında korunur, böylece dedupe davranışı da kapsanır. Boş bir
/// yoklamadan satır (yeni bildirim yoksa boş blok) üretilmez.
pub async fn scan_polls(
    client: &ApiClient,
    query: &[(&str, &str)],
    polls: usize,
) -> Result<Vec<String>, CliError> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut lines = Vec::new();

    for _ in 0..polls {
        let mut buf = Vec::new();
        poll_once(client, query, &mut seen, &mut buf).await?;
        if !buf.is_empty() {
            lines.push(String::from_utf8_lossy(&buf).into_owned());
        }
    }

    Ok(lines)
}
