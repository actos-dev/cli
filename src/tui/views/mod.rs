pub mod actors;
pub mod detail;
pub mod feed;
pub mod help;
pub mod inbox;
pub mod profile;
pub mod saves;
pub mod search;
pub mod tags;

use actos_sdk::actos_types::content::ContentSummary;

/// Liste satırı başlığı: postta `title`, yorumda gövde özeti
/// (yorumun başlığı olmaz — web-lite kuralı).
#[must_use]
pub fn row_title(item: &ContentSummary) -> String {
    if item.content_type == "comment" {
        let flat = item
            .body
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let cut: String = flat.chars().take(40).collect();
        if flat.chars().count() > 40 {
            format!("{cut}…")
        } else {
            cut
        }
    } else {
        item.title.as_deref().unwrap_or("(no title)").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn post(title: Option<&str>, body: &str, kind: &str) -> ContentSummary {
        serde_json::from_value(json!({
            "id": "c_x",
            "content_type": kind,
            "author": {
                "id": "a_1", "username": "u", "actor_type": "human",
                "display_name": null, "bio": null,
                "created_at": "2026-01-01T00:00:00Z",
                "trust_level": 0, "avatar_url": null
            },
            "author_deleted": false,
            "title": title,
            "body": body,
            "body_format": "markdown",
            "body_html": null,
            "metadata": {},
            "tags": [],
            "score": 0, "upvotes": 0, "downvotes": 0,
            "comment_count": 0,
            "created_at": "2026-01-01T00:00:00Z",
            "edited_at": null,
            "attachments": null,
            "deleted": false
        }))
        .expect("fixture parses")
    }

    #[test]
    fn test_row_title_post_and_comment() {
        assert_eq!(row_title(&post(Some("Hi"), "b", "post")), "Hi");
        assert_eq!(row_title(&post(None, "b", "post")), "(no title)");
        assert_eq!(row_title(&post(None, "hello world", "comment")), "hello world");
        let long = post(None, &"x".repeat(50), "comment");
        assert!(row_title(&long).ends_with('…'));
    }
}
