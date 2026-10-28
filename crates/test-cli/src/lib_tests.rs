use tempfile::TempDir;

use super::*;

#[test]
fn run_rejects_definition_overrides() {
    for (flag, value) in [
        ("--command", "echo override"),
        ("--ticket", "ticket-override"),
        ("--slow-threshold-ms", "1"),
    ] {
        assert!(
            parse_cli_from(["test", "run", "--spec-id", "vt-fixed", flag, value]).is_err(),
            "{flag} must be defined only in the stored definition"
        );
    }
    assert!(
        parse_cli_from([
            "test",
            "run",
            "--spec-id",
            "vt-fixed",
            "--spec-id",
            "vt-other"
        ])
        .is_err()
    );
}

fn store_args(dir: &TempDir) -> Vec<String> {
    vec![
        "test".to_string(),
        "--workspace".to_string(),
        dir.path().to_string_lossy().to_string(),
    ]
}

#[test]
fn run_uses_stored_command_when_command_is_omitted() {
    let dir = TempDir::new().unwrap();
    let mut record = store_args(&dir);
    record.extend(
        [
            "record-spec",
            "--id",
            "vt-stored",
            "--title",
            "Stored command",
            "--ticket",
            "ticket-stored",
            "--command",
            "echo stored-command-ok",
            "--slow-threshold-ms",
            "0",
        ]
        .map(String::from),
    );
    run(parse_cli_from(record).unwrap()).unwrap();

    let mut args = store_args(&dir);
    args.extend(
        [
            "--json",
            "run",
            "--spec-id",
            "vt-stored",
            "--id",
            "stored-run",
            "--domain",
            "test",
            "--operation",
            "run",
            "--run-id",
            "stored-check",
            "--log-dir",
            dir.path().join("logs").to_str().unwrap(),
        ]
        .map(String::from),
    );
    let value = run_value(args);
    assert_eq!(value["outcome"], "passed");
    assert_eq!(value["slow_threshold_ms"], 0);
    let contents = std::fs::read_to_string(value["log_locator"].as_str().unwrap()).unwrap();
    assert!(contents.contains("stored-command-ok"));
    let mut get = store_args(&dir);
    get.extend(["--json", "get", "--id", "stored-run"].map(String::from));
    let stored = run_value(get);
    assert!(stored.to_string().contains("echo stored-command-ok"));
}

#[test]
fn run_stored_command_requires_an_existing_runnable_definition() {
    for command in [None, Some(""), Some("   ")] {
        let dir = TempDir::new().unwrap();
        let mut record = store_args(&dir);
        record
            .extend(["record-spec", "--id", "vt-empty", "--title", "No command"].map(String::from));
        if let Some(command) = command {
            record.extend(["--command", command].map(String::from));
        }
        run(parse_cli_from(record).unwrap()).unwrap();
        for id in ["vt-empty", "vt-missing"] {
            let mut args = store_args(&dir);
            args.extend(
                [
                    "run",
                    "--spec-id",
                    id,
                    "--log-dir",
                    dir.path().join("logs").to_str().unwrap(),
                ]
                .map(String::from),
            );
            let error = match run(parse_cli_from(args).unwrap()) {
                Err(error) => error.to_string(),
                Ok(_) => panic!("missing runnable definition must fail"),
            };
            assert!(error.contains(id), "{error}");
            assert!(!dir.path().join("logs").exists());
        }
        let mut list = store_args(&dir);
        list.extend(["--json", "list"].map(String::from));
        assert_eq!(run_value(list)["count"], 0);
    }
}

