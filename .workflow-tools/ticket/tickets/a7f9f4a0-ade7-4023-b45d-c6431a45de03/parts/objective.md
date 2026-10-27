## Observed failure

The test CLI records a validation execution and then recomputes verification for each linked spec. For the W3 spec `44f100c4-215e-445a-b452-1a8a969d96cc`, this path reported a redundant `verified -> verified` transition, while canonical `spec get` returned state `implemented`.

## Confirmed cause

`test/crates/test-cli/src/lib.rs::resolve_spec_root` derives the spec root with `store_root.parent().join(".spec")` whenever the test store has a parent. With the canonical test store at `workflow-tools/.workflow-tools/test`, this selects the legacy sibling `workflow-tools/.workflow-tools/.spec` index instead of the canonical spec store `workflow-tools/.workflow-tools/spec`. The test CLI therefore verifies against a different, stale index than the spec CLI and reads the wrong indexed state.

The canonical W3 manifest remains `implemented` at revision 7. Do not manually change it or treat this verifier error as a W3 implementation failure.

## Scope

Fix test-cli workspace resolution for linked specs, add regression coverage for divergent legacy and canonical stores, and ensure repeated successful verification does not fail on a redundant state transition.