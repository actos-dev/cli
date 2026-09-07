//! TUI render smoke testleri (F7): her ekran ve overlay, 100x32'lik
//! sanal terminalde paniklemeden çizilir ve ayırt edici metni içerir.
#![cfg(feature = "tui")]

use actos::tui::app::{App, CurrentTab, Overlay, OverlayField, PagedList};
use actos::tui::ui::render_ui;
use ratatui::{Terminal, backend::TestBackend};

fn blank_app() -> App {
    App {
        current_tab: CurrentTab::Feed,
        back_stack: Vec::new(),
        feed: PagedList::default(),
        selected_post: None,
        post_comments: Vec::new(),
        comment_order: Vec::new(),
        comment_selected: 0,
        detail_scroll: 0,
        detail_lines: 0,
        overlay: None,
        search_query: String::new(),
        search_type: "post".to_string(),
        search: PagedList::default(),
        search_actors: PagedList::default(),
        feed_sort: "new".to_string(),
        feed_window: "all".to_string(),
        feed_actor_type: None,
        feed_following: false,
        tags: PagedList::default(),
        tag_posts_name: String::new(),
        tag_posts_sort: "new".to_string(),
        tag_posts: PagedList::default(),
        actors: PagedList::default(),
        actors_type: None,
        inbox: PagedList::default(),
        inbox_unread_only: false,
        inbox_unread_total: 0,
        saves: PagedList::default(),
        profile_info: None,
        profile_username: None,
        profile_posts_tab: true,
        profile_posts: PagedList::default(),
        status_message: String::new(),
        show_help_popup: false,
        should_quit: false,
        hit_areas: Vec::new(),
    }
}

fn actor_json() -> serde_json::Value {
    serde_json::json!({
        "id": "a_1", "username": "alice", "actor_type": "human",
        "display_name": "Alice", "bio": "hi",
        "created_at": "2026-01-01T00:00:00Z",
        "trust_level": 3, "avatar_url": null
    })
}

fn post_json(id: &str, title: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id, "content_type": "post",
        "author": actor_json(),
        "author_deleted": false, "title": title, "body": "body text",
        "body_format": "markdown", "body_html": null, "metadata": {},
        "tags": ["rust"], "score": 4, "upvotes": 4, "downvotes": 0,
        "comment_count": 2, "created_at": "2026-01-01T00:00:00Z",
        "edited_at": null, "attachments": null, "deleted": false
    })
}

fn screen_text(app: &mut App) -> String {
    #![allow(clippy::expect_used)]
    let backend = TestBackend::new(100, 32);
    let mut term = Terminal::new(backend).expect("terminal builds");
    term.draw(|f| render_ui(f, app)).expect("frame draws");
    term.backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
}

#[test]
fn test_render_feed_with_post() {
    let mut app = blank_app();
    app.feed.items = vec![serde_json::from_value(post_json("c_1", "Hello TUI")).unwrap()];
    let text = screen_text(&mut app);
    assert!(text.contains("Hello TUI"), "feed shows title");
    assert!(text.contains("@alice"), "feed shows author");
}

#[test]
fn test_render_detail_with_post() {
    let mut app = blank_app();
    app.current_tab = CurrentTab::Detail;
    app.selected_post = Some(serde_json::from_value(post_json("c_1", "Hello TUI")).unwrap());
    let text = screen_text(&mut app);
    assert!(text.contains("Hello TUI"));
    assert!(text.contains("Comments"));
    assert!(text.contains("body text"));
}

#[test]
fn test_render_tags_actors_inbox_saves() {
    let mut app = blank_app();

    app.current_tab = CurrentTab::Tags;
    app.tags.items = vec![
        serde_json::from_value(serde_json::json!({
            "name": "rust", "post_count": 12,
            "created_at": "2026-01-01T00:00:00Z"
        }))
        .unwrap(),
    ];
    assert!(screen_text(&mut app).contains("#rust"));

    app.current_tab = CurrentTab::Actors;
    app.actors.items = vec![serde_json::from_value(actor_json()).unwrap()];
    assert!(screen_text(&mut app).contains("@alice"));

    app.current_tab = CurrentTab::Inbox;
    app.inbox.items = vec![
        serde_json::from_value(serde_json::json!({
            "id": "n_1", "kind": "comment_on_post", "actor": actor_json(),
            "target_type": "content", "target_id": "c_9", "payload": {},
            "created_at": "2026-01-01T00:00:00Z", "read_at": null
        }))
        .unwrap(),
    ];
    let inbox_text = screen_text(&mut app);
    assert!(inbox_text.contains("comment_on_post"));

    app.current_tab = CurrentTab::Saves;
    app.saves.items = vec![serde_json::from_value(post_json("c_2", "Saved")).unwrap()];
    assert!(screen_text(&mut app).contains("Saved"));

    app.current_tab = CurrentTab::TagPosts;
    app.tag_posts_name = "rust".to_string();
    app.tag_posts.items = vec![serde_json::from_value(post_json("c_3", "Tagged")).unwrap()];
    assert!(screen_text(&mut app).contains("#rust"));
}

#[test]
fn test_render_profile_search_help() {
    let mut app = blank_app();

    app.current_tab = CurrentTab::Profile;
    app.profile_info = Some(
        serde_json::from_value(serde_json::json!({
            "actor": actor_json(),
            "stats": {"post_count": 5, "comment_count": 9, "total_score": 42}
        }))
        .unwrap(),
    );
    app.profile_username = Some("alice".to_string());
    let profile_text = screen_text(&mut app);
    assert!(profile_text.contains("@alice"));
    assert!(profile_text.contains("42"));

    app.current_tab = CurrentTab::Search;
    app.search_query = "rust".to_string();
    let search_text = screen_text(&mut app);
    assert!(search_text.contains("rust"));

    app.current_tab = CurrentTab::Help;
    assert!(screen_text(&mut app).contains("Keyboard Shortcuts"));
}

#[test]
fn test_render_overlays() {
    let mut app = blank_app();
    app.current_tab = CurrentTab::Detail;
    app.selected_post = Some(serde_json::from_value(post_json("c_1", "Hi")).unwrap());

    app.overlay = Some(Overlay::Reply {
        post_id: "c_1".to_string(),
        text: String::new(),
    });
    assert!(screen_text(&mut app).contains("Reply"));

    app.overlay = Some(Overlay::Composer {
        title: String::new(),
        body: String::new(),
        tags: String::new(),
        focus: OverlayField::First,
    });
    let composer = screen_text(&mut app);
    assert!(composer.contains("Title"));
    assert!(composer.contains("Ctrl+S"));

    app.overlay = Some(Overlay::ConfirmDeletePost {
        post_id: "c_1".to_string(),
    });
    assert!(screen_text(&mut app).contains("permanent"));

    app.overlay = None;
    app.show_help_popup = true;
    assert!(screen_text(&mut app).contains("Quick Shortcuts"));
}
