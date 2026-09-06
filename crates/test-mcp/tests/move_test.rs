//! Move-tool tests for test-mcp: verifies preflight/apply/resume/rollback
//! routing through the shared move kernel preserves journal semantics.

use rmcp::handler::server::wrapper::Parameters;
use serde_json::Value;
use test_mcp::server::{
    RecordExecutionInput,
    RecordSpecInput,
    TestMoveInput,
    TestMoveJournalInput,
    TestServer,
};

fn init_git(dir: &std::path::Path) {
    let status = std::process::Command::new("git")
        .current_dir(dir)
        .args(["init"])
        .status()
        .expect("git init");
    assert!(status.success());
}

async fn seeded_server() -> (tempfile::TempDir, TestServer, std::path::PathBuf) {
    let repo = tempfile::tempdir().expect("tempdir");
    init_git(repo.path());
    let source = repo.path().join("source");
    let target = repo.path().join("target");
    let source_store = source.join(".workflow-tools").join("test");
    let target_store = target.join(".workflow-tools").join("test");
    std::fs::create_dir_all(&source_store).expect("create source test store");
    std::fs::create_dir_all(&target_store).expect("create target test store");

    let server = TestServer::new(source_store.clone());

    server
        .test_record_spec(Parameters(RecordSpecInput {
            workspace: source.to_string_lossy().to_string(),
            id: "vt-core".to_string(),
            title: "Core tests".to_string(),
            command: None,
            detail: None,
            slow_threshold_ms: None,
            ticket_ids: vec!["ticket-1".to_string()],
            spec_ids: vec![],
            acceptance_criterion_ids: vec![],
        }))
        .await
        .expect("record spec");

    server
        .test_record_execution(Parameters(RecordExecutionInput {
            workspace: source.to_string_lossy().to_string(),
            id: "exec-core-1".to_string(),
            validation_spec_id: "vt-core".to_string(),
            outcome: "passed".to_string(),
            executed_at: None,
            duration_ms: Some(120),
            throughput: None,
            detail: None,
            ticket_ids: vec!["ticket-1".to_string()],
            spec_ids: vec![],
            acceptance_criterion_ids: vec![],
            doc_evidence_ids: vec![],
            log_ids: vec![],
            source_path: None,
            test_id: None,
            domain: Some("test".to_string()),
            operation: Some("move".to_string()),
            transport: Some("mcp".to_string()),
            run_id: Some("run-1".to_string()),
        }))
        .await
        .expect("record execution");

    // Migrate legacy records into canonical UUID entity folders so the move
    // kernel has something to plan against.
    let config = test_api::TestStoreConfig::new(source_store);
    config.migrate_to_canonical().expect("migrate");

    (repo, server, target)
}

#[tokio::test]
async fn move_preflight_reports_supported_plan_after_migration() {
    let (_repo, server, target) = seeded_server().await;

    let result = server
        .test_move_preflight(Parameters(TestMoveInput {
            workspace: None,
            kind: "spec".to_string(),
            id: "vt-core".to_string(),
            to_workspace_root: target.to_string_lossy().to_string(),
        }))
        .await
        .expect("preflight");
    let json = extract_json(result);
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn move_apply_resume_rollback_round_trip() {
    let (_repo, server, target) = seeded_server().await;

    let apply = server
        .test_move_apply(Parameters(TestMoveInput {
            workspace: None,
            kind: "execution".to_string(),
            id: "exec-core-1".to_string(),
            to_workspace_root: target.to_string_lossy().to_string(),
        }))
        .await
        .expect("apply");
    let apply_json = extract_json(apply);
    let journal_id = apply_json["journal_id"].as_str().unwrap().to_string();

    let resumed = server
        .test_move_resume(Parameters(TestMoveJournalInput {
            workspace: None,
            kind: "execution".to_string(),
            id: journal_id.clone(),
        }))
        .await
        .expect("resume");
    assert_eq!(extract_json(resumed)["mode"], "resume");

    let rolled_back = server
        .test_move_rollback(Parameters(TestMoveJournalInput {
            workspace: None,
            kind: "execution".to_string(),
            id: journal_id,
        }))
        .await
        .expect("rollback");
    assert_eq!(extract_json(rolled_back)["mode"], "rollback");
}

#[tokio::test]
async fn move_apply_blocked_without_preflight_returns_error() {
    let (_repo, server, target) = seeded_server().await;

    let result = server
        .test_move_apply(Parameters(TestMoveInput {
            workspace: None,
            kind: "spec".to_string(),
            id: "never-migrated".to_string(),
            to_workspace_root: target.to_string_lossy().to_string(),
        }))
        .await;
    assert!(result.is_err());
}

fn extract_json(result: rmcp::model::CallToolResult) -> Value {
    result
        .content
        .iter()
        .find_map(|content| {
            if let rmcp::model::RawContent::Text(text) = &content.raw {
                Some(text.text.clone())
            } else {
                None
            }
        })
        .map(|text| serde_json::from_str(&text).expect("parse json"))
        .expect("text content")
}
