## Objective
Build exhaustive regression and benchmark coverage for cross-repository move safety.

## Requirements
- Cover same, parent-to-submodule, submodule-to-parent, and unrelated repository topologies.
- Cover single/set plan, apply, resume, rollback, sequential batches, reconciliation, and fixed commit order.
- Cover every accepted blocker: invisible references, missing code refs, incomplete batch closure, dirty tracked files, incompatible filesystem devices, active leases, and non-terminal journals.
- Measure preflight, apply, scan, rewrite, index, recovery, and read-back separately.
- Record warm/cold conditions and identify slow paths with scenario labels.

## Acceptance Criteria
1. Every matrix cell has a deterministic pass/fail assertion and fixture identity.
2. Negative cases prove failure occurs before mutation.
3. Benchmarks include successful and blocked cross-repository scenarios and report operation-level timings.
4. The matrix runs in CI or the repository's canonical benchmark command and produces inspectable evidence.

## Validation
`cargo test --manifest-path workflow-tools/memory-kernel/Cargo.toml`
`cargo test --manifest-path workflow-tools/spec/crates/spec-api/Cargo.toml`
`cargo bench --manifest-path workflow-tools/test/Cargo.toml`
