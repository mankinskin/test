//! Test-domain adapter onto the domain-neutral move kernel.
//!
//! Canonical entities (see [`crate::canonical`]) live at
//! `<store_root>/<specs|executions>/<uuid>/entity.json`. The
//! kernel's `entity_subdir` is therefore `<specs|executions>`
//! relative to the store root (the `.test` directory), so
//! `target_store_root.join(entity_subdir).join(entity_id)` lands exactly on
//! that canonical folder in the destination store.
//!
//! One [`TestMoveDomain`] is constructed per record kind (spec vs execution);
//! the kind must be supplied again identically on resume/rollback so the
//! kernel recomputes the same `entity_subdir`.

use std::path::{
    Path,
    PathBuf,
};

use memory_kernel::storage::move_kernel::{
    self,
    MoveDomain,
    MoveError,
    MoveOutcome,
    MovePlan,
    MoveReferences,
    MoveResult,
};
use uuid::Uuid;

use crate::{
    TestError,
    TestStoreConfig,
    canonical::{
        ENTITY_FILE,
        TestRecordKind,
        canonical_entity_id,
    },
};

const TEST_STORE_INDEX_DIR: &str = ".test";

fn from_move_error(error: MoveError) -> TestError {
    match error {
        MoveError::Io(io) => TestError::Move(io.to_string()),
        MoveError::Domain(message) => TestError::Move(message),
        MoveError::InteroperabilityContract {
            artifact_class,
            detail,
        } => TestError::Move(format!(
            "interoperability contract violation for {artifact_class}: {detail}"
        )),
    }
}

/// Test-domain implementation of the move kernel's [`MoveDomain`] trait for
/// one record kind (spec or execution).
pub struct TestMoveDomain<'a> {
    config: &'a TestStoreConfig,
    entity_subdir: String,
}

impl<'a> TestMoveDomain<'a> {
    pub fn new(
        config: &'a TestStoreConfig,
        kind: TestRecordKind,
    ) -> Self {
        let entity_subdir = kind.subdir().to_string();
        Self {
            config,
            entity_subdir,
        }
    }

    fn entity_dir(
        &self,
        store_root: &Path,
        entity_id: &Uuid,
    ) -> PathBuf {
        store_root.join(&self.entity_subdir).join(entity_id.to_string())
    }
}

impl MoveDomain for TestMoveDomain<'_> {
    fn entity_subdir(&self) -> &str {
        &self.entity_subdir
    }

    fn store_index_dir(&self) -> &str {
        TEST_STORE_INDEX_DIR
    }

    fn source_store_root(&self) -> PathBuf {
        self.config.root.clone()
    }

    fn source_entity_path(
        &self,
        entity_id: &Uuid,
    ) -> MoveResult<Option<PathBuf>> {
        let dir = self.entity_dir(&self.config.root, entity_id);
        Ok(if dir.join(ENTITY_FILE).is_file() {
            Some(dir)
        } else {
            None
        })
    }

    fn related_entities(
        &self,
        _entity_id: &Uuid,
    ) -> MoveResult<MoveReferences> {
        Ok(MoveReferences::default())
    }

    fn target_store_present(
        &self,
        target_store_root: &Path,
    ) -> MoveResult<bool> {
        Ok(target_store_root.is_dir())
    }

    fn entity_indexed_in(
        &self,
        store_root: &Path,
        entity_id: &Uuid,
    ) -> MoveResult<bool> {
        Ok(self.entity_dir(store_root, entity_id).join(ENTITY_FILE).is_file())
    }

    fn scan_store(
        &self,
        _store_root: &Path,
    ) -> MoveResult<()> {
        Ok(())
    }
}

impl TestStoreConfig {
    /// Build a read-only preflight plan for moving `id_or_alias` (a legacy
    /// alias or canonical UUID) of the given record kind to
    /// `target_workspace_root`.
    pub fn plan_move_preflight(
        &self,
        kind: TestRecordKind,
        id_or_alias: &str,
        target_workspace_root: &Path,
    ) -> Result<MovePlan, TestError> {
        let canonical_id =
            canonical_entity_id(&self.workspace_path(), kind, id_or_alias);
        let domain = TestMoveDomain::new(self, kind);
        move_kernel::plan_move(&domain, &canonical_id, target_workspace_root)
            .map_err(from_move_error)
    }

