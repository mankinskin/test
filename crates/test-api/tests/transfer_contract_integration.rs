//! Temporary-root parity test for the test-domain transfer contract
//! (see `transcripts/03-09-2026_repository-entity-distribution/03-test-transfer-contract.md`).
//!
//! Migrates the fixture `workflow-tools/test/test-fixtures/transfer-fixture`
//! legacy JSON records into canonical UUID entity folders, moves one spec and
//! one execution into a canonical `workflow-tools/test/.test` destination,
//! rolls both back, and confirms discovery finds the same physical store
//! (same `entity_id`, `canonical_path`, `digest`) whether scanned from a
//! synthesized `meta-workspace` root or directly from `workflow-tools/test`.

use std::{
    fs,
    path::{
        Path,
        PathBuf,
    },
};

use memory_kernel::ContentKind;
use test_api::{
    TestRecordKind,
    TestStoreConfig,
};
use uuid::Uuid;

fn run_git(
    repo_root: &Path,
    args: &[&str],
) {
    let status = std::process::Command::new("git")
        .current_dir(repo_root)
        .args(args)
        .status()
        .expect("git command");
    assert!(status.success(), "git {args:?} failed: {status}");
}

fn copy_dir_recursive(
    from: &Path,
    to: &Path,
) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir_recursive(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), &dest).unwrap();
        }
    }
}

/// Read every canonical entity folder under `store_root` (whichever store the
/// `.test` marker resolves to), returning `(entity_id, canonical_path,
/// digest)` tuples sorted by id, across both spec and execution kinds.
fn discovery_tuples(store_root: &Path) -> Vec<(Uuid, PathBuf, String)> {
    let config = TestStoreConfig::new(store_root.to_path_buf());
    let mut tuples = Vec::new();
    for kind in [TestRecordKind::Spec, TestRecordKind::Execution] {
        let dir = store_root.join(kind.subdir());
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Ok(id) = name.parse::<Uuid>() else {
                continue;
            };
            if !path.join("entity.json").is_file() {
                continue;
            }
            let canonical_path = Path::new(kind.subdir())
                .join(name)
                .join("entity.json");
            let digest = config.canonical_entity_digest(kind, &id).unwrap();
            tuples.push((id, canonical_path, digest));
        }
    }
    tuples.sort_by_key(|(id, _, _)| *id);
    tuples
}

