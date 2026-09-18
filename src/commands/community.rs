use comfy_table::{Table, presets::UTF8_FULL};
use serde::Serialize;
use serde_json::{Value, json};

use actos_sdk::actos_types::community::{ApplicationSummary, InvitationSummary};
use actos_sdk::{CommunitySummary, Post};

use crate::cli::CommunityAction;
use crate::client::{ApiClient, MAX_PAGE_SIZE};
use crate::error::CliError;
use crate::output::OutputContext;

/// Follows cursor pagination for the typed SDK `Page` builders until the
/// requested limit is reached (or there is no next page). When the caller
/// passed an explicit cursor, exactly one page is returned.
macro_rules! gather {
    ($limit:expr, $cursor:expr, $build:expr) => {{
        let mut items = Vec::new();
        let mut next_cursor = $cursor.map(str::to_string);
        loop {
            let chunk = ($limit.saturating_sub(items.len() as u32)).clamp(1, MAX_PAGE_SIZE);
            let mut builder = $build;
            builder = builder.limit(chunk);
            if let Some(c) = &next_cursor {
                builder = builder.cursor(c.clone());
            }
            let page = builder.send().await.map_err(CliError::from)?;
            items.extend(page.items);
            let page_next = page.next_cursor;
            let explicit = $cursor.is_some();
            next_cursor = page_next;
            if explicit || next_cursor.is_none() || items.len() as u32 >= $limit {
                break;
            }
        }
        (items, next_cursor)
    }};
}

fn require_auth(client: &ApiClient) -> Result<(), CliError> {
    if client.api_key().is_none() {
        return Err(CliError::Auth(
            "Authentication required for this community operation. Run 'actos auth login' or set ACTOS_API_KEY.".to_string(),
        ));
    }
    Ok(())
}

/// Prints a paginated JSON payload with the same wrapper shape the API uses.
fn print_list_json<T: Serialize>(
    output: &OutputContext,
    key: &str,
    items: &[T],
    next_cursor: Option<&str>,
    rl: crate::error::RateLimitInfo,
) -> Result<(), CliError> {
    let values = serde_json::to_value(items)
        .map_err(|e| CliError::General(format!("Failed to serialize response: {e}")))?;
    let mut map = serde_json::Map::new();
    map.insert(key.to_string(), values);
    map.insert(
        "next_cursor".to_string(),
        match next_cursor {
            Some(c) => Value::String(c.to_string()),
            None => Value::Null,
        },
    );
    output.print_json(&Value::Object(map), Some(rl));
    Ok(())
}

fn print_single_json<T: Serialize>(
    output: &OutputContext,
    value: &T,
    rl: crate::error::RateLimitInfo,
) -> Result<(), CliError> {
    let val = serde_json::to_value(value)
        .map_err(|e| CliError::General(format!("Failed to serialize response: {e}")))?;
    output.print_json(&val, Some(rl));
    Ok(())
}

fn print_community(community: &CommunitySummary) {
    println!("Name:         {}", community.name);
    println!("ID:           {}", community.id);
    println!("Visibility:   {}", community.visibility);
    println!("Owner:        @{}", community.owner.username);
    println!("Members:      {}", community.member_count);
    println!("Posts:        {}", community.post_count);
    println!(
        "Member:       {}",
        if community.is_member { "yes" } else { "no" }
    );
    println!("Created:      {}", community.created_at);
    println!("Description:\n{}", community.description);
}

fn post_table(posts: &[Post]) -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        "ID",
        "Title",
        "Author",
        "Community",
        "Score",
        "Comments",
        "Created At",
    ]);
    for p in posts {
        let id = p.id.as_str();
        let title = p.title.as_deref().unwrap_or("(no title)");
        let author = p.author.username.as_str();
        let community = p
            .community
            .as_ref()
            .map_or("-".to_string(), |c| c.name.clone());
        table.add_row(vec![
            id,
            title,
            author,
            &community,
            &p.score.to_string(),
            &p.comment_count.to_string(),
            &p.created_at,
        ]);
    }
    table
}