    /// Execute a supported test-record move with a fresh journal.
    pub fn execute_move_with_journal(
        &self,
        kind: TestRecordKind,
        plan: &MovePlan,
    ) -> Result<MoveOutcome, TestError> {
        let domain = TestMoveDomain::new(self, kind);
        move_kernel::execute_move(&domain, plan).map_err(from_move_error)
    }

    /// Resume an interrupted test-record move from its journal id. `kind`
    /// must match the kind used when the move was planned.
    pub fn resume_move_with_journal(
        &self,
        kind: TestRecordKind,
        journal_id: Uuid,
    ) -> Result<MoveOutcome, TestError> {
        let domain = TestMoveDomain::new(self, kind);
        move_kernel::resume_move(&domain, journal_id).map_err(from_move_error)
    }

    /// Roll back a test-record move from its journal id. `kind` must match
    /// the kind used when the move was planned.
    pub fn rollback_move_with_journal(
        &self,
        kind: TestRecordKind,
        journal_id: Uuid,
    ) -> Result<MoveOutcome, TestError> {
        let domain = TestMoveDomain::new(self, kind);
        move_kernel::rollback_move(&domain, journal_id).map_err(from_move_error)
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use chrono::Utc;
    use tempfile::tempdir;

    use super::*;
    use crate::{
        ValidationExecution,
        ValidationOutcome,
        ValidationSpec,
    };

    fn run_git(
        repo_root: &Path,
        args: &[&str],
    ) {
        let status = Command::new("git")
            .current_dir(repo_root)
            .args(args)
            .status()
            .expect("git command");
        assert!(status.success(), "git {args:?} failed: {status}");
    }

    fn init_repo() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let temp = tempdir().unwrap();
        let repo = temp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        run_git(&repo, &["init"]);

        let source_workspace = repo.join("source");
        let target_workspace = repo.join("target");
        std::fs::create_dir_all(&source_workspace).unwrap();
        std::fs::create_dir_all(
            target_workspace.join(".workflow-tools").join("test"),
        )
            .unwrap();
        (temp, source_workspace, target_workspace)
    }

    fn seeded_source_config(source_workspace: &Path) -> TestStoreConfig {
        let config = TestStoreConfig::new(source_workspace.join(TEST_STORE_INDEX_DIR));
        config
            .record_spec(&ValidationSpec::new("vt-core", "Core tests"))
            .unwrap();
        config
            .record_execution(&ValidationExecution {
                id: "exec-core-1".to_string(),
                validation_spec_id: "vt-core".to_string(),
                outcome: ValidationOutcome::Passed,
                executed_at: Utc::now(),
                duration_ms: Some(120),
                throughput: None,
                detail: None,
                links: crate::ValidationLinks {
                    ticket_ids: vec!["ticket-1".to_string()],
                    ..Default::default()
                },
                provenance: crate::ValidationProvenance {
                    domain: Some("test".to_string()),
                    operation: Some("move".to_string()),
                    run_id: Some("run-1".to_string()),
                    ..Default::default()
                },
            })
            .unwrap();
        config.migrate_to_canonical().unwrap();
        config
    }

    #[test]
    fn deterministic_uuid_mapping_is_stable_and_workspace_scoped() {
        let workspace = Path::new("workspace-a");
        let a = canonical_entity_id(workspace, TestRecordKind::Spec, "vt-core");
        let b = canonical_entity_id(workspace, TestRecordKind::Spec, "vt-core");
        assert_eq!(a, b);

        let other_workspace =
            canonical_entity_id(Path::new("workspace-b"), TestRecordKind::Spec, "vt-core");
        assert_ne!(a, other_workspace);

        let other_kind =
            canonical_entity_id(workspace, TestRecordKind::Execution, "vt-core");
        assert_ne!(a, other_kind);

        let already_uuid = Uuid::new_v4();
        assert_eq!(
            canonical_entity_id(
                workspace,
                TestRecordKind::Spec,
                &already_uuid.to_string()
            ),
            already_uuid
        );
    }

