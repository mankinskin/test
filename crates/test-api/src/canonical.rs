//! Canonical UUID-keyed entity folders for validation specs and executions.
//!
//! Legacy validation specs/executions are persisted as flat `<id>.json` files
//! keyed by an arbitrary path-safe string id (see [`crate::store`]). This
//! module adds a domain-local, [`memory_kernel::storage::move_kernel::MoveDomain`]-compatible
//! canonical representation: each record also gets a folder
//! `<specs|executions>/<uuid>/entity.json` keyed by a
//! deterministic UUID, with the legacy id preserved as an immutable alias.
//!
//! A legacy identifier that already parses as a UUID keeps that UUID as its
//! canonical id. Every other legacy identifier maps to a UUIDv5 derived from a
//! fixed test-domain namespace plus the canonical workspace path, record kind, and
//! legacy identifier, so the mapping is stable across process runs.

use std::{
    fs,
    io::ErrorKind,
    path::{
        Path,
        PathBuf,
    },
};

use serde::{
    Deserialize,
    Serialize,
    de::DeserializeOwned,
};
use sha2::{
    Digest,
    Sha256,
};
use uuid::Uuid;

use crate::{
    TestError,
    TestStoreConfig,
    ValidationExecution,
    ValidationSpec,
    store::{
        read_json_if_exists,
        write_json,
    },
};

/// Fixed UUIDv5 namespace for deriving canonical test-entity identifiers.
/// Frozen once chosen; never regenerate, or previously derived canonical ids
/// would no longer match freshly computed ones.
pub const TEST_DOMAIN_NAMESPACE: Uuid = Uuid::from_bytes([
    0x8e, 0x2a, 0x4f, 0x61, 0x3d, 0x9c, 0x4b, 0x2e, 0x9a, 0x71, 0x5c, 0x0e,
    0x4d, 0x6b, 0x2f, 0x33,
]);

/// The file holding a canonical entity's payload inside its UUID folder.
pub(crate) const ENTITY_FILE: &str = "entity.json";

/// The two record kinds canonicalized by the test domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TestRecordKind {
    Spec,
    Execution,
}

impl TestRecordKind {
    /// Subdirectory under the workspace directory holding this kind's legacy
    /// JSON files and canonical UUID folders (they coexist: legacy files end
    /// in `.json`, canonical entities are directories named after their
    /// UUID).
    pub fn subdir(&self) -> &'static str {
        match self {
            TestRecordKind::Spec => "specs",
            TestRecordKind::Execution => "executions",
        }
    }

    fn namespace_tag(&self) -> &'static str {
        match self {
            TestRecordKind::Spec => "spec",
            TestRecordKind::Execution => "execution",
        }
    }
}

/// Deterministically map a legacy identifier to its canonical UUID.
///
/// A legacy identifier that already parses as a UUID keeps that UUID.
/// Otherwise the UUID is derived via UUIDv5 from [`TEST_DOMAIN_NAMESPACE`]
/// plus `<canonical-workspace-path>:<kind>:<legacy_id>`.
pub fn canonical_entity_id(
    workspace_path: &Path,
    kind: TestRecordKind,
    legacy_id: &str,
) -> Uuid {
    if let Ok(existing) = Uuid::parse_str(legacy_id) {
        return existing;
    }
    let workspace_path = memory_kernel::workspace::canonicalize_workspace_root(workspace_path);
    let name = format!(
        "{}:{}:{legacy_id}",
        workspace_path.to_string_lossy(),
        kind.namespace_tag()
    );
    Uuid::new_v5(&TEST_DOMAIN_NAMESPACE, name.as_bytes())
}

/// A canonical validation-spec entity: the deterministic UUID, the preserved
/// legacy alias, and the original spec payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalSpecEntity {
    pub canonical_id: Uuid,
    pub legacy_id: String,
    pub spec: ValidationSpec,
}

/// A canonical validation-execution entity: the deterministic UUID, the
/// preserved legacy alias, the resolved canonical id of the spec it links to
/// (when that spec was present in the canonical index at migration time), and
/// the original execution payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalExecutionEntity {
    pub canonical_id: Uuid,
    pub legacy_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_spec_canonical_id: Option<Uuid>,
    pub execution: ValidationExecution,
}

pub(crate) fn write_canonical_entity<T: Serialize>(
    dir: &Path,
    value: &T,
) -> Result<PathBuf, TestError> {
    let path = dir.join(ENTITY_FILE);
    write_json(&path, value)?;
    Ok(path)
}

pub(crate) fn read_canonical_entity<T: DeserializeOwned>(
    dir: &Path
) -> Result<Option<T>, TestError> {
    read_json_if_exists(&dir.join(ENTITY_FILE))
}

/// Whether a canonical entity folder exists for `canonical_id` under `dir`
/// (the kind's subdir).
fn canonical_entity_present(
    dir: &Path,
    canonical_id: &Uuid,
) -> bool {
    dir.join(canonical_id.to_string()).join(ENTITY_FILE).is_file()
}

