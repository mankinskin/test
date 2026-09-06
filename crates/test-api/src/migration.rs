//! Resumable migration of legacy validation specs/executions into canonical
//! UUID entity folders (see [`crate::canonical`]).
//!
//! Migration proceeds in two durable phases recorded in a manifest at
//! `<store-root>/migration.manifest.json`:
//!
//! 1. `Staged` — every legacy record and every execution-to-spec relationship
//!    is validated, then canonical entities are written into a staging
//!    directory. Nothing under the live `specs/`/`executions/` directories is
//!    touched yet.
//! 2. `Published` — the staged canonical folders are moved into place
//!    atomically (per-entity `rename`) and the manifest is updated.
//!
//! An interrupted migration is resumed from its last durable phase via
//! [`TestStoreConfig::resume_migration`], and can be undone via
//! [`TestStoreConfig::rollback_migration`], which removes only the canonical
//! folders it created — legacy JSON is never deleted by migration.

use std::{
    collections::BTreeMap,
    fs,
    io::ErrorKind,
    path::PathBuf,
};

use chrono::{
    DateTime,
    Utc,
};
use serde::{
    Deserialize,
    Serialize,
};
use uuid::Uuid;

use crate::{
    ExecutionQuery,
    TestError,
    TestStoreConfig,
    canonical::{
        CanonicalExecutionEntity,
        CanonicalSpecEntity,
        TestRecordKind,
        any_canonical_entity_present,
        canonical_entity_id,
        write_canonical_entity,
    },
    store::read_json_if_exists,
};

const MIGRATION_MANIFEST_FILE: &str = "migration.manifest.json";
const MIGRATION_STAGING_DIR: &str = "migration-staging";

/// Durable phase of a canonicalization migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MigrationPhase {
    /// Canonical entities validated and written to staging; not yet
    /// published.
    Staged,
    /// Canonical entities published into `specs/`/`executions/`.
    Published,
}

/// Durable record of a canonicalization migration run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationManifest {
    pub phase: MigrationPhase,
    pub workspace_path: PathBuf,
    /// Legacy spec id -> canonical UUID.
    pub migrated_specs: BTreeMap<String, Uuid>,
    /// Legacy execution id -> canonical UUID.
    pub migrated_executions: BTreeMap<String, Uuid>,
    pub started_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

impl TestStoreConfig {
    fn migration_manifest_path(&self) -> Result<PathBuf, TestError> {
        Ok(self.workspace_dir()?.join(MIGRATION_MANIFEST_FILE))
    }

    fn migration_staging_dir(&self) -> Result<PathBuf, TestError> {
        Ok(self.workspace_dir()?.join(MIGRATION_STAGING_DIR))
    }

    /// Read the migration manifest, if one has been written.
    pub fn read_migration_manifest(
        &self
    ) -> Result<Option<MigrationManifest>, TestError> {
        read_json_if_exists(&self.migration_manifest_path()?)
    }

    fn write_migration_manifest(
        &self,
        manifest: &MigrationManifest,
    ) -> Result<(), TestError> {
        crate::store::write_json(&self.migration_manifest_path()?, manifest)
    }