    #[test]
    fn spec_move_preflight_apply_and_rollback_round_trip() {
        let (_temp, source_workspace, target_workspace) = init_repo();
        let config = seeded_source_config(&source_workspace);

        let plan = config
            .plan_move_preflight(TestRecordKind::Spec, "vt-core", &target_workspace)
            .unwrap();
        assert!(plan.supported(), "blockers: {:?}", plan.blockers);

        let canonical_id = config.resolve_canonical_spec_id("vt-core");
        let source_digest = config
            .canonical_entity_digest(
                TestRecordKind::Spec,
            &canonical_id,
            )
            .unwrap();

        let outcome = config
            .execute_move_with_journal(TestRecordKind::Spec, &plan)
            .unwrap();
        assert!(!outcome.resumed);
        assert!(!outcome.rolled_back);

        // Source entity gone, destination entity present with matching digest.
        assert!(!config.canonical_spec_exists("vt-core").unwrap());
        let target_config = TestStoreConfig::for_workspace(&target_workspace);
        assert!(target_config.canonical_spec_exists("vt-core").unwrap());
        let target_digest = target_config
            .canonical_entity_digest(
                TestRecordKind::Spec,
                &canonical_id,
            )
            .unwrap();
        assert_eq!(source_digest, target_digest);

        // Rollback restores the source and removes the destination.
        let rollback_outcome = config
            .rollback_move_with_journal(
                TestRecordKind::Spec,
                outcome.journal.id,
            )
            .unwrap();
        assert!(rollback_outcome.rolled_back);
        assert!(config.canonical_spec_exists("vt-core").unwrap());
        assert!(!target_config.canonical_spec_exists("vt-core").unwrap());
    }

    #[test]
    fn execution_move_preflight_apply_and_resume_round_trip() {
        let (_temp, source_workspace, target_workspace) = init_repo();
        let config = seeded_source_config(&source_workspace);

        let mut plan = config
            .plan_move_preflight(
                TestRecordKind::Execution,
                "exec-core-1",
                &target_workspace,
            )
            .unwrap();
        assert!(plan.supported(), "blockers: {:?}", plan.blockers);

        let outcome = config
            .execute_move_with_journal(TestRecordKind::Execution, &plan)
            .unwrap();
        assert!(!outcome.resumed);

        // Resuming an already-completed journal is a no-op success.
        let resumed = config
            .resume_move_with_journal(
                TestRecordKind::Execution,
                outcome.journal.id,
            )
            .unwrap();
        assert!(resumed.resumed);

        let target_config = TestStoreConfig::for_workspace(&target_workspace);
        assert!(target_config.canonical_execution_exists("exec-core-1").unwrap());

        // Re-planning the same (now moved) entity against a fresh target
        // reports it missing at the source.
        let temp2 = tempdir().unwrap();
        let other_target = temp2.path().join("other-target");
        std::fs::create_dir_all(other_target.join(TEST_STORE_INDEX_DIR)).unwrap();
        plan = config
            .plan_move_preflight(
                TestRecordKind::Execution,
                "exec-core-1",
                &other_target,
            )
            .unwrap();
        assert!(!plan.supported());
    }

    #[test]
    fn preflight_blocks_on_missing_source_entity() {
        let (_temp, source_workspace, target_workspace) = init_repo();
        let config = TestStoreConfig::new(source_workspace.join(TEST_STORE_INDEX_DIR));

        let plan = config
            .plan_move_preflight(TestRecordKind::Spec, "never-migrated", &target_workspace)
            .unwrap();
        assert!(!plan.supported());
    }

    #[test]
    fn preflight_blocks_on_missing_target_store() {
        let (_temp, source_workspace, _target_workspace) = init_repo();
        let config = seeded_source_config(&source_workspace);

        let missing_target = source_workspace.join("does-not-exist");
        let plan = config
            .plan_move_preflight(TestRecordKind::Spec, "vt-core", &missing_target)
            .unwrap();
        assert!(!plan.supported());
    }
}
