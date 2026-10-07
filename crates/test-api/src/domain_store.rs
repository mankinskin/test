use std::path::{Path, PathBuf};

use memory_kernel::{
    domain_store::{
        CreateEntity, DomainStore, DomainStoreResolution, ListEntities, ReadEntity, StoreAccessMode,
    },
    model::domain::DomainId,
    workspace::StoreRootDiagnostic,
};

use crate::{ExecutionQuery, TestError, TestStoreConfig, ValidationExecution, ValidationSpec};

/// A Test-domain entity retained in its existing domain-specific form.
#[derive(Debug, Clone, PartialEq)]
pub enum TestEntity {
    Spec(ValidationSpec),
    Execution(ValidationExecution),
}

/// A typed identifier for one supported Test-domain entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestEntityId {
    Spec(String),
    Execution(String),
}

/// A typed Test-domain create request.
#[derive(Debug, Clone, PartialEq)]
pub enum TestCreateInput {
    Spec(ValidationSpec),
    Execution(ValidationExecution),
}

/// The supported Test-domain list operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestListQuery {
    Specs,
    Executions(ExecutionQuery),
}

/// Typed capabilities over the existing Test spec and execution persistence.
///
/// Test keeps its two persisted record models rather than forcing them into a
/// shared entity schema. Benchmarks and store-index generation remain
/// Test-specific extensions outside the generic capability surface.
#[derive(Debug, Clone)]
pub struct TestDomainStore {
    config: TestStoreConfig,
}

impl TestDomainStore {
    pub fn new(config: TestStoreConfig) -> Self {
        Self { config }
    }
}

impl DomainStore for TestStoreConfig {
    fn domain_id() -> DomainId {
        DomainId::new("test").expect("test is a valid domain id")
    }

    fn store_dir_name() -> &'static str {
        ".test"
    }
}

impl TestStoreConfig {
    /// Resolve the Test store for one selected workspace through the shared
    /// DomainStore contract.
    ///
    /// Read-only access preserves legacy `.test` compatibility. Writes always
    /// use the canonical `.workflow-tools/test` store.
    pub fn resolve_workspace_store(
        local_workspace: &Path,
        access_mode: StoreAccessMode,
    ) -> Result<DomainStoreResolution, TestError> {
        if access_mode == StoreAccessMode::CreateOrOpen {
            memory_kernel::workspace::validate_explicit_store_root_for_write(
                local_workspace,
                ".test",
            )?;
        }
        let mut resolution = <Self as DomainStore>::resolve_store(local_workspace, access_mode)?;
        let legacy_root = resolution.local_workspace.join(".test");
        let canonical_root =
            memory_kernel::workspace::canonical_store_root(&resolution.local_workspace, ".test");
        let legacy_exists = legacy_root.is_dir();
        let canonical_exists = canonical_root.is_dir();

        if access_mode == StoreAccessMode::ReadOnly && legacy_exists && !canonical_exists {
            resolution.store_root = legacy_root.clone();
        }
        if legacy_exists {
            let diagnostic = if canonical_exists {
                StoreRootDiagnostic::BothLayoutsPresent {
                    domain: "test".to_string(),
                    legacy_path: legacy_root,
                    canonical_path: canonical_root,
                }
            } else {
                StoreRootDiagnostic::LegacyStore {
                    domain: "test".to_string(),
                    legacy_path: legacy_root,
                    canonical_path: canonical_root,
                }
            };
            if !resolution.diagnostics.contains(&diagnostic) {
                resolution.diagnostics.push(diagnostic);
            }
        }
        Ok(resolution)
    }
}

impl CreateEntity for TestDomainStore {
    type Entity = TestEntity;
    type CreateInput = TestCreateInput;
    type CreateResult = PathBuf;
    type Error = TestError;

    fn create_entity(&self, input: Self::CreateInput) -> Result<Self::CreateResult, Self::Error> {
        match input {
            TestCreateInput::Spec(spec) => self.config.record_spec(&spec),
            TestCreateInput::Execution(execution) => self.config.record_execution(&execution),
        }
    }
}

impl ReadEntity for TestDomainStore {
    type EntityId = TestEntityId;
    type Entity = TestEntity;
    type ReadResult = TestEntity;
    type Error = TestError;

    fn read_entity(&self, id: Self::EntityId) -> Result<Self::ReadResult, Self::Error> {
        match id {
            TestEntityId::Spec(id) => self.config.get_spec(&id).map(TestEntity::Spec),
            TestEntityId::Execution(id) => {
                self.config.get_execution(&id).map(TestEntity::Execution)
            }
        }
    }
}

impl ListEntities for TestDomainStore {
    type Query = TestListQuery;
    type Entity = TestEntity;
    type ListResult = Vec<TestEntity>;
    type Error = TestError;

    fn list_entities(&self, query: Self::Query) -> Result<Self::ListResult, Self::Error> {
        match query {
            TestListQuery::Specs => self
                .config
                .list_specs()
                .map(|specs| specs.into_iter().map(TestEntity::Spec).collect()),
            TestListQuery::Executions(query) => self
                .config
                .list_executions(&query)
                .map(|executions| executions.into_iter().map(TestEntity::Execution).collect()),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use memory_kernel::{
        domain_store::{CreateEntity, ReadEntity, StoreAccessMode},
        workspace::canonical_store_root,
    };
    use tempfile::tempdir;

    use super::*;
    use crate::{ValidationLinks, ValidationProvenance};

    #[test]
    fn typed_capabilities_write_and_read_back_only_from_selected_canonical_store() {
        let parent = tempdir().unwrap();
        let selected = parent.path().join("selected");
        let sibling = parent.path().join("sibling");
        std::fs::create_dir_all(&selected).unwrap();
        std::fs::create_dir_all(&sibling).unwrap();

        let resolution =
            TestStoreConfig::resolve_workspace_store(&selected, StoreAccessMode::CreateOrOpen)
                .unwrap();
        let store = TestDomainStore::new(TestStoreConfig::new(resolution.store_root));
        let spec = ValidationSpec::new("vt-domain-store", "Test domain store");
        store
            .create_entity(TestCreateInput::Spec(spec.clone()))
            .unwrap();

        let mut execution = ValidationExecution::passed(
            "exec-domain-store",
            &spec.id,
            chrono::Utc
                .with_ymd_and_hms(2026, 10, 7, 12, 0, 0)
                .single()
                .unwrap(),
        );
        execution.links = ValidationLinks {
            ticket_ids: vec!["b36ad4ef-77b9-4225-ab5e-abd0ba13b0b8".to_string()],
            ..Default::default()
        };
        execution.provenance = ValidationProvenance {
            domain: Some("test-api".to_string()),
            operation: Some("domain-store".to_string()),
            run_id: Some("fixture".to_string()),
            ..Default::default()
        };
        store
            .create_entity(TestCreateInput::Execution(execution.clone()))
            .unwrap();

        assert_eq!(
            store
                .read_entity(TestEntityId::Spec(spec.id.clone()))
                .unwrap(),
            TestEntity::Spec(spec)
        );
        assert_eq!(
            store
                .read_entity(TestEntityId::Execution(execution.id.clone()))
                .unwrap(),
            TestEntity::Execution(execution)
        );
        assert!(
            canonical_store_root(&selected, ".test")
                .join("specs/vt-domain-store.json")
                .is_file()
        );
        assert!(!canonical_store_root(parent.path(), ".test").exists());
        assert!(!canonical_store_root(&sibling, ".test").exists());
    }
}
