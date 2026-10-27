## Validation plan

- Add a focused test-cli regression for canonical workspace resolution with a conflicting legacy `.spec` index.
- Add or extend spec-api verification coverage for repeated recomputation when the canonical spec is already `verified`.
- Run `cargo test --manifest-path workflow-tools/Cargo.toml -p test-cli` and `cargo test --manifest-path workflow-tools/Cargo.toml -p spec-api`.
- Verify by id that canonical `spec get` and test-cli verification use the same `.workflow-tools/spec` root, and confirm the legacy `.workflow-tools/.spec` index was not updated.

## Results (2026-10-26)

- `cargo test --manifest-path workflow-tools/Cargo.toml --locked -p test-cli resolve_spec_root`: 2 passed (explicit and ambient canonical-root regressions).
- `cargo test --manifest-path workflow-tools/Cargo.toml --locked -p test-cli record_verifies_linked_spec_in_canonical_store_not_legacy_index`: 1 passed. The fixture creates the same spec ID as `implemented` in the canonical store and `verified` in the incorrect sibling store; the canonical record is updated and legacy state remains unchanged.
- `cargo test --manifest-path workflow-tools/Cargo.toml --locked -p spec-api`: 99 passed, including repeated guard recomputation for an already-verified spec.
- `cargo build --manifest-path workflow-tools/Cargo.toml --locked -p test-cli`: passed.
- Initial full test-cli run: 16 passed, 1 failed at `run_flags_over_budget_against_spec_threshold`; `echo slow` measured 0 ms against a 0 ms limit. See follow-up below.
- W3 package gate `ae41c0ca-75fa-497e-b4ac-90e3fa04f6ec-mission-w3-canonical-root-fix-01` passed 41 mission-api and 50 mission tests.
- No legacy store data was deleted or migrated. The test-cli path no longer consults the incorrectly derived legacy sibling.

## Follow-up results (2026-10-26)

- `run_harness` now uses only the actual spec UUIDs linked by the registered validation spec. It no longer treats a validation-spec ID as a linked spec ID. Runs with no linked spec IDs must provide another genuine traceability link, such as a ticket; affected tests now do so.
- The focused `run_verifies_specs_linked_from_the_validation_spec`, `run_failure_maps_to_failed_outcome`, `record_verifies_linked_spec_in_canonical_store_not_legacy_index`, and `resolve_spec_root` tests passed (5 tests total). Evidence: `a7f9f4a0-canonical-root-regressions-01`.
- Full test-cli suite: 17 passed, 1 failed. The sole failure is `run_flags_over_budget_against_spec_threshold`, which assumes `echo slow` exceeds a 0 ms threshold; its measured duration was 0 ms. Evidence: `a7f9f4a0-test-cli-full-suite-01`. This timing-sensitive failure is unrelated to spec-store resolution and was not changed.
- Final W3 end-to-end `test run` `ae41c0ca-75fa-497e-b4ac-90e3fa04f6ec-mission-w3-canonical-root-fix-03`, using the rebuilt CLI, passed `mission-api` (41 tests) and `mission` (50 tests), and auto-verified linked spec `44f100c4-215e-445a-b452-1a8a969d96cc`. `spec get` reads the canonical W3 spec as `verified`; no manual state transition was used.
- The bug ticket remains `in-implementation` pending its distinct review. W3 remains `in-implementation` pending W3 review.
