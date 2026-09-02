use comfy_table::{Table, presets::UTF8_FULL};

use crate::cli::FeedArgs;
use crate::client::ApiClient;
use crate::error::CliError;
use crate::output::OutputContext;

pub async fn handle_feed(
    args: FeedArgs,
    client: &ApiClient,
    output: &OutputContext,
    limit: u32,
    cursor: Option<&str>,
) -> Result<(), CliError> {
    if args.following && client.api_key().is_none() {
        return Err(CliError::Auth(
            "Authentication required to view following feed. Run 'actos auth login' or set ACTOS_API_KEY.".to_string(),
        ));
    }

    let path = if args.following {
        "/feed/following"
    } else {
        "/feed"
    };

    let mut query_params: Vec<(&str, &str)> = Vec::new();
    if let Some(ref s) = args.sort {
        query_params.push(("sort", s.as_str()));
    }
    if let Some(ref w) = args.window {
        query_params.push(("window", w.as_str()));
    }

    let (val, rate_limit) = client.paginate(path, &query_params, limit, cursor).await?;

    if output.json {
        output.print_json(&val, Some(rate_limit));
    } else {
        let posts_opt = val.get("posts").and_then(|v| v.as_array());

        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(vec![
            "ID",
            "Title",
            "Author",
            "Score",
            "Comments",
            "Created At",
        ]);

        if let Some(posts) = posts_opt {
            for p in posts {
                let id = p["id"].as_str().unwrap_or("-");
                let title = p["title"].as_str().unwrap_or("(no title)");
                let author = p["author"]["username"].as_str().unwrap_or("-");
                let score = p["score"].to_string();
                let comments = p["comment_count"].to_string();
                let created_at = p["created_at"].as_str().unwrap_or("-");

                table.add_row(vec![id, title, author, &score, &comments, created_at]);
            }
        }

        println!("{table}");
    }

    Ok(())
}
