use serde_json::json;
use std::path::Path;

use crate::cli::UploadAction;
use crate::client::ApiClient;
use crate::error::CliError;
use crate::output::OutputContext;

pub const MAX_UPLOAD_BYTES: u64 = 8 * 1024 * 1024; // 8 MB

/// Dosyanın boyutunu ve uzantısını doğrular, dosya adı ve MIME türünü döner.
pub fn validate_and_read_file(path_str: &str) -> Result<(String, String, Vec<u8>), CliError> {
    let path = Path::new(path_str);
    if !path.is_file() {
        return Err(CliError::Validation(format!(
            "File '{path_str}' does not exist or is not a regular file"
        )));
    }

    let metadata = std::fs::metadata(path).map_err(|e| {
        CliError::Io(format!(
            "Failed to read file metadata for '{path_str}': {e}"
        ))
    })?;

    if metadata.len() > MAX_UPLOAD_BYTES {
        return Err(CliError::Validation(format!(
            "File size exceeds maximum allowed limit of 8 MB (actual: {} bytes)",
            metadata.len()
        )));
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    let mime = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => {
            return Err(CliError::Validation(format!(
                "Unsupported file extension '{ext}'. Supported: jpeg, png, gif, webp"
            )));
        }
    };

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("upload")
        .to_string();

    let bytes = std::fs::read(path)
        .map_err(|e| CliError::Io(format!("Failed to read file '{path_str}': {e}")))?;

    Ok((file_name, mime.to_string(), bytes))
}

/// Tek bir dosyayı sunucuya yükler ve UploadResponse döner.
pub async fn upload_file(
    client: &ApiClient,
    file_path: &str,
) -> Result<actos_sdk::actos_types::upload::UploadResponse, CliError> {
    if client.api_key().is_none() {
        return Err(CliError::Auth(
            "Authentication required to upload files. Run 'actos auth login' or set ACTOS_API_KEY."
                .to_string(),
        ));
    }

    let (file_name, mime, _bytes) = validate_and_read_file(file_path)?;

    let res = client
        .uploads()
        .create(file_path)
        .filename(file_name)
        .mime_type(mime)
        .send()
        .await
        .map_err(CliError::from)?;

    Ok(res)
}

pub async fn handle_upload(
    action: UploadAction,
    client: &ApiClient,
    output: &OutputContext,
    yes: bool,
) -> Result<(), CliError> {
    match action {
        UploadAction::Create { file } => {
            let res = upload_file(client, &file).await?;

            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                let val = serde_json::to_value(&res)
                    .map_err(|e| CliError::General(format!("Failed to serialize upload: {e}")))?;
                output.print_json(&val, Some(rl));
            } else {
                println!("Upload successful: {}", res.id);
                println!("URL:           {}", res.url);
                println!("Thumbnail:     {}", res.thumbnail_url);
                println!("Size:          {} bytes ({})", res.byte_size, res.mime_type);
            }
        }

        UploadAction::Delete { id } => {
            if client.api_key().is_none() {
                return Err(CliError::Auth(
                    "Authentication required to delete uploads.".to_string(),
                ));
            }

            // Ajan Sözleşmesi §2 kural 5
            if !yes {
                return Err(CliError::Usage(
                    "Deleting an upload is permanent. Pass '--yes' to confirm.".to_string(),
                ));
            }

            client.uploads().delete(&id).await.map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                let res = json!({ "status": "deleted", "id": id });
                output.print_json(&res, Some(rl));
            } else {
                println!("Upload '{id}' deleted.");
            }
        }
    }

    Ok(())
}
