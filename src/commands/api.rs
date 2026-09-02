use reqwest::Method;
use serde_json::{Value, json};
use std::io::Read;
use std::path::Path;

use crate::cli::ApiArgs;
use crate::client::ApiClient;
use crate::error::CliError;
use crate::output::OutputContext;

pub async fn handle_api(
    args: ApiArgs,
    client: &ApiClient,
    output: &OutputContext,
) -> Result<(), CliError> {
    let method = args
        .method
        .to_uppercase()
        .parse::<Method>()
        .map_err(|_| CliError::Usage(format!("Invalid HTTP method '{}'", args.method)))?;

    let path = if args.path.starts_with("http://")
        || args.path.starts_with("https://")
        || args.path.starts_with('/')
    {
        args.path.clone()
    } else {
        format!("/{}", args.path)
    };

    // Parse fields
    let mut query_params: Vec<(String, String)> = Vec::new();
    let mut json_fields = serde_json::Map::new();

    for field in &args.field {
        let (k, v) = match field.split_once('=') {
            Some((k, v)) => (k.trim().to_string(), v.trim().to_string()),
            None => (field.trim().to_string(), String::new()),
        };

        if method == Method::GET || method == Method::HEAD {
            query_params.push((k, v));
        } else {
            let val = match serde_json::from_str::<Value>(&v) {
                Ok(parsed) => parsed,
                Err(_) => Value::String(v),
            };
            json_fields.insert(k, val);
        }
    }

    // Resolve body
    let body_bytes: Option<Vec<u8>> = if let Some(ref input) = args.input {
        if input == "-" {
            let mut buffer = Vec::new();
            std::io::stdin()
                .read_to_end(&mut buffer)
                .map_err(|e| CliError::Io(format!("Failed to read body from stdin: {e}")))?;
            Some(buffer)
        } else {
            let file_path = if let Some(stripped) = input.strip_prefix('@') {
                stripped
            } else {
                input.as_str()
            };

            if Path::new(file_path).is_file() {
                let bytes = std::fs::read(file_path)
                    .map_err(|e| CliError::Io(format!("Failed to read file '{file_path}': {e}")))?;
                Some(bytes)
            } else {
                Some(input.as_bytes().to_vec())
            }
        }
    } else if !json_fields.is_empty() {
        let bytes = serde_json::to_vec(&Value::Object(json_fields))
            .map_err(|e| CliError::Validation(format!("Failed to serialize body fields: {e}")))?;
        Some(bytes)
    } else {
        None
    };

    let query_refs: Vec<(&str, &str)> = query_params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let query_opt = if query_refs.is_empty() {
        None
    } else {
        Some(query_refs.as_slice())
    };

    let (_status, _headers, bytes, rate_limit) = client
        .execute_request(method, &path, query_opt, body_bytes, None)
        .await?;

    if args.raw {
        if let Ok(s) = std::str::from_utf8(&bytes) {
            print!("{s}");
        } else {
            let _ = std::io::Write::write_all(&mut std::io::stdout(), &bytes);
        }
        return Ok(());
    }

    if let Ok(val) = serde_json::from_slice::<Value>(&bytes) {
        output.print_json(&val, Some(rate_limit));
    } else if let Ok(s) = std::str::from_utf8(&bytes) {
        if output.json {
            let res = json!({ "content": s });
            output.print_json(&res, Some(rate_limit));
        } else {
            println!("{s}");
        }
    }

    Ok(())
}
