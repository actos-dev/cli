//! `actos community *` — the 0.3.0 communities surface.
//!
//! Every request is asserted against the real API paths the SDK builds:
//! `/communities`, `/communities/{name}`, membership, moderation and the
//! `/me/invitations` queue.
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use wiremock::matchers::{body_partial_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn actor(username: &str) -> serde_json::Value {
    serde_json::json!({
        "id": format!("a_{username}"),
        "username": username,
        "actor_type": "human",
        "display_name": null,
        "bio": null,
        "created_at": "2026-01-01T00:00:00Z",
        "avatar_url": null
    })
}

fn community(name: &str) -> serde_json::Value {
    serde_json::json!({
        "id": format!("m_{name}"),
        "name": name,
        "description": "Rust talk",
        "visibility": "public",
        "owner": actor("alice"),
        "member_count": 3,
        "post_count": 2,
        "is_member": false,
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z"
    })
}

fn post(id: &str, title: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "content_type": "post",
        "author": actor("alice"),
        "author_deleted": false,
        "community": {"id": "m_rust", "name": "rust"},
        "title": title,
        "body": "body",
        "body_format": "markdown",
        "body_html": null,
        "tags": [],
        "score": 1,
        "upvotes": 1,
        "downvotes": 0,
        "comment_count": 0,
        "created_at": "2026-01-01T00:00:00Z",
        "edited_at": null,
        "attachments": null,
        "deleted": false,
        "is_cross_post": false,
        "cross_post": null
    })
}

#[tokio::test]
async fn test_community_list_and_cursor_pagination() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/communities"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "communities": [community("rust")],
            "next_cursor": "cur_2"
        })))
        .mount(&mock_server)
        .await;

    // One page only when an explicit cursor is supplied.
    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["--cursor", "cur_1", "community", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("rust"))
        .stdout(predicate::str::contains("public"));
}

#[tokio::test]
async fn test_community_list_json_shape() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/communities"))
        .and(query_param("cursor", "cur_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "communities": [community("rust")],
            "next_cursor": null
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    let assert = cmd
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["--json", "--cursor", "cur_1", "community", "list"])
        .assert()
        .success();

    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let val: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(val["communities"][0]["name"], "rust");
    assert!(val["next_cursor"].is_null());
}

#[tokio::test]
async fn test_community_create_sends_name_description_visibility() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("POST"))
        .and(path("/communities"))
        .and(body_partial_json(serde_json::json!({
            "name": "rust",
            "description": "Rust talk",
            "visibility": "private"
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "m_rust",
            "name": "rust",
            "description": "Rust talk",
            "visibility": "private",
            "owner": actor("alice"),
            "member_count": 1,
            "post_count": 0,
            "is_member": true,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args([
            "community",
            "create",
            "rust",
            "--description",
            "Rust talk",
            "--visibility",
            "private",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Community 'rust' created."));
}

#[tokio::test]
async fn test_community_info_get_alias() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/communities/rust"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "m_rust",
            "name": "rust",
            "description": "Rust talk",
            "visibility": "public",
            "owner": actor("alice"),
            "member_count": 3,
            "post_count": 2,
            "is_member": true,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["community", "get", "rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Name:         rust"))
        .stdout(predicate::str::contains("Owner:        @alice"));
}

#[tokio::test]
async fn test_community_update() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("PATCH"))
        .and(path("/communities/rust"))
        .and(body_partial_json(serde_json::json!({
            "description": "New description"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "m_rust",
            "name": "rust",
            "description": "New description",
            "visibility": "public",
            "owner": actor("alice"),
            "member_count": 3,
            "post_count": 2,
            "is_member": true,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args([
            "community",
            "update",
            "rust",
            "--description",
            "New description",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Community 'rust' updated."));
}

#[tokio::test]
async fn test_community_join_and_leave() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("POST"))
        .and(path("/communities/rust/join"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/communities/rust/join"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let mut cmd_join = Command::cargo_bin("actos").unwrap();
    cmd_join
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["community", "join", "rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Joined community 'rust'."));

    let mut cmd_leave = Command::cargo_bin("actos").unwrap();
    cmd_leave
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["community", "leave", "rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Left community 'rust'."));
}

#[tokio::test]
async fn test_community_members() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/communities/rust/members"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "members": [
                {"actor": actor("alice"), "joined_at": "2026-01-01T00:00:00Z"}
            ],
            "next_cursor": null
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["community", "members", "rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alice"));
}

#[tokio::test]
async fn test_community_posts_with_sort() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/communities/rust/posts"))
        .and(query_param("sort", "top"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "posts": [post("c_1", "Hello rust")],
            "next_cursor": null
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["community", "posts", "rust", "--sort", "top"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello rust"))
        .stdout(predicate::str::contains("rust"));
}

#[tokio::test]
async fn test_community_kick_requires_yes() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("DELETE"))
        .and(path("/communities/rust/members/bob"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let mut cmd_no = Command::cargo_bin("actos").unwrap();
    cmd_no
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["community", "kick", "rust", "bob"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Pass '--yes' to confirm"));

    let mut cmd_yes = Command::cargo_bin("actos").unwrap();
    cmd_yes
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["community", "kick", "rust", "bob", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Kicked '@bob' from 'rust'."));
}

#[tokio::test]
async fn test_community_close_requires_yes() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("POST"))
        .and(path("/communities/rust/close"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let mut cmd_no = Command::cargo_bin("actos").unwrap();
    cmd_no
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["community", "close", "rust"])
        .assert()
        .code(2);

    let mut cmd_yes = Command::cargo_bin("actos").unwrap();
    cmd_yes
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["community", "close", "rust", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Community 'rust' closed."));
}

#[tokio::test]
async fn test_community_successor_and_invite() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("PUT"))
        .and(path("/communities/rust/successor"))
        .and(body_partial_json(serde_json::json!({"username": "bob"})))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(path("/communities/rust/invitations"))
        .and(body_partial_json(serde_json::json!({"username": "carol"})))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let mut cmd_succ = Command::cargo_bin("actos").unwrap();
    cmd_succ
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["community", "successor", "rust", "bob"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Successor for 'rust' set to '@bob'.",
        ));

    let mut cmd_invite = Command::cargo_bin("actos").unwrap();
    cmd_invite
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["community", "invite", "rust", "carol"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Invited '@carol' to 'rust'."));
}

#[tokio::test]
async fn test_community_invitations_accept_decline() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/me/invitations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "invitations": [
                {
                    "id": "i_1",
                    "community": {"id": "m_rust", "name": "rust"},
                    "invited_by": actor("alice"),
                    "created_at": "2026-01-01T00:00:00Z"
                }
            ],
            "next_cursor": null
        })))
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(path("/me/invitations/i_1/accept"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(path("/me/invitations/i_2/decline"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let mut cmd_list = Command::cargo_bin("actos").unwrap();
    cmd_list
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["community", "invitations"])
        .assert()
        .success()
        .stdout(predicate::str::contains("i_1"))
        .stdout(predicate::str::contains("rust"));

    let mut cmd_accept = Command::cargo_bin("actos").unwrap();
    cmd_accept
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["community", "accept", "i_1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Invitation 'i_1' accepted."));

    let mut cmd_decline = Command::cargo_bin("actos").unwrap();
    cmd_decline
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["community", "decline", "i_2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Invitation 'i_2' declined."));
}

