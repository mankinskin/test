//! Test-domain adapter boundary onto the generic entity kernel's domain
//! manifest and migration contracts
//! (`memory_kernel::model::domain_manifest`, `memory_kernel::MigrationDryRunReport`,
//! `memory_kernel::MigrationPhase`).
//!
//! This module registers the test domain's two canonical record kinds
//! (validation specs and validation executions, see
//! [`crate::canonical::TestRecordKind`]) with the kernel's generic
//! [`DomainManifest`], and proves that test-api's existing durable
//! staged/published/resume/rollback migration ([`crate::migration`]) can be
//! described in terms of the kernel's generic [`MigrationDryRunReport`] and
//! [`KernelMigrationPhase`] vocabulary. [`crate::migration`] itself is not
//! rewritten or otherwise changed by this step: its manifest file, staging
//! directory, phases, and resume/rollback logic are all untouched.

use std::collections::BTreeMap;

use memory_kernel::{
    model::{
        domain::{DomainId, DomainSchemaVersion, EntityTypeId, EntityTypeSchemaVersion},
        domain_manifest::{
            DomainManifest, DomainManifestError, EntityTypeMembership, EntityTypeStatus,
        },
    },
    MigrationDryRunReport, MigrationPhase as KernelMigrationPhase,
};

use crate::{canonical::TestRecordKind, migration::MigrationPhase as TestMigrationPhase};

/// The kernel [`DomainId`] under which test-api registers itself.
pub const TEST_DOMAIN_ID: &str = "test";

/// The test domain's own [`DomainSchemaVersion`] as of this adoption step.
/// This tracks the domain manifest's own membership/activation set, not any
/// individual entity type's schema version (see [`EntityTypeMembership`]).
pub const TEST_DOMAIN_SCHEMA_VERSION: u32 = 1;

/// The kernel [`EntityTypeId`] string registered for one
/// [`TestRecordKind`]. Kept local to this module so [`crate::canonical`]
/// does not need to know about the kernel's domain-manifest vocabulary.
fn entity_type_id_for(kind: TestRecordKind) -> &'static str {
    match kind {
        TestRecordKind::Spec => "validation-spec",
        TestRecordKind::Execution => "validation-execution",
    }
}

/// Build the test domain's [`DomainManifest`] from its two currently
/// delivered record kinds ([`TestRecordKind::Spec`],
/// [`TestRecordKind::Execution`]). Both are registered at schema version 1
/// and [`EntityTypeStatus::Active`], since test-api has no existing
/// per-type versioning or inactive-type concept to preserve as of this
/// adoption step.
pub fn test_domain_manifest() -> Result<DomainManifest, DomainManifestError> {
    let domain_id = DomainId::new(TEST_DOMAIN_ID).expect("TEST_DOMAIN_ID is non-empty");

    let entity_types = [TestRecordKind::Spec, TestRecordKind::Execution]
        .into_iter()
        .map(|kind| EntityTypeMembership {
            entity_type_id: EntityTypeId::new(entity_type_id_for(kind))
                .expect("test record kind entity type id is non-empty"),
            schema_version: EntityTypeSchemaVersion(1),
            status: EntityTypeStatus::Active,
        })
        .collect();

    DomainManifest::new(
        domain_id,
        DomainSchemaVersion(TEST_DOMAIN_SCHEMA_VERSION),
        entity_types,
    )
}

/// Build a kernel-shaped dry-run report for the test domain's current
/// manifest state: every active entity type's target schema version equals
/// its current version, so the report always plans zero steps and zero
/// violations. This proves test-api can produce the kernel's generic
/// [`MigrationDryRunReport`] shape from its own domain manifest without any
/// actual schema change, and without touching [`crate::migration`]'s
/// existing staged/published behavior.
pub fn test_domain_current_state_report() -> Result<MigrationDryRunReport, DomainManifestError> {
    let manifest = test_domain_manifest()?;
    let target_versions: BTreeMap<EntityTypeId, EntityTypeSchemaVersion> = manifest
        .active_entity_types()
        .map(|membership| (membership.entity_type_id.clone(), membership.schema_version))
        .collect();

    Ok(MigrationDryRunReport::plan(
        &manifest,
        manifest.schema_version,
        &target_versions,
        &BTreeMap::new(),
        &[],
    ))
}

/// Map test-api's existing durable migration phase
/// ([`crate::migration::MigrationPhase`]) onto the kernel's generic
/// [`KernelMigrationPhase`] vocabulary. This is a pure vocabulary mapping:
/// it does not change test-api's own migration manifest, staging directory,
/// or resume/rollback logic in any way.
pub fn kernel_migration_phase(local: TestMigrationPhase) -> KernelMigrationPhase {
    match local {
        TestMigrationPhase::Staged => KernelMigrationPhase::Staged,
        TestMigrationPhase::Published => KernelMigrationPhase::Published,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn test_domain_manifest_registers_both_record_kinds_as_active() {
        let manifest = test_domain_manifest().expect("test domain manifest is valid");

        assert_eq!(manifest.domain_id.as_str(), TEST_DOMAIN_ID);
        assert_eq!(
            manifest.schema_version,
            DomainSchemaVersion(TEST_DOMAIN_SCHEMA_VERSION)
        );

        let registered: BTreeSet<String> = manifest
            .entity_types()
            .iter()
            .map(|m| m.entity_type_id.as_str().to_string())
            .collect();
        assert_eq!(
            registered,
            BTreeSet::from([
                "validation-spec".to_string(),
                "validation-execution".to_string(),
            ])
        );
        assert_eq!(manifest.active_entity_types().count(), 2);
        assert_eq!(manifest.inactive_entity_types().count(), 0);
    }

    #[test]
    fn test_domain_manifest_membership_versions_are_one() {
        let manifest = test_domain_manifest().expect("test domain manifest is valid");

        for kind in [TestRecordKind::Spec, TestRecordKind::Execution] {
            let entity_type_id = EntityTypeId::new(entity_type_id_for(kind)).unwrap();
            let membership = manifest
                .membership(&entity_type_id)
                .unwrap_or_else(|| panic!("{entity_type_id} missing from test domain manifest"));
            assert_eq!(membership.schema_version, EntityTypeSchemaVersion(1));
            assert!(membership.status.is_active());
        }
    }

    #[test]
    fn current_state_report_plans_zero_steps_and_zero_violations() {
        let report =
            test_domain_current_state_report().expect("current-state dry-run report can be built");

        assert_eq!(report.domain_id.as_str(), TEST_DOMAIN_ID);
        assert_eq!(
            report.from_domain_schema_version,
            DomainSchemaVersion(TEST_DOMAIN_SCHEMA_VERSION)
        );
        assert_eq!(
            report.to_domain_schema_version,
            DomainSchemaVersion(TEST_DOMAIN_SCHEMA_VERSION)
        );
        assert!(report.steps.is_empty());
        assert!(report.inactive_entity_types.is_empty());
        assert!(report.external_urn_violations.is_empty());
    }

    #[test]
    fn kernel_migration_phase_maps_every_local_phase() {
        assert_eq!(
            kernel_migration_phase(TestMigrationPhase::Staged),
            KernelMigrationPhase::Staged
        );
        assert_eq!(
            kernel_migration_phase(TestMigrationPhase::Published),
            KernelMigrationPhase::Published
        );
    }
}
