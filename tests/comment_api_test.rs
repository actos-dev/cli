use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_comment_create_root_and_nested() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. Root comment
    Mock::given(method("POST"))
        .and(path("/posts/c_post123/comments"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "c_com_root",
            "content_type": "comment",
            "author": {
                "id": "a_u1",
                "username": "alice",
                "actor_type": "human",
                "display_name": null,
                "bio": null,
                "created_at": "2026-09-02T12:00:00Z"
            },
            "author_deleted": false,
            "title": null,
            "body": "Harika bir gönderi!",
            "body_format": "plain",
            "metadata": {},
            "tags": [],
            "score": 0,
            "upvotes": 0,
            "downvotes": 0,
            "comment_count": 0,
            "created_at": "2026-09-02T12:00:00Z",
            "edited_at": null,
            "deleted": false
        })))
        .mount(&mock_server)
        .await;

    let mut cmd_root = Command::cargo_bin("actos").unwrap();
    cmd_root
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args([
            "comment",
            "create",
            "https://actos.com.tr/posts/c_post123",
            "--body",
            "Harika bir gönderi!",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Comment created: c_com_root"));

    // 2. Nested reply with --parent
    let mut cmd_nested = Command::cargo_bin("actos").unwrap();
    cmd_nested
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args([
            "comment",
            "create",
            "c_post123",
            "--body",
            "Katılıyorum!",
            "--parent",
            "https://actos.com.tr/comments/c_com_root",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Comment created: c_com_root"));
}

#[tokio::test]
async fn test_comment_view_normal_and_deleted() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. Normal comment view
    Mock::given(method("GET"))
        .and(path("/comments/c_active"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "comment": {
                "id": "c_active",
                "content_type": "comment",
                "author": {
                    "id": "a_u1",
                    "username": "bob",
                    "actor_type": "human",
                    "display_name": null,
                    "bio": null,
                    "created_at": "2026-09-02T12:00:00Z"
                },
                "author_deleted": false,
                "title": null,
                "body": "Aktif yorum gövdesi",
                "body_format": "plain",
                "metadata": {},
                "tags": [],
                "score": 3,
                "upvotes": 3,
                "downvotes": 0,
                "comment_count": 0,
                "created_at": "2026-09-02T12:00:00Z",
                "edited_at": null,
                "deleted": false
            },
            "ancestors": [
                {
                    "id": "c_post_root",
                    "content_type": "post",
                    "author": {
                        "id": "a_u0",
                        "username": "alice",
                        "actor_type": "human",
                        "display_name": null,
                        "bio": null,
                        "created_at": "2026-09-02T12:00:00Z"
                    },
                    "author_deleted": false,
                    "title": "Kök Gönderi",
                    "body": "Gövde",
                    "body_format": "markdown",
                    "metadata": {},
                    "tags": [],
                    "score": 10,
                    "upvotes": 10,
                    "downvotes": 0,
                    "comment_count": 1,
                    "created_at": "2026-09-02T12:00:00Z",
                    "edited_at": null,
                    "deleted": false
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    // 2. Deleted comment view (deleted: true -> 200 OK + [silindi])
    Mock::given(method("GET"))
        .and(path("/comments/c_deleted"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "comment": {
                "id": "c_deleted",
                "content_type": "comment",
                "author": {
                    "id": "a_u1",
                    "username": "bob",
                    "actor_type": "human",
                    "display_name": null,
                    "bio": null,
                    "created_at": "2026-09-02T12:00:00Z"
                },
                "author_deleted": true,
                "title": null,
                "body": "[silindi]",
                "body_format": "plain",
                "metadata": {},
                "tags": [],
                "score": 0,
                "upvotes": 0,
                "downvotes": 0,
                "comment_count": 0,
                "created_at": "2026-09-02T12:00:00Z",
                "edited_at": null,
                "deleted": true
            },
            "ancestors": []
        })))
        .mount(&mock_server)
        .await;

    // View active comment
    let mut cmd_act = Command::cargo_bin("actos").unwrap();
    cmd_act
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["comment", "view", "c_active"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Aktif yorum gövdesi"))
        .stdout(predicate::str::contains("bob (human)"))
        .stdout(predicate::str::contains("Kök Gönderi"));

    // View deleted comment: exit code MUST BE 0 and display [silindi]
    let mut cmd_del = Command::cargo_bin("actos").unwrap();
    cmd_del
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["comment", "view", "c_deleted"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[silindi]"));
}

#[tokio::test]
async fn test_comment_list_indented_tree() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/posts/c_post123/comments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "comments": [
                {
                    "id": "c_root_1",
                    "content_type": "comment",
                    "author": {
                        "id": "a_u1",
                        "username": "alice",
                        "actor_type": "human",
                        "display_name": null,
                        "bio": null,
                        "created_at": "2026-09-02T12:00:00Z"
                    },
                    "author_deleted": false,
                    "title": null,
                    "body": "Birinci seviye yorum",
                    "body_format": "plain",
                    "metadata": {},
                    "tags": [],
                    "score": 5,
                    "upvotes": 5,
                    "downvotes": 0,
                    "comment_count": 1,
                    "created_at": "2026-09-02T12:00:00Z",
                    "edited_at": null,
                    "deleted": false,
                    "replies": [
                        {
                            "id": "c_child_1",
                            "content_type": "comment",
                            "author": {
                                "id": "a_u2",
                                "username": "bob",
                                "actor_type": "human",
                                "display_name": null,
                                "bio": null,
                                "created_at": "2026-09-02T12:00:00Z"
                            },
                            "author_deleted": false,
                            "title": null,
                            "body": "İkinci seviye yanıt",
                            "body_format": "plain",
                            "metadata": {},
                            "tags": [],
                            "score": 2,
                            "upvotes": 2,
                            "downvotes": 0,
                            "comment_count": 0,
                            "created_at": "2026-09-02T12:00:00Z",
                            "edited_at": null,
                            "deleted": false,
                            "replies": []
                        }
                    ]
                }
            ],
            "next_cursor": null
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .args(["comment", "list", "c_post123"])
        .assert()
        .success()
        .stdout(predicate::str::contains("@alice (c_root_1)"))
        .stdout(predicate::str::contains("Birinci seviye yorum"))
        .stdout(predicate::str::contains("@bob (c_child_1)"))
        .stdout(predicate::str::contains("İkinci seviye yanıt"));
}

#[tokio::test]
async fn test_comment_edit_and_delete() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. Edit
    Mock::given(method("PATCH"))
        .and(path("/comments/c_com_edit"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "c_com_edit",
            "content_type": "comment",
            "author": {
                "id": "a_u1",
                "username": "alice",
                "actor_type": "human",
                "display_name": null,
                "bio": null,
                "created_at": "2026-09-02T12:00:00Z"
            },
            "author_deleted": false,
            "title": null,
            "body": "Düzenlenmiş yorum gövdesi",
            "body_format": "plain",
            "metadata": {},
            "tags": [],
            "score": 0,
            "upvotes": 0,
            "downvotes": 0,
            "comment_count": 0,
            "created_at": "2026-09-02T12:00:00Z",
            "edited_at": "2026-09-02T13:00:00Z",
            "deleted": false
        })))
        .mount(&mock_server)
        .await;

    // 2. Delete
    Mock::given(method("DELETE"))
        .and(path("/comments/c_com_del"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // Edit
    let mut cmd_edit = Command::cargo_bin("actos").unwrap();
    cmd_edit
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args([
            "comment",
            "edit",
            "c_com_edit",
            "--body",
            "Düzenlenmiş yorum gövdesi",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Comment 'c_com_edit' updated successfully.",
        ));

    // Delete without --yes -> exit code 2 (USAGE)
    let mut cmd_del_fail = Command::cargo_bin("actos").unwrap();
    cmd_del_fail
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["comment", "delete", "c_com_del"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Pass '--yes' to confirm"));

    // Delete with --yes -> exit code 0
    let mut cmd_del_ok = Command::cargo_bin("actos").unwrap();
    cmd_del_ok
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args(["comment", "delete", "c_com_del", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Comment 'c_com_del' deleted."));
}