#[test]
fn run_inherits_definition_links_and_rejects_duplicate_provenance() {
    let dir = TempDir::new().unwrap();
    let config = TestStoreConfig::for_workspace(dir.path());
    let mut spec = test_api::ValidationSpec::new("vt-inherit", "Inherited definition");
    spec.command = Some("echo inherited".into());
    spec.links.ticket_ids = vec!["ticket-owner".into()];
    spec.links.acceptance_criterion_ids = vec!["criterion-owner".into()];
    spec.links.doc_evidence_ids = vec!["doc-owner".into()];
    spec.provenance.domain = Some("test".into());
    spec.provenance.operation = Some("run".into());
    config.record_spec(&spec).unwrap();
    let mut args = store_args(&dir);
    args.extend(
        [
            "--json",
            "run",
            "--spec-id",
            "vt-inherit",
            "--run-id",
            "inherit",
            "--log-dir",
            dir.path().join("logs").to_str().unwrap(),
        ]
        .map(String::from),
    );
    let value = run_value(args.clone());
    let execution = config
        .get_execution(value["execution_id"].as_str().unwrap())
        .unwrap();
    assert_eq!(execution.links.ticket_ids, spec.links.ticket_ids);
    assert_eq!(
        execution.links.acceptance_criterion_ids,
        spec.links.acceptance_criterion_ids
    );
    assert_eq!(
        execution.links.doc_evidence_ids,
        spec.links.doc_evidence_ids
    );
    assert_eq!(config.get_spec("vt-inherit").unwrap(), spec);
    for domain in ["test", "override"] {
        let mut duplicate = args.clone();
        duplicate.extend(["--domain", domain].map(String::from));
        assert!(matches!(
            run(parse_cli_from(duplicate).unwrap()),
            Err(CliRunError::BadRequest(_))
        ));
    }
}

#[test]
fn run_stored_failing_command_records_failed_outcome() {
    let dir = TempDir::new().unwrap();
    let mut record = store_args(&dir);
    record.extend(
        [
            "record-spec",
            "--id",
            "vt-failed",
            "--title",
            "Failure",
            "--ticket",
            "ticket-failure",
            "--command",
            "exit 7",
        ]
        .map(String::from),
    );
    run(parse_cli_from(record).unwrap()).unwrap();
    let mut args = store_args(&dir);
    args.extend(
        [
            "--json",
            "run",
            "--spec-id",
            "vt-failed",
            "--log-dir",
            dir.path().join("logs").to_str().unwrap(),
            "--domain",
            "test",
            "--operation",
            "run",
            "--run-id",
            "failure-check",
        ]
        .map(String::from),
    );
    let value = run_value(args);
    assert_eq!(value["outcome"], "failed");
    assert_eq!(value["exit_code"], 7);
}

#[test]
fn run_rejects_missing_provenance_before_starting_command() {
    let dir = TempDir::new().unwrap();
    let mut record = store_args(&dir);
    record.extend(
        [
            "record-spec",
            "--id",
            "vt-blank",
            "--title",
            "Blank override",
            "--ticket",
            "ticket-owned",
            "--command",
            "echo should-not-run",
        ]
        .map(String::from),
    );
    run(parse_cli_from(record).unwrap()).unwrap();
    let mut args = store_args(&dir);
    args.extend(
        [
            "run",
            "--spec-id",
            "vt-blank",
            "--log-dir",
            dir.path().join("logs").to_str().unwrap(),
        ]
        .map(String::from),
    );
    assert!(run(parse_cli_from(args).unwrap()).is_err());
    assert!(!dir.path().join("logs").exists());
}