    fn legacy_workspace_dir(&self) -> Result<Option<PathBuf>, TestError> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(TestError::Io {
                path: self.root.clone(),
                source,
            }),
        };

        let mut candidates = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_dir()
                    && (path.join(TestRecordKind::Spec.subdir()).is_dir()
                        || path.join(TestRecordKind::Execution.subdir()).is_dir())
            })
            .collect::<Vec<_>>();
        candidates.sort();

        match candidates.len() {
            0 => Ok(None),
            1 => Ok(candidates.pop()),
            _ => Err(TestError::Migration(format!(
                "multiple legacy workspace directories found under {}; migrate each legacy workspace separately before cutover",
                self.root.display()
            ))),
        }
    }

    /// Validate every legacy record and execution-to-spec relationship, then
    /// write canonical entities into the staging directory and persist a
    /// `Staged` manifest.
    ///
    /// Fails closed if canonical entity folders already exist without a
    /// migration manifest (an unowned mixed layout).
    fn stage_migration(&self) -> Result<MigrationManifest, TestError> {
        if any_canonical_entity_present(&self.specs_dir()?)?
            || any_canonical_entity_present(&self.executions_dir()?)?
        {
            return Err(TestError::Migration(
                "canonical entity folders already present without a \
                 migration manifest; unowned mixed layout"
                    .to_string(),
            ));
        }

        let (specs, executions) = match self.legacy_workspace_dir()? {
            Some(legacy_workspace) => (
                self.read_dir_json(&legacy_workspace.join(TestRecordKind::Spec.subdir()))?,
                self.read_dir_json(&legacy_workspace.join(TestRecordKind::Execution.subdir()))?,
            ),
            None => (
                self.list_specs()?,
                self.list_executions(&ExecutionQuery::default())?,
            ),
        };

        let spec_ids: std::collections::BTreeSet<&str> =
            specs.iter().map(|spec| spec.id.as_str()).collect();
        for execution in &executions {
            if !spec_ids.contains(execution.validation_spec_id.as_str()) {
                return Err(TestError::Migration(format!(
                    "execution `{}` references missing validation spec `{}`",
                    execution.id, execution.validation_spec_id
                )));
            }
        }

        let migrated_specs: BTreeMap<String, Uuid> = specs
            .iter()
            .map(|spec| {
                (
                    spec.id.clone(),
                    canonical_entity_id(
                        &self.workspace_path(),
                        TestRecordKind::Spec,
                        &spec.id,
                    ),
                )
            })
            .collect();
        let migrated_executions: BTreeMap<String, Uuid> = executions
            .iter()
            .map(|execution| {
                (
                    execution.id.clone(),
                    canonical_entity_id(
                        &self.workspace_path(),
                        TestRecordKind::Execution,
                        &execution.id,
                    ),
                )
            })
            .collect();

        let staging_root = self.migration_staging_dir()?;
        let _ = fs::remove_dir_all(&staging_root);

        for spec in &specs {
            let canonical_id = migrated_specs[&spec.id];
            let entity = CanonicalSpecEntity {
                canonical_id,
                legacy_id: spec.id.clone(),
                spec: spec.clone(),
            };
            let dir = staging_root
                .join(TestRecordKind::Spec.subdir())
                .join(canonical_id.to_string());
            fs::create_dir_all(&dir).map_err(|source| TestError::Io {
                path: dir.clone(),
                source,
            })?;
            write_canonical_entity(&dir, &entity)?;
        }
        for execution in &executions {
            let canonical_id = migrated_executions[&execution.id];
            let validation_spec_canonical_id =
                migrated_specs.get(&execution.validation_spec_id).copied();
            let entity = CanonicalExecutionEntity {
                canonical_id,
                legacy_id: execution.id.clone(),
                validation_spec_canonical_id,
                execution: execution.clone(),
            };
            let dir = staging_root
                .join(TestRecordKind::Execution.subdir())
                .join(canonical_id.to_string());
            fs::create_dir_all(&dir).map_err(|source| TestError::Io {
                path: dir.clone(),
                source,
            })?;
            write_canonical_entity(&dir, &entity)?;
        }

        let manifest = MigrationManifest {
            phase: MigrationPhase::Staged,
            workspace_path: self.workspace_path(),
            migrated_specs,
            migrated_executions,
            started_at: Utc::now(),
            completed_at: None,
        };
        self.write_migration_manifest(&manifest)?;
        Ok(manifest)
    }

    /// Move every staged canonical entity folder into place and persist a
    /// `Published` manifest. Idempotent: republishing an already-published
    /// manifest is a no-op beyond refreshing `completed_at`.
    fn publish_staged_migration(
        &self,
        manifest: &MigrationManifest,
    ) -> Result<MigrationManifest, TestError> {
        let staging_root = self.migration_staging_dir()?;
        for kind in [TestRecordKind::Spec, TestRecordKind::Execution] {
            let staged_subdir = staging_root.join(kind.subdir());
            if !staged_subdir.is_dir() {
                continue;
            }
            let dest_subdir = self.workspace_dir()?.join(kind.subdir());
            fs::create_dir_all(&dest_subdir).map_err(|source| {
                TestError::Io {
                    path: dest_subdir.clone(),
                    source,
                }
            })?;
            let entries = fs::read_dir(&staged_subdir).map_err(|source| {
                TestError::Io {
                    path: staged_subdir.clone(),
                    source,
                }
            })?;
            for entry in entries {
                let entry = entry.map_err(|source| TestError::Io {
                    path: staged_subdir.clone(),
                    source,
                })?;
                let dest = dest_subdir.join(entry.file_name());
                if dest.exists() {
                    fs::remove_dir_all(&dest).map_err(|source| {
                        TestError::Io {
                            path: dest.clone(),
                            source,
                        }
                    })?;
                }
                fs::rename(entry.path(), &dest).map_err(|source| {
                    TestError::Io {
                        path: dest.clone(),
                        source,
                    }
                })?;
            }
        }
        let _ = fs::remove_dir_all(&staging_root);

        let published = MigrationManifest {
            phase: MigrationPhase::Published,
            completed_at: Some(Utc::now()),
            ..manifest.clone()
        };
        self.write_migration_manifest(&published)?;
        Ok(published)
    }

    /// Validate, stage, and publish the canonicalization migration.
    ///
    /// Idempotent: a fully published migration returns immediately without
    /// re-validating; an interrupted (staged) migration resumes and
    /// publishes from staging.
    pub fn migrate_to_canonical(&self) -> Result<MigrationManifest, TestError> {
        match self.read_migration_manifest()? {
            Some(manifest) if manifest.phase == MigrationPhase::Published =>
                Ok(manifest),
            Some(manifest) => self.publish_staged_migration(&manifest),
            None => {
                let manifest = self.stage_migration()?;
                self.publish_staged_migration(&manifest)
            },
        }
    }

    /// Resume an interrupted migration from its durable manifest.
    ///
    /// Fails if no manifest exists (nothing to resume).
    pub fn resume_migration(&self) -> Result<MigrationManifest, TestError> {
        match self.read_migration_manifest()? {
            Some(manifest) if manifest.phase == MigrationPhase::Published =>
                Ok(manifest),
            Some(manifest) => self.publish_staged_migration(&manifest),
            None => Err(TestError::Migration(
                "no migration manifest to resume".to_string(),
            )),
        }
    }

    /// Roll back a migration: remove every canonical entity folder and
    /// staging directory it created, plus the manifest itself. Legacy JSON
    /// files are never touched.
    pub fn rollback_migration(&self) -> Result<(), TestError> {
        let manifest = self.read_migration_manifest()?.ok_or_else(|| {
            TestError::Migration("no migration manifest to roll back".to_string())
        })?;

        for canonical_id in manifest.migrated_specs.values() {
            let dir = self.specs_dir()?.join(canonical_id.to_string());
            let _ = fs::remove_dir_all(&dir);
        }
        for canonical_id in manifest.migrated_executions.values() {
            let dir = self.executions_dir()?.join(canonical_id.to_string());
            let _ = fs::remove_dir_all(&dir);
        }
        let _ = fs::remove_dir_all(&self.migration_staging_dir()?);

        let manifest_path = self.migration_manifest_path()?;
        match fs::remove_file(&manifest_path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(source) => Err(TestError::Io {
                path: manifest_path,
                source,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::{
        ValidationExecution,
        ValidationOutcome,
        ValidationSpec,
    };

    fn config(dir: &TempDir) -> TestStoreConfig {
        TestStoreConfig::new(dir.path().join(".test"))
    }

    fn seed_legacy_records(config: &TestStoreConfig) {
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
                    operation: Some("migrate".to_string()),
                    run_id: Some("run-1".to_string()),
                    ..Default::default()
                },
            })
            .unwrap();
    }

    #[test]
    fn migrate_to_canonical_is_idempotent_and_resolves_aliases() {
        let dir = TempDir::new().unwrap();
        let config = config(&dir);
        seed_legacy_records(&config);

        let manifest = config.migrate_to_canonical().unwrap();
        assert_eq!(manifest.phase, MigrationPhase::Published);
        assert_eq!(manifest.migrated_specs.len(), 1);
        assert_eq!(manifest.migrated_executions.len(), 1);

        let spec_entity = config.get_canonical_spec("vt-core").unwrap();
        assert_eq!(spec_entity.legacy_id, "vt-core");
        let execution_entity =
            config.get_canonical_execution("exec-core-1").unwrap();
        assert_eq!(
            execution_entity.validation_spec_canonical_id,
            Some(spec_entity.canonical_id)
        );

        // Same UUID reachable directly.
        let by_uuid = config
            .get_canonical_spec(&spec_entity.canonical_id.to_string())
            .unwrap();
        assert_eq!(by_uuid, spec_entity);

        // Idempotent republish.
        let second = config.migrate_to_canonical().unwrap();
        assert_eq!(second.migrated_specs, manifest.migrated_specs);
        assert_eq!(second.phase, MigrationPhase::Published);

        // Legacy JSON retained as read-only backup.
        assert!(config.get_spec("vt-core").is_ok());
        assert!(config.get_execution("exec-core-1").is_ok());
    }

    #[test]
    fn resume_migration_publishes_from_staged_phase() {
        let dir = TempDir::new().unwrap();
        let config = config(&dir);
        seed_legacy_records(&config);

        let staged = config.stage_migration().unwrap();
        assert_eq!(staged.phase, MigrationPhase::Staged);
        // Not yet published: canonical entities not visible in the live dirs.
        assert!(!config.canonical_spec_exists("vt-core").unwrap());

        let resumed = config.resume_migration().unwrap();
        assert_eq!(resumed.phase, MigrationPhase::Published);
        assert!(config.canonical_spec_exists("vt-core").unwrap());
    }

    #[test]
    fn resume_migration_without_manifest_errors() {
        let dir = TempDir::new().unwrap();
        let config = config(&dir);
        seed_legacy_records(&config);
        let err = config.resume_migration().unwrap_err();
        assert!(matches!(err, TestError::Migration(_)));
    }

    #[test]
    fn rollback_migration_removes_canonical_entities_but_keeps_legacy_json() {
        let dir = TempDir::new().unwrap();
        let config = config(&dir);
        seed_legacy_records(&config);
        config.migrate_to_canonical().unwrap();

        config.rollback_migration().unwrap();

        assert!(!config.canonical_spec_exists("vt-core").unwrap());
        assert!(!config.canonical_execution_exists("exec-core-1").unwrap());
        assert!(config.read_migration_manifest().unwrap().is_none());
        assert!(config.get_spec("vt-core").is_ok());
        assert!(config.get_execution("exec-core-1").is_ok());
    }

    #[test]
    fn migration_rejects_execution_referencing_missing_spec() {
        let dir = TempDir::new().unwrap();
        let config = config(&dir);
        config
            .record_execution(&ValidationExecution {
                id: "exec-orphan".to_string(),
                validation_spec_id: "missing-spec".to_string(),
                outcome: ValidationOutcome::Passed,
                executed_at: Utc::now(),
                duration_ms: None,
                throughput: None,
                detail: None,
                links: crate::ValidationLinks {
                    ticket_ids: vec!["ticket-1".to_string()],
                    ..Default::default()
                },
                provenance: crate::ValidationProvenance {
                    domain: Some("test".to_string()),
                    operation: Some("migrate".to_string()),
                    run_id: Some("run-orphan".to_string()),
                    ..Default::default()
                },
            })
            .unwrap();

        let err = config.migrate_to_canonical().unwrap_err();
        assert!(matches!(err, TestError::Migration(_)));
    }

    #[test]
    fn migration_fails_closed_on_unowned_mixed_layout() {
        let dir = TempDir::new().unwrap();
        let config = config(&dir);
        seed_legacy_records(&config);

        // Simulate an unowned canonical folder with no manifest.
        let stray_id = Uuid::new_v4();
        let dir_path = config.specs_dir().unwrap().join(stray_id.to_string());
        fs::create_dir_all(&dir_path).unwrap();
        write_canonical_entity(
            &dir_path,
            &CanonicalSpecEntity {
                canonical_id: stray_id,
                legacy_id: "stray".to_string(),
                spec: ValidationSpec::new("stray", "Stray"),
            },
        )
        .unwrap();

        let err = config.migrate_to_canonical().unwrap_err();
        assert!(matches!(err, TestError::Migration(_)));
    }
}