#[test]
fn test_transfer_contract_temporary_root_parity() {
    let temp = tempfile::tempdir().unwrap();
    let meta_workspace = temp.path().join("meta-workspace");
    let workflow_tools_test = meta_workspace.join("workflow-tools").join("test");
    fs::create_dir_all(&workflow_tools_test).unwrap();
    run_git(&meta_workspace, &["init"]);

    // Seed the source fixture into the temporary root.
    let fixture_source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("test-api crate root")
        .parent()
        .expect("test/crates parent")
        .join("test-fixtures")
        .join("transfer-fixture");
    let fixture_target =
        workflow_tools_test.join("test-fixtures").join("transfer-fixture");
    copy_dir_recursive(&fixture_source, &fixture_target);

    let source_store_root = fixture_target.join(".test");
    let source_config = TestStoreConfig::new(source_store_root.clone());

    // Canonicalize the legacy fixture JSON before any move (never move legacy
    // JSON directly).
    source_config.migrate_to_canonical().unwrap();

    let spec_id = source_config.resolve_canonical_spec_id("vt-fixture-spec");
    let execution_id =
        source_config.resolve_canonical_execution_id("exec-fixture-1");
    let source_spec_digest = source_config
        .canonical_entity_digest(TestRecordKind::Spec, &spec_id)
        .unwrap();
    let source_execution_digest = source_config
        .canonical_entity_digest(TestRecordKind::Execution, &execution_id)
        .unwrap();

    // The destination store is presumed already onboarded: pre-create the
    // (empty) canonical `.test` container the same way an already-migrated
    // sibling store would leave it.
    fs::create_dir_all(workflow_tools_test.join(".workflow-tools").join("test")).unwrap();

    let spec_plan = source_config
        .plan_move_preflight(
            TestRecordKind::Spec,
            "vt-fixture-spec",
            &workflow_tools_test,
        )
        .unwrap();
    assert!(spec_plan.supported(), "blockers: {:?}", spec_plan.blockers);
    let spec_outcome = source_config
        .execute_move_with_journal(TestRecordKind::Spec, &spec_plan)
        .unwrap();

    let execution_plan = source_config
        .plan_move_preflight(
            TestRecordKind::Execution,
            "exec-fixture-1",
            &workflow_tools_test,
        )
        .unwrap();
    assert!(
        execution_plan.supported(),
        "blockers: {:?}",
        execution_plan.blockers
    );
    let execution_outcome = source_config
        .execute_move_with_journal(TestRecordKind::Execution, &execution_plan)
        .unwrap();

    let target_config = TestStoreConfig::for_workspace(&workflow_tools_test);
    assert_eq!(
        target_config
            .canonical_entity_digest(TestRecordKind::Spec, &spec_id)
            .unwrap(),
        source_spec_digest,
        "spec digest must survive the move"
    );
    assert_eq!(
        target_config
            .canonical_entity_digest(TestRecordKind::Execution, &execution_id)
            .unwrap(),
        source_execution_digest,
        "execution digest must survive the move"
    );
    assert!(!source_config.canonical_spec_exists("vt-fixture-spec").unwrap());
    assert!(
        !source_config
            .canonical_execution_exists("exec-fixture-1")
            .unwrap()
    );

    // Discovery parity: scanning from the synthesized "meta-workspace" root
    // and scanning directly from "workflow-tools/test" must agree on the
    // same physical destination store (the nested fixture copy also carries
    // a `.test` marker, so filter by workspace_root to select the canonical
    // destination rather than the fixture's own leftover store).
    let from_root = memory_kernel::discover_stores(&meta_workspace);
    let from_submodule = memory_kernel::discover_stores(&workflow_tools_test);
    let canonical_workspace = fs::canonicalize(&workflow_tools_test).unwrap();

    let test_store_from_root = from_root
        .iter()
        .find(|store| {
            store.kind == ContentKind::Test
                && fs::canonicalize(&store.workspace_root).ok().as_ref()
                    == Some(&canonical_workspace)
        })
        .expect("root discovery finds the canonical test store");
    let test_store_from_submodule = from_submodule
        .iter()
        .find(|store| {
            store.kind == ContentKind::Test
                && fs::canonicalize(&store.workspace_root).ok().as_ref()
                    == Some(&canonical_workspace)
        })
        .expect("submodule discovery finds the canonical test store");

    let canonical_root_a =
        fs::canonicalize(&test_store_from_root.store_root).unwrap();
    let canonical_root_b =
        fs::canonicalize(&test_store_from_submodule.store_root).unwrap();
    assert_eq!(
        canonical_root_a, canonical_root_b,
        "root and submodule discovery must resolve to the same physical store_root"
    );

    let tuples_from_root = discovery_tuples(&canonical_root_a);
    let tuples_from_submodule = discovery_tuples(&canonical_root_b);
    assert_eq!(
        tuples_from_root, tuples_from_submodule,
        "(entity_id, canonical_path, digest) tuples must match between discovery vantage points"
    );
    assert_eq!(tuples_from_root.len(), 2);

    // Roll back both moves and compare checksums before and after.
    let spec_rollback = source_config
        .rollback_move_with_journal(TestRecordKind::Spec, spec_outcome.journal.id)
        .unwrap();
    assert!(spec_rollback.rolled_back);
    let execution_rollback = source_config
        .rollback_move_with_journal(
            TestRecordKind::Execution,
            execution_outcome.journal.id,
        )
        .unwrap();
    assert!(execution_rollback.rolled_back);

    assert_eq!(
        source_config
            .canonical_entity_digest(TestRecordKind::Spec, &spec_id)
            .unwrap(),
        source_spec_digest,
        "rollback must restore the exact spec digest"
    );
    assert_eq!(
        source_config
            .canonical_entity_digest(TestRecordKind::Execution, &execution_id)
            .unwrap(),
        source_execution_digest,
        "rollback must restore the exact execution digest"
    );
    assert!(
        !target_config.canonical_spec_exists("vt-fixture-spec").unwrap(),
        "destination spec entity removed after rollback"
    );
    assert!(
        !target_config
            .canonical_execution_exists("exec-fixture-1")
            .unwrap(),
        "destination execution entity removed after rollback"
    );
}