#[test]
fn parses_record_command() {
    let cli = parse_cli_from([
        "test",
        "record",
        "--id",
        "exec-1",
        "--spec-id",
        "vt-a",
        "--outcome",
        "passed",
        "--ticket",
        "ticket-1",
    ])
    .expect("parse record");
    assert!(cli.workspace_root.is_none());
    match cli.command {
        TestCommand::Record(args) => {
            assert_eq!(args.id, "exec-1");
            assert_eq!(args.spec_id, "vt-a");
            assert_eq!(args.ticket_ids, vec!["ticket-1".to_string()]);
            assert!(matches!(args.outcome, OutcomeArg::Passed));
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn record_spec_rejects_ambient_workspace_aliases() {
    for selector in ["", "  ", "default", ".."] {
        let mut cli = parse_cli_from([
            "test",
            "--workspace",
            ".",
            "record-spec",
            "--id",
            "vt-selector",
            "--title",
            "Selector rejection",
        ])
        .unwrap();
        cli.workspace_root = Some(PathBuf::from(selector));

        match run(cli) {
            Err(CliRunError::BadRequest(message)) => {
                assert!(message.contains("requires an explicit workspace path"));
            }
            _ => panic!("ambient selector must be rejected before store access"),
        }
    }
}

#[test]
fn move_rejects_ambient_workspace_aliases_before_store_access() {
    let dir = TempDir::new().unwrap();
    let source_root = dir.path().join("not-created-source");
    let config = TestStoreConfig::new(&source_root);

    for selector in ["", "  ", "default", ".."] {
        let result = move_cmd::cmd_move(
            MoveArgs {
                kind: Some(RecordKindArg::Spec),
                id: Some("vt-missing".to_string()),
                to_workspace_root: Some(selector.to_string()),
                dry_run: true,
                resume: None,
                rollback: None,
            },
            &config,
        );
        assert!(result.is_err(), "selector {selector:?} must be rejected");
        assert!(!source_root.exists());
    }
}

#[test]
fn json_and_toon_conflict() {
    let result = parse_cli_from(["test", "--json", "--toon", "get", "--id", "exec-1"]);
    assert!(result.is_err());
}

#[test]
fn record_then_list_round_trips_through_store() {
    let dir = TempDir::new().unwrap();

    let mut spec_args = store_args(&dir);
    spec_args.extend(
        [
            "record-spec",
            "--id",
            "vt-core",
            "--title",
            "Core tests",
            "--command",
            "cargo test -p ticket-vscode-core",
            "--ticket",
            "ticket-parity",
        ]
        .map(String::from),
    );
    run(parse_cli_from(spec_args).unwrap()).expect("record spec");

    let mut exec_args = store_args(&dir);
    exec_args.extend(
        [
            "record",
            "--id",
            "exec-core",
            "--spec-id",
            "vt-core",
            "--outcome",
            "passed",
            "--detail",
            "16 passed",
            "--executed-at",
            "2026-06-15T12:00:00Z",
            "--domain",
            "test",
            "--operation",
            "record",
            "--run-id",
            "run-core",
            "--ticket",
            "ticket-parity",
        ]
        .map(String::from),
    );
    run(parse_cli_from(exec_args).unwrap()).expect("record execution");

    let mut list_args = store_args(&dir);
    list_args.extend(["--json", "list", "--ticket", "ticket-parity"].map(String::from));
    let output = run(parse_cli_from(list_args).unwrap()).expect("list");

    match output {
        CliOutput::Machine(value, MachineOutputFormat::Json) => {
            assert_eq!(value["count"], 1);
            assert_eq!(value["executions"][0]["id"], "exec-core");
            assert_eq!(value["executions"][0]["outcome"], "passed");
        }
        other => panic!(
            "unexpected output variant: {}",
            matches!(other, CliOutput::Text(_))
        ),
    }
}

#[test]
fn cli_producer_reads_back_selected_workspace_without_cross_writes() {
    let parent = TempDir::new().unwrap();
    let selected = parent.path().join("selected");
    let sibling = parent.path().join("sibling");
    std::fs::create_dir_all(&selected).unwrap();
    std::fs::create_dir_all(&sibling).unwrap();

    let selected_dir = TempDir::new_in(&selected).unwrap();
    let workspace = selected_dir.path();

    let mut spec_args = vec![
        "test".to_string(),
        "--workspace".to_string(),
        workspace.to_string_lossy().to_string(),
        "record-spec".to_string(),
        "--id".to_string(),
        "vt-cli-domain-store".to_string(),
        "--title".to_string(),
        "CLI domain-store fixture".to_string(),
    ];
    run(parse_cli_from(spec_args.drain(..)).unwrap()).expect("record CLI spec");

    let execution_args = [
        "test",
        "--workspace",
        &workspace.to_string_lossy(),
        "record",
        "--id",
        "exec-cli-domain-store",
        "--spec-id",
        "vt-cli-domain-store",
        "--outcome",
        "passed",
        "--executed-at",
        "2026-10-07T12:00:00Z",
        "--ticket",
        "b36ad4ef-77b9-4225-ab5e-abd0ba13b0b8",
        "--domain",
        "test-cli",
        "--operation",
        "record",
        "--run-id",
        "fixture",
    ];
    run(parse_cli_from(execution_args).unwrap()).expect("record CLI execution");

    let spec = run(parse_cli_from([
        "test",
        "--workspace",
        &workspace.to_string_lossy(),
        "--json",
        "get-spec",
        "--id",
        "vt-cli-domain-store",
    ])
    .unwrap())
    .expect("read CLI spec");
    let execution = run(parse_cli_from([
        "test",
        "--workspace",
        &workspace.to_string_lossy(),
        "--json",
        "get",
        "--id",
        "exec-cli-domain-store",
    ])
    .unwrap())
    .expect("read CLI execution");

    assert!(matches!(
        spec,
        CliOutput::Machine(value, MachineOutputFormat::Json)
            if value["title"] == "CLI domain-store fixture"
    ));
    assert!(matches!(
        execution,
        CliOutput::Machine(value, MachineOutputFormat::Json)
            if value["validation_spec_id"] == "vt-cli-domain-store"
    ));
    let store = workspace.join(".workflow-tools/test");
    assert!(store.join("specs/vt-cli-domain-store.json").is_file());
    assert!(
        store
            .join("executions/exec-cli-domain-store.json")
            .is_file()
    );
    assert!(!parent.path().join(".workflow-tools/test").exists());
    assert!(!sibling.join(".workflow-tools/test").exists());
}

#[test]
fn log_record_then_logs_round_trips_through_store() {
    let dir = TempDir::new().unwrap();

    let mut record_args = store_args(&dir);
    record_args.extend(
        [
            "--json",
            "log-record",
            "--id",
            "cap-1",
            "--execution",
            "exec-1",
            "--kind",
            "stderr",
            "--locator",
            "target/test-logs/x.log",
            "--ticket",
            "ticket-1",
        ]
        .map(String::from),
    );
    run(parse_cli_from(record_args).unwrap()).expect("record log capture");

    let canonical_capture_path = dir
        .path()
        .join(".workflow-tools/log/default/captures/cap-1.json");
    assert!(canonical_capture_path.is_file());
    assert!(!dir.path().join(".workflow-tools/.log").exists());
    let capture = LogStoreConfig::new(dir.path().join(".workflow-tools/log"), "default")
        .get_capture("cap-1")
        .expect("read capture from canonical log store");
    assert_eq!(capture.validation_execution_id, "exec-1");

    let mut logs_args = store_args(&dir);
    logs_args.extend(["--json", "logs", "--execution", "exec-1"].map(String::from));
    let output = run(parse_cli_from(logs_args).unwrap()).expect("list logs");

    match output {
        CliOutput::Machine(value, MachineOutputFormat::Json) => {
            assert_eq!(value["count"], 1);
            assert_eq!(value["captures"][0]["id"], "cap-1");
            assert_eq!(value["captures"][0]["kind"], "stderr");
        }
        other => panic!(
            "unexpected output variant: {}",
            matches!(other, CliOutput::Text(_))
        ),
    }
}

#[test]
fn audit_reports_failed_and_slow_counts() {
    let dir = TempDir::new().unwrap();

    let mut spec_args = store_args(&dir);
    spec_args.extend(
        [
            "record-spec",
            "--id",
            "vt-a",
            "--title",
            "A",
            "--slow-threshold-ms",
            "10",
        ]
        .map(String::from),
    );
    run(parse_cli_from(spec_args).unwrap()).expect("record spec");

    let mut fail_args = store_args(&dir);
    fail_args.extend(
        [
            "record",
            "--id",
            "exec-fail",
            "--spec-id",
            "vt-a",
            "--outcome",
            "failed",
            "--duration-ms",
            "50",
            "--executed-at",
            "2026-06-15T12:00:00Z",
            "--domain",
            "test",
            "--operation",
            "record",
            "--run-id",
            "run-audit",
            "--ticket",
            "ticket-audit",
        ]
        .map(String::from),
    );
    run(parse_cli_from(fail_args).unwrap()).expect("record failed execution");

    let mut audit_args = store_args(&dir);
    audit_args.extend(["--json", "audit"].map(String::from));
    let output = run(parse_cli_from(audit_args).unwrap()).expect("audit");

    match output {
        CliOutput::Machine(value, MachineOutputFormat::Json) => {
            assert_eq!(value["failed_count"], 1);
            assert_eq!(value["slow_count"], 1);
            assert_eq!(value["failed"][0]["id"], "exec-fail");
        }
        other => panic!(
            "unexpected output variant: {}",
            matches!(other, CliOutput::Text(_))
        ),
    }
}

fn run_value(args: Vec<String>) -> Value {
    match run(parse_cli_from(args).unwrap()).expect("run command") {
        CliOutput::Machine(value, MachineOutputFormat::Json) => value,
        _ => panic!("expected json output"),
    }
}

#[test]
fn run_passes_records_execution_and_capture() {
    let dir = TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    let mut spec = test_api::ValidationSpec::new("vt-a", "Harness");
    spec.command = Some("echo harness-ok".into());
    spec.links.ticket_ids = vec!["ticket-1".into()];
    TestStoreConfig::for_workspace(dir.path())
        .record_spec(&spec)
        .unwrap();

    let mut args = store_args(&dir);
    args.extend(
        [
            "--json",
            "run",
            "--spec-id",
            "vt-a",
            "--run-id",
            "run-1",
            "--domain",
            "test",
            "--operation",
            "run",
            "--log-dir",
            log_dir.to_string_lossy().as_ref(),
        ]
        .map(String::from),
    );
    let value = run_value(args);

    assert_eq!(value["status"], "ran");
    assert_eq!(value["outcome"], "passed");
    assert_eq!(value["exit_code"], 0);
    assert_eq!(value["execution_id"], "run-1-vt-a");
    assert_eq!(value["log_capture_id"], "run-1-vt-a-log");

    let locator = value["log_locator"].as_str().unwrap();
    let contents = std::fs::read_to_string(locator).expect("read log");
    assert!(contents.contains("harness-ok"));

    let mut list_args = store_args(&dir);
    list_args.extend(["--json", "list", "--ticket", "ticket-1"].map(String::from));
    let listed = run_value(list_args);
    assert_eq!(listed["count"], 1);
    assert_eq!(listed["executions"][0]["id"], "run-1-vt-a");
    assert!(listed["executions"][0]["duration_ms"].is_number());

    let mut logs_args = store_args(&dir);
    logs_args.extend(["--json", "logs", "--execution", "run-1-vt-a"].map(String::from));
    let logs = run_value(logs_args);
    assert_eq!(logs["count"], 1);
    assert_eq!(logs["captures"][0]["id"], "run-1-vt-a-log");
}

#[test]
fn run_failure_maps_to_failed_outcome() {
    let dir = TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    let mut spec = test_api::ValidationSpec::new("vt-fail", "Failure");
    spec.command = Some("exit 3".into());
    spec.links.ticket_ids = vec!["ticket-fail".into()];
    TestStoreConfig::for_workspace(dir.path())
        .record_spec(&spec)
        .unwrap();

    let mut args = store_args(&dir);
    args.extend(
        [
            "--json",
            "run",
            "--spec-id",
            "vt-fail",
            "--domain",
            "test",
            "--operation",
            "run",
            "--run-id",
            "run-fail",
            "--log-dir",
            log_dir.to_string_lossy().as_ref(),
        ]
        .map(String::from),
    );
    let value = run_value(args);

    assert_eq!(value["outcome"], "failed");
    assert_eq!(value["exit_code"], 3);
    assert_eq!(value["execution_id"], "run-fail-vt-fail");
}

#[test]
fn run_flags_over_budget_against_spec_threshold() {
    let dir = TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    let slow_command = if cfg!(windows) {
        "ping -n 2 127.0.0.1 >NUL"
    } else {
        "sleep 0.05"
    };

    let mut spec_args = store_args(&dir);
    spec_args.extend(
        [
            "record-spec",
            "--id",
            "vt-slow",
            "--title",
            "Slow",
            "--command",
            slow_command,
            "--ticket",
            "ticket-slow",
            "--slow-threshold-ms",
            "0",
        ]
        .map(String::from),
    );
    run(parse_cli_from(spec_args).unwrap()).expect("record spec");

    let mut args = store_args(&dir);
    args.extend(
        [
            "--json",
            "run",
            "--spec-id",
            "vt-slow",
            "--domain",
            "test",
            "--operation",
            "run",
            "--run-id",
            "run-slow",
            "--log-dir",
            log_dir.to_string_lossy().as_ref(),
        ]
        .map(String::from),
    );
    let value = run_value(args);

    assert_eq!(value["over_budget"], true);
    assert_eq!(value["slow_threshold_ms"], 0);
    assert!(value["duration_ms"].as_u64().unwrap() > 0);
}

#[test]
fn parses_move_command() {
    let cli = parse_cli_from([
        "test",
        "move",
        "--kind",
        "spec",
        "--to-workspace-root",
        "/tmp/target",
        "--dry-run",
        "vt-core",
    ])
    .expect("parse move");
    match cli.command {
        TestCommand::Move(args) => {
            assert!(matches!(args.kind, Some(RecordKindArg::Spec)));
            assert_eq!(args.id.as_deref(), Some("vt-core"));
            assert_eq!(args.to_workspace_root.as_deref(), Some("/tmp/target"));
            assert!(args.dry_run);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn move_dry_run_reports_blocked_plan_for_unmigrated_spec() {
    let repo = TempDir::new().unwrap();
    std::process::Command::new("git")
        .current_dir(repo.path())
        .args(["init"])
        .status()
        .expect("git init");
    let source_dir = repo.path().join("source");
    let target_dir = repo.path().join("target");
    std::fs::create_dir_all(&source_dir).unwrap();
    std::fs::create_dir_all(target_dir.join(".test")).unwrap();

    let mut record_args = vec![
        "test".to_string(),
        "--workspace".to_string(),
        source_dir.to_string_lossy().to_string(),
    ];
    record_args.extend(
        [
            "record-spec",
            "--id",
            "vt-core",
            "--title",
            "Core tests",
            "--ticket",
            "ticket-parity",
        ]
        .map(String::from),
    );
    run(parse_cli_from(record_args).unwrap()).expect("record spec");

    let mut move_args = vec![
        "test".to_string(),
        "--workspace".to_string(),
        source_dir.to_string_lossy().to_string(),
    ];
    move_args.extend(
        [
            "--json",
            "move",
            "--kind",
            "spec",
            "--to-workspace-root",
            target_dir.to_string_lossy().as_ref(),
            "--dry-run",
            "vt-core",
        ]
        .map(String::from),
    );
    let value = run_value(move_args);
    // The spec has not been migrated into a canonical entity folder yet, so
    // the move plan is blocked at the source.
    assert_eq!(value["status"], "blocked");
    assert_eq!(value["plan"]["supported"], false);
}

#[test]
fn move_requires_kind() {
    let dir = TempDir::new().unwrap();
    let mut move_args = store_args(&dir);
    move_args.extend(
        [
            "move",
            "--to-workspace-root",
            "/tmp/target",
            "--dry-run",
            "vt-core",
        ]
        .map(String::from),
    );
    let result = run(parse_cli_from(move_args).unwrap());
    assert!(result.is_err());
}

#[test]
fn resolve_spec_root_prefers_canonical_store_over_conflicting_legacy_index() {
    let dir = TempDir::new().unwrap();
    let canonical = memory_kernel::workspace::canonical_store_root(dir.path(), ".spec");
    let test_store = memory_kernel::workspace::canonical_store_root(dir.path(), ".test");
    std::fs::create_dir_all(&canonical).unwrap();
    std::fs::create_dir_all(&test_store).unwrap();
    std::fs::create_dir_all(dir.path().join(".spec")).unwrap();

    let root = resolve_spec_root(&test_store, Some(dir.path()));
    assert_eq!(root, canonical);
    assert_ne!(root, dir.path().join(".spec"));
}

#[test]
fn resolve_spec_root_uses_canonical_store_for_ambient_workspace() {
    let dir = TempDir::new().unwrap();
    let test_store = memory_kernel::workspace::canonical_store_root(dir.path(), ".test");
    std::fs::create_dir_all(&test_store).unwrap();
    let legacy_sibling = test_store.parent().unwrap().join(".spec");
    std::fs::create_dir_all(&legacy_sibling).unwrap();
    let canonical = memory_kernel::workspace::canonical_store_root(dir.path(), ".spec");

    let root = resolve_spec_root(&test_store, None);

    assert_eq!(root, canonical);
    assert_ne!(root, legacy_sibling);
}

#[test]
fn record_verifies_linked_spec_in_canonical_store_not_legacy_index() {
    use spec_api::{SpecManifest, SpecStore};

    let dir = TempDir::new().unwrap();
    let body = "# Spec\n\n## Guards\n- `vt-guard`\n";
    let canonical = memory_kernel::workspace::canonical_store_root(dir.path(), ".spec");
    let test_store = memory_kernel::workspace::canonical_store_root(dir.path(), ".test");
    let manifest = SpecManifest::new("root/shared", "Shared spec", "cmp");
    let mut canonical_store = SpecStore::init(&canonical).unwrap();
    let spec_id = canonical_store
        .create(&manifest, body, None)
        .unwrap()
        .to_string();
    for state in ["reviewed", "approved", "implemented"] {
        canonical_store
            .update(&spec_id, std::collections::BTreeMap::new(), Some(state))
            .unwrap();
    }

    // The same id is already verified in the wrongly derived legacy index.
    let legacy = test_store.parent().unwrap().join(".spec");
    let mut legacy_store = SpecStore::init(&legacy).unwrap();
    legacy_store.create(&manifest, body, None).unwrap();
    for state in ["reviewed", "approved", "implemented", "verified"] {
        legacy_store
            .update(&spec_id, std::collections::BTreeMap::new(), Some(state))
            .unwrap();
    }

    let mut args = store_args(&dir);
    args.extend(
        [
            "--json",
            "record",
            "--id",
            "exec-1",
            "--spec-id",
            "vt-guard",
            "--outcome",
            "passed",
            "--spec",
            spec_id.as_str(),
            "--domain",
            "test",
            "--operation",
            "run",
            "--run-id",
            "run-1",
        ]
        .map(String::from),
    );
    let value = run_value(args);
    assert_eq!(value["verified_spec_ids"][0], spec_id.as_str(), "{value}");
    assert_eq!(
        value["spec_verification"][0]["status"], "verified",
        "{value}"
    );

    let canonical_store = SpecStore::open_or_init(&canonical).unwrap();
    assert_eq!(
        canonical_store.get(&spec_id).unwrap().state(),
        Some("verified")
    );
    let legacy_store = SpecStore::open_or_init(&legacy).unwrap();
    assert_eq!(
        legacy_store.get(&spec_id).unwrap().state(),
        Some("verified")
    );
}

#[test]
fn run_verifies_specs_linked_from_the_validation_spec() {
    use spec_api::{SpecManifest, SpecStore};

    let dir = TempDir::new().unwrap();
    let body = "# Spec\n\n## Guards\n- `vt-guard`\n";
    let canonical = memory_kernel::workspace::canonical_store_root(dir.path(), ".spec");
    let mut canonical_store = SpecStore::init(&canonical).unwrap();
    let spec_id = canonical_store
        .create(
            &SpecManifest::new("root/run-guard", "Run guard", "cmp"),
            body,
            None,
        )
        .unwrap()
        .to_string();
    for state in ["reviewed", "approved", "implemented"] {
        canonical_store
            .update(&spec_id, std::collections::BTreeMap::new(), Some(state))
            .unwrap();
    }

    let mut spec_args = store_args(&dir);
    spec_args.extend(
        [
            "record-spec",
            "--id",
            "vt-guard",
            "--title",
            "Run guard validation",
            "--command",
            "echo guard-passed",
            "--spec",
            spec_id.as_str(),
        ]
        .map(String::from),
    );
    run(parse_cli_from(spec_args).unwrap()).expect("record validation spec");

    let log_dir = dir.path().join("logs");
    let mut run_args = store_args(&dir);
    run_args.extend(
        [
            "--json",
            "run",
            "--spec-id",
            "vt-guard",
            "--domain",
            "test",
            "--operation",
            "run",
            "--run-id",
            "run-guard",
            "--log-dir",
            log_dir.to_string_lossy().as_ref(),
        ]
        .map(String::from),
    );
    let value = run_value(run_args);

    assert_eq!(value["verified_spec_ids"][0], spec_id.as_str(), "{value}");
    assert_eq!(
        value["spec_verification"][0]["status"], "verified",
        "{value}"
    );
}
