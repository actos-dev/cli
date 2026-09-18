use std::path::Path;

use crate::error::CliError;

/// Maximum size accepted for an image that travels with a post, a comment or
/// the avatar. The server enforces its own limit too; this is only a friendly
/// client-side check that avoids sending an obviously oversized body.
pub const MAX_UPLOAD_BYTES: u64 = 8 * 1024 * 1024; // 8 MB

/// Validates an image path and wraps it as the SDK's [`actos_sdk::FileUpload`].
///
/// There is no standalone upload endpoint any more (REFACTOR.md §4): an image
/// is only ever created as a side effect of the content or avatar request that
/// carries it. The returned value is fed straight into the SDK builder
/// (`.attach(..)`/`.files(..)` or `upload_avatar`), which sends one multipart
/// request.
pub fn load_image(path_str: &str) -> Result<actos_sdk::FileUpload, CliError> {
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
        .map(str::to_lowercase)
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

    let upload = actos_sdk::FileUpload::from_path(path)
        .map_err(|e| CliError::Io(format!("Failed to read file '{path_str}': {e}")))?;

    Ok(upload.content_type(mime))
}