fn find_canonical_entity_by_alias<T: DeserializeOwned>(
    dir: &Path,
    alias: &str,
    entity_alias: impl Fn(&T) -> &str,
) -> Result<Option<T>, TestError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(TestError::Io {
            path: dir.to_path_buf(),
            source,
        }),
    };

    for entry in entries {
        let entry = entry.map_err(|source| TestError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let Some(entity) = read_canonical_entity::<T>(&entry.path())? else {
            continue;
        };
        if entity_alias(&entity) == alias {
            return Ok(Some(entity));
        }
    }
    Ok(None)
}

/// Whether `dir` (a kind's subdir) contains any canonical UUID-named entity
/// folder, regardless of whether it is tracked by a migration manifest.
pub(crate) fn any_canonical_entity_present(dir: &Path) -> Result<bool, TestError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(false),
        Err(source) =>
            return Err(TestError::Io {
                path: dir.to_path_buf(),
                source,
            }),
    };
    for entry in entries {
        let entry = entry.map_err(|source| TestError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() && path.join(ENTITY_FILE).is_file() {
            return Ok(true);
        }
    }
    Ok(false)
}

impl TestStoreConfig {
    /// Deterministically resolve a legacy alias or canonical UUID string to
    /// its canonical spec UUID (no existence check).
    pub fn resolve_canonical_spec_id(
        &self,
        id_or_alias: &str,
    ) -> Uuid {
        canonical_entity_id(&self.workspace_path(), TestRecordKind::Spec, id_or_alias)
    }

    /// Deterministically resolve a legacy alias or canonical UUID string to
    /// its canonical execution UUID (no existence check).
    pub fn resolve_canonical_execution_id(
        &self,
        id_or_alias: &str,
    ) -> Uuid {
        canonical_entity_id(
            &self.workspace_path(),
            TestRecordKind::Execution,
            id_or_alias,
        )
    }

    /// Read a canonical spec entity by legacy alias or canonical UUID string.
    ///
    /// A legacy alias is scanned when the path-derived UUID does not exist,
    /// allowing an entity to retain its canonical UUID after a workspace move.
    pub fn get_canonical_spec(
        &self,
        id_or_alias: &str,
    ) -> Result<CanonicalSpecEntity, TestError> {
        let canonical_id = self.resolve_canonical_spec_id(id_or_alias);
        let dir = self.specs_dir()?.join(canonical_id.to_string());
        match read_canonical_entity(&dir)? {
            Some(entity) => Ok(entity),
            None => find_canonical_entity_by_alias(
                &self.specs_dir()?,
                id_or_alias,
                |entity: &CanonicalSpecEntity| entity.legacy_id.as_str(),
            )?
            .ok_or_else(|| TestError::SpecNotFound(id_or_alias.to_string())),
        }
    }

    /// Read a canonical execution entity by legacy alias or canonical UUID
    /// string.
    pub fn get_canonical_execution(
        &self,
        id_or_alias: &str,
    ) -> Result<CanonicalExecutionEntity, TestError> {
        let canonical_id = self.resolve_canonical_execution_id(id_or_alias);
        let dir = self.executions_dir()?.join(canonical_id.to_string());
        match read_canonical_entity(&dir)? {
            Some(entity) => Ok(entity),
            None => find_canonical_entity_by_alias(
                &self.executions_dir()?,
                id_or_alias,
                |entity: &CanonicalExecutionEntity| entity.legacy_id.as_str(),
            )?
            .ok_or_else(|| TestError::ExecutionNotFound(id_or_alias.to_string())),
        }
    }

    /// Whether a canonical spec entity exists for `id_or_alias`.
    pub fn canonical_spec_exists(
        &self,
        id_or_alias: &str,
    ) -> Result<bool, TestError> {
        Ok(self.get_canonical_spec(id_or_alias).is_ok())
    }

    /// Whether a canonical execution entity exists for `id_or_alias`.
    pub fn canonical_execution_exists(
        &self,
        id_or_alias: &str,
    ) -> Result<bool, TestError> {
        Ok(self.get_canonical_execution(id_or_alias).is_ok())
    }

    /// SHA-256 digest (hex) of a canonical entity's serialized payload.
    pub fn canonical_entity_digest(
        &self,
        kind: TestRecordKind,
        canonical_id: &Uuid,
    ) -> Result<String, TestError> {
        let dir = match kind {
            TestRecordKind::Spec => self.specs_dir()?,
            TestRecordKind::Execution => self.executions_dir()?,
        }
        .join(canonical_id.to_string());
        let path = dir.join(ENTITY_FILE);
        let bytes = fs::read(&path).map_err(|source| TestError::Io {
            path: path.clone(),
            source,
        })?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        Ok(format!("{:x}", hasher.finalize()))
    }
}