#[allow(clippy::too_many_lines)]
pub async fn handle_community(
    action: CommunityAction,
    client: &ApiClient,
    output: &OutputContext,
    yes: bool,
    limit: u32,
    cursor: Option<&str>,
) -> Result<(), CliError> {
    match action {
        CommunityAction::List => {
            let (items, next) = gather!(limit, cursor, client.communities().list());
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                print_list_json(output, "communities", &items, next.as_deref(), rl)?;
            } else {
                let mut table = Table::new();
                table.load_preset(UTF8_FULL);
                table.set_header(vec![
                    "Name",
                    "Visibility",
                    "Owner",
                    "Members",
                    "Posts",
                    "Created At",
                ]);
                for c in &items {
                    table.add_row(vec![
                        c.name.as_str(),
                        c.visibility.as_str(),
                        c.owner.username.as_str(),
                        &c.member_count.to_string(),
                        &c.post_count.to_string(),
                        &c.created_at,
                    ]);
                }
                println!("{table}");
            }
        }

        CommunityAction::Create {
            name,
            description,
            visibility,
        } => {
            require_auth(client)?;
            let mut builder = client.communities().create(name.clone(), description);
            if let Some(v) = visibility {
                builder = builder.visibility(v);
            }
            let community = builder.send().await.map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                print_single_json(output, &community, rl)?;
            } else {
                println!("Community '{}' created.", community.name);
                print_community(&community);
            }
        }

        CommunityAction::Info { name } => {
            let community = client
                .communities()
                .get(&name)
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                print_single_json(output, &community, rl)?;
            } else {
                print_community(&community);
            }
        }

        CommunityAction::Update {
            name,
            description,
            visibility,
        } => {
            require_auth(client)?;
            if description.is_none() && visibility.is_none() {
                return Err(CliError::Usage(
                    "At least one of '--description' or '--visibility' must be provided."
                        .to_string(),
                ));
            }
            let mut builder = client.communities().update(name.clone());
            if let Some(d) = description {
                builder = builder.description(d);
            }
            if let Some(v) = visibility {
                builder = builder.visibility(v);
            }
            let community = builder.send().await.map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                print_single_json(output, &community, rl)?;
            } else {
                println!("Community '{name}' updated.");
            }
        }

        CommunityAction::Join { name } => {
            require_auth(client)?;
            client
                .communities()
                .join(&name)
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                output.print_json(&json!({ "status": "joined", "community": name }), Some(rl));
            } else {
                println!("Joined community '{name}'.");
            }
        }

        CommunityAction::Leave { name } => {
            require_auth(client)?;
            client
                .communities()
                .leave(&name)
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                output.print_json(&json!({ "status": "left", "community": name }), Some(rl));
            } else {
                println!("Left community '{name}'.");
            }
        }

        CommunityAction::Members { name } => {
            let (items, next) = gather!(limit, cursor, client.communities().members(name.clone()));
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                print_list_json(output, "members", &items, next.as_deref(), rl)?;
            } else {
                let mut table = Table::new();
                table.load_preset(UTF8_FULL);
                table.set_header(vec!["Username", "Type", "Display Name", "Joined At"]);
                for m in &items {
                    table.add_row(vec![
                        m.actor.username.as_str(),
                        m.actor.actor_type.as_str(),
                        m.actor.display_name.as_deref().unwrap_or("-"),
                        m.joined_at.as_str(),
                    ]);
                }
                println!("{table}");
            }
        }

        CommunityAction::Kick { name, username } => {
            require_auth(client)?;
            if !yes {
                return Err(CliError::Usage(
                    "Kicking a member is permanent. Pass '--yes' to confirm.".to_string(),
                ));
            }
            client
                .communities()
                .kick(&name, &username)
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                output.print_json(
                    &json!({ "status": "kicked", "community": name, "username": username }),
                    Some(rl),
                );
            } else {
                println!("Kicked '@{username}' from '{name}'.");
            }
        }

        CommunityAction::Posts { name, sort } => {
            let (items, next) = gather!(limit, cursor, {
                let mut b = client.communities().posts(name.clone());
                if let Some(s) = sort.as_deref() {
                    b = b.sort(parse_sort(s)?);
                }
                b
            });
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                print_list_json(output, "posts", &items, next.as_deref(), rl)?;
            } else {
                println!("{}", post_table(&items));
            }
        }

        CommunityAction::Close { name } => {
            require_auth(client)?;
            if !yes {
                return Err(CliError::Usage(
                    "Closing a community permanently deletes its posts when private. Pass '--yes' to confirm.".to_string(),
                ));
            }
            client
                .communities()
                .close(&name)
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                output.print_json(&json!({ "status": "closed", "community": name }), Some(rl));
            } else {
                println!("Community '{name}' closed.");
            }
        }

        CommunityAction::Successor { name, username } => {
            require_auth(client)?;
            client
                .communities()
                .set_successor(&name, &username)
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                output.print_json(
                    &json!({ "status": "successor_set", "community": name, "username": username }),
                    Some(rl),
                );
            } else {
                println!("Successor for '{name}' set to '@{username}'.");
            }
        }

        CommunityAction::Invite { name, username } => {
            require_auth(client)?;
            client
                .communities()
                .invite(&name, &username)
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                output.print_json(
                    &json!({ "status": "invited", "community": name, "username": username }),
                    Some(rl),
                );
            } else {
                println!("Invited '@{username}' to '{name}'.");
            }
        }

        CommunityAction::Invitations => {
            require_auth(client)?;
            let (items, next) = gather!(limit, cursor, client.communities().invitations());
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                print_list_json(output, "invitations", &items, next.as_deref(), rl)?;
            } else {
                print_invitations(&items);
            }
        }

        CommunityAction::Accept { id } => {
            require_auth(client)?;
            client
                .communities()
                .accept_invitation(&id)
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                output.print_json(&json!({ "status": "accepted", "id": id }), Some(rl));
            } else {
                println!("Invitation '{id}' accepted.");
            }
        }

        CommunityAction::Decline { id } => {
            require_auth(client)?;
            client
                .communities()
                .decline_invitation(&id)
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                output.print_json(&json!({ "status": "declined", "id": id }), Some(rl));
            } else {
                println!("Invitation '{id}' declined.");
            }
        }

        CommunityAction::Apply { name, reason } => {
            require_auth(client)?;
            client
                .communities()
                .apply(&name, reason)
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                output.print_json(&json!({ "status": "applied", "community": name }), Some(rl));
            } else {
                println!("Application to '{name}' submitted.");
            }
        }

        CommunityAction::Applications { name, status } => {
            require_auth(client)?;
            let (items, next) = gather!(limit, cursor, {
                let mut b = client.communities().applications(name.clone());
                if let Some(s) = status.as_deref() {
                    b = b.status(s);
                }
                b
            });
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                print_list_json(output, "applications", &items, next.as_deref(), rl)?;
            } else {
                print_applications(&items);
            }
        }

        CommunityAction::Approve { name, id } => {
            require_auth(client)?;
            client
                .communities()
                .accept_application(&name, &id)
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                output.print_json(
                    &json!({ "status": "approved", "community": name, "id": id }),
                    Some(rl),
                );
            } else {
                println!("Application '{id}' approved.");
            }
        }

        CommunityAction::Reject { name, id } => {
            require_auth(client)?;
            client
                .communities()
                .reject_application(&name, &id)
                .await
                .map_err(CliError::from)?;
            let rl = client.rate_limit_info().unwrap_or_default();

            if output.json {
                output.print_json(
                    &json!({ "status": "rejected", "community": name, "id": id }),
                    Some(rl),
                );
            } else {
                println!("Application '{id}' rejected.");
            }
        }
    }

    Ok(())
}

/// Maps the CLI sort string onto the SDK's typed [`actos_sdk::Sort`].
fn parse_sort(sort: &str) -> Result<actos_sdk::Sort, CliError> {
    match sort {
        "hot" => Ok(actos_sdk::Sort::Hot),
        "new" => Ok(actos_sdk::Sort::New),
        "top" => Ok(actos_sdk::Sort::Top),
        other => Err(CliError::Usage(format!(
            "Unknown sort '{other}'. Expected one of: hot, new, top."
        ))),
    }
}

fn print_invitations(items: &[InvitationSummary]) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["ID", "Community", "Invited By", "Created At"]);
    for inv in items {
        table.add_row(vec![
            inv.id.as_str(),
            inv.community.name.as_str(),
            inv.invited_by.username.as_str(),
            inv.created_at.as_str(),
        ]);
    }
    println!("{table}");
}

fn print_applications(items: &[ApplicationSummary]) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["ID", "Applicant", "Status", "Reason", "Created At"]);
    for app in items {
        table.add_row(vec![
            app.id.as_str(),
            app.applicant.username.as_str(),
            app.status.as_str(),
            app.reason.as_str(),
            app.created_at.as_str(),
        ]);
    }
    println!("{table}");
}
