use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_report_create_and_duplicate_conflict_exit_7() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. Report create success
    Mock::given(method("POST"))
        .and(path("/reports"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "rep_123",
            "target_type": "post",
            "target_id": "c_post1",
            "reason": "Spam içerik",
            "status": "pending",
            "notes": null,
            "created_at": "2026-09-02T12:00:00Z",
            "resolved_at": null
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // 2. Duplicate report returns 409 Conflict
    Mock::given(method("POST"))
        .and(path("/reports"))
        .respond_with(ResponseTemplate::new(409).set_body_json(serde_json::json!({
            "type": "https://docs.actos.dev/errors/conflict",
            "title": "Bu hedef hakkında zaten açık bir şikayetiniz var",
            "status": 409,
            "code": "CONFLICT"
        })))
        .mount(&mock_server)
        .await;

    // Report create succeeds
    let mut cmd_create = Command::cargo_bin("actos").unwrap();
    cmd_create
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args([
            "report",
            "create",
            "--target",
            "c_post1",
            "--type",
            "post",
            "--reason",
            "Spam içerik",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Report created successfully (ID: rep_123).",
        ));

    // Duplicate report -> exit code 7 (CONFLICT)
    let mut cmd_dup = Command::cargo_bin("actos").unwrap();
    cmd_dup
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_test_key")
        .args([
            "report",
            "create",
            "--target",
            "c_post1",
            "--type",
            "post",
            "--reason",
            "Spam içerik",
        ])
        .assert()
        .code(7); // ExitCode::Conflict
}

#[tokio::test]
async fn test_admin_reports_list_and_update() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. List reports
    Mock::given(method("GET"))
        .and(path("/admin/reports"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "reports": [
                {
                    "id": "rep_1",
                    "target_type": "post",
                    "target_id": "c_p1",
                    "reason": "Nefret söylemi",
                    "status": "pending",
                    "notes": null,
                    "created_at": "2026-09-02T12:00:00Z",
                    "resolved_at": null
                }
            ],
            "next_cursor": null
        })))
        .mount(&mock_server)
        .await;

    // 2. Update report
    Mock::given(method("PATCH"))
        .and(path("/admin/reports/rep_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "rep_1",
            "target_type": "post",
            "target_id": "c_p1",
            "reason": "Nefret söylemi",
            "status": "resolved",
            "notes": "İçerik silindi",
            "created_at": "2026-09-02T12:00:00Z",
            "resolved_at": "2026-09-02T13:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    // List
    let mut cmd_list = Command::cargo_bin("actos").unwrap();
    cmd_list
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_admin_key")
        .args(["admin", "reports", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("rep_1"))
        .stdout(predicate::str::contains("pending"))
        .stdout(predicate::str::contains("Nefret söylemi"));

    // Update
    let mut cmd_upd = Command::cargo_bin("actos").unwrap();
    cmd_upd
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_admin_key")
        .args([
            "admin",
            "reports",
            "update",
            "rep_1",
            "--status",
            "resolved",
            "--notes",
            "İçerik silindi",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Report 'rep_1' updated to 'resolved'.",
        ));
}

#[tokio::test]
async fn test_admin_content_delete_requires_yes() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("DELETE"))
        .and(path("/admin/contents/c_bad_post"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // 1. Without --yes -> exit code 2
    let mut cmd_no_yes = Command::cargo_bin("actos").unwrap();
    cmd_no_yes
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_admin_key")
        .args([
            "admin",
            "content",
            "delete",
            "c_bad_post",
            "--reason",
            "Kurallara aykırı",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Pass '--yes' to confirm"));

    // 2. With --yes -> succeeds
    let mut cmd_yes = Command::cargo_bin("actos").unwrap();
    cmd_yes
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_admin_key")
        .args([
            "admin",
            "content",
            "delete",
            "c_bad_post",
            "--reason",
            "Kurallara aykırı",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Content 'c_bad_post' deleted by moderator.",
        ));
}

#[tokio::test]
async fn test_admin_bans_and_permissions() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // 1. Ban add (platform-wide)
    Mock::given(method("POST"))
        .and(path("/admin/bans"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "username": "troll",
            "reason": "Taciz",
            "banned_at": "2026-09-02T12:00:00Z",
            "expires_at": null,
            "community": null
        })))
        .mount(&mock_server)
        .await;

    // 2. Ban remove (platform-wide)
    Mock::given(method("DELETE"))
        .and(path("/admin/bans/troll"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // 3. Permission grant (global)
    Mock::given(method("PUT"))
        .and(path("/admin/permissions"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // 4. Permission revoke (global)
    Mock::given(method("DELETE"))
        .and(path("/admin/permissions"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // Ban add
    let mut cmd_ban = Command::cargo_bin("actos").unwrap();
    cmd_ban
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_admin_key")
        .args(["admin", "ban", "add", "troll", "--reason", "Taciz"])
        .assert()
        .success()
        .stdout(predicate::str::contains("User 'troll' has been banned."));

    // Ban remove
    let mut cmd_unban = Command::cargo_bin("actos").unwrap();
    cmd_unban
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_admin_key")
        .args(["admin", "ban", "remove", "troll"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Ban removed for user 'troll'."));

    // Permission grant
    let mut cmd_grant = Command::cargo_bin("actos").unwrap();
    cmd_grant
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_admin_key")
        .args([
            "admin",
            "permission",
            "grant",
            "moderator_candidate",
            "--permission",
            "content.delete",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Permission 'content.delete' granted to 'moderator_candidate'.",
        ));

    // Permission revoke
    let mut cmd_revoke = Command::cargo_bin("actos").unwrap();
    cmd_revoke
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_admin_key")
        .args([
            "admin",
            "permission",
            "revoke",
            "moderator_candidate",
            "--permission",
            "content.delete",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Permission 'content.delete' revoked from 'moderator_candidate'.",
        ));
}

#[tokio::test]
async fn test_admin_community_scoped_ban_and_permission() {
    use wiremock::matchers::body_partial_json;

    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    // Community-scoped ban with post deletion.
    Mock::given(method("POST"))
        .and(path("/admin/bans"))
        .and(body_partial_json(serde_json::json!({
            "username": "troll",
            "community": "rust",
            "delete_posts": true
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "username": "troll",
            "reason": "Taciz",
            "banned_at": "2026-09-02T12:00:00Z",
            "expires_at": null,
            "community": "rust"
        })))
        .mount(&mock_server)
        .await;

    // Community-scoped ban removal carries the ?community= query.
    Mock::given(method("DELETE"))
        .and(path("/admin/bans/troll"))
        .and(wiremock::matchers::query_param("community", "rust"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // Community-scoped permission grant.
    Mock::given(method("PUT"))
        .and(path("/admin/permissions"))
        .and(body_partial_json(serde_json::json!({
            "username": "alice",
            "permission": "member.kick",
            "community": "rust"
        })))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let mut cmd_ban = Command::cargo_bin("actos").unwrap();
    cmd_ban
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_admin_key")
        .args([
            "admin",
            "ban",
            "add",
            "troll",
            "--reason",
            "Taciz",
            "--community",
            "rust",
            "--delete-posts",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "User 'troll' has been banned from community 'rust'.",
        ));

    let mut cmd_unban = Command::cargo_bin("actos").unwrap();
    cmd_unban
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_admin_key")
        .args(["admin", "ban", "remove", "troll", "--community", "rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Community ban removed for user 'troll' in 'rust'.",
        ));

    let mut cmd_grant = Command::cargo_bin("actos").unwrap();
    cmd_grant
        .env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_admin_key")
        .args([
            "admin",
            "permission",
            "grant",
            "alice",
            "--permission",
            "member.kick",
            "--community",
            "rust",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Permission 'member.kick' granted to 'alice' in 'rust'.",
        ));
}

#[tokio::test]
async fn test_admin_unauthorized_fails_with_code_4() {
    let mock_server = MockServer::start().await;
    let tmp_config = NamedTempFile::new().unwrap();

    Mock::given(method("GET"))
        .and(path("/admin/reports"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "type": "https://docs.actos.dev/errors/forbidden",
            "title": "Bu işlem için admin rolü gereklidir",
            "status": 403,
            "code": "FORBIDDEN"
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("actos").unwrap();
    cmd.env("ACTOS_CONFIG", tmp_config.path())
        .env("ACTOS_API_URL", mock_server.uri())
        .env("ACTOS_API_KEY", "actos_regular_user_key")
        .args(["admin", "reports", "list"])
        .assert()
        .code(4); // ExitCode::Forbidden
}