#[tokio::test]
async fn test_community_applications_flow() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("POST"))
        .and(path("/communities/rust/applications"))
        .and(body_partial_json(
            serde_json::json!({"reason": "I like Rust"}),
        ))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/communities/rust/applications"))
        .and(query_param("status", "pending"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "applications": [
                {
                    "id": "p_1",
                    "community": {"id": "m_rust", "name": "rust"},
                    "applicant": actor("bob"),
                    "reason": "I like Rust",
                    "status": "pending",
                    "created_at": "2026-01-01T00:00:00Z",
                    "resolved_at": null
                }
            ],
            "next_cursor": null
        })))
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(path("/communities/rust/applications/p_1/accept"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(path("/communities/rust/applications/p_2/reject"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let mut cmd_apply = Command::cargo_bin("actos").unwrap();
    cmd_apply
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["community", "apply", "rust", "--reason", "I like Rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Application to 'rust' submitted."));

    let mut cmd_list = Command::cargo_bin("actos").unwrap();
    cmd_list
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["community", "applications", "rust", "--status", "pending"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bob"))
        .stdout(predicate::str::contains("pending"));

    let mut cmd_approve = Command::cargo_bin("actos").unwrap();
    cmd_approve
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["community", "approve", "rust", "p_1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Application 'p_1' approved."));

    let mut cmd_reject = Command::cargo_bin("actos").unwrap();
    cmd_reject
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["community", "reject", "rust", "p_2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Application 'p_2' rejected."));
}

#[tokio::test]
async fn test_post_create_with_community_and_cross_post() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("POST"))
        .and(path("/posts"))
        .and(body_partial_json(serde_json::json!({
            "title": "t",
            "body": "b",
            "community": "rust"
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "c_comm",
            "content_type": "post",
            "author": actor("alice"),
            "author_deleted": false,
            "community": {"id": "m_rust", "name": "rust"},
            "title": "t",
            "body": "b",
            "body_format": "markdown",
            "body_html": null,
            "tags": [],
            "score": 0,
            "upvotes": 0,
            "downvotes": 0,
            "comment_count": 0,
            "created_at": "2026-01-01T00:00:00Z",
            "edited_at": null,
            "attachments": null,
            "deleted": false,
            "is_cross_post": false,
            "cross_post": null
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/posts"))
        .and(body_partial_json(serde_json::json!({
            "cross_post_source": "c_source"
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "c_cross",
            "content_type": "post",
            "author": actor("alice"),
            "author_deleted": false,
            "community": null,
            "title": null,
            "body": "",
            "body_format": "markdown",
            "body_html": null,
            "tags": [],
            "score": 0,
            "upvotes": 0,
            "downvotes": 0,
            "comment_count": 0,
            "created_at": "2026-01-01T00:00:00Z",
            "edited_at": null,
            "attachments": null,
            "deleted": false,
            "is_cross_post": true,
            "cross_post": {
                "id": "c_source",
                "title": "Source",
                "author": actor("bob"),
                "community": {"id": "m_rust", "name": "rust"}
            }
        })))
        .mount(&mock_server)
        .await;

    let mut cmd_community = Command::cargo_bin("actos").unwrap();
    cmd_community
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args([
            "post",
            "create",
            "--title",
            "t",
            "--body",
            "b",
            "--community",
            "rust",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("c_comm"));

    let mut cmd_cross = Command::cargo_bin("actos").unwrap();
    cmd_cross
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args([
            "post",
            "create",
            "--title",
            "t",
            "--body",
            "b",
            "--cross-post",
            "c_source",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("c_cross"));
}

#[tokio::test]
async fn test_community_requires_auth_for_writes() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["community", "join", "rust"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("Authentication required"));
}
