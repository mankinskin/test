---
description: "Use when running tests, choosing test scope, or executing validation commands. Covers test execution patterns, scope selection, and frontend/viewer validation."
---

## Quick Reference — Common Commands

```bash
# Run a single test by name (fastest, use first)
cargo test -p <crate> <test_name> -- --nocapture

# Run all tests in a crate
cargo test -p ticket-api
cargo test -p ticket-http

# Full workspace test (slow — only after local crate tests pass)
cargo test
```

## Test Execution Strategy

- Start with nearest unit/integration tests.
- Expand to crate-level runs once local failures are resolved.
- Keep working outward until the required validation passes or you have a clearly repeated blocker to report.
- Prefer the strongest focused validation surface already owned by the changed tool or crate; run the underlying command directly and record the exact command or manual step in ticket/spec summaries.
- For documentation or generated-guidance checks, run the relevant validation command directly and record unsupported coverage or manual follow-up explicitly.
- If dedicated automation is unavailable, use the closest manual or command-line validation path and record the limitation in the status summary.
- Avoid unrelated full-workspace test runs unless required.

For frontend-impacting changes:

- Run lint and typecheck in each affected frontend package.
- Run nearest unit/component tests for changed UI code.
- Run at least one browser-based end-to-end path that covers changed UX behavior.

For viewer/API integration changes:

- Add or run assertions that verify the viewer contract with context-api or ticket-api for changed endpoints.
- For filesystem-backed behaviors, include path-handling and access-boundary assertions.

For performance-sensitive paths (storage, BFS, graph queries):

- Add or run a Criterion benchmark in `crates/<crate>/benches/`.
- Confirm `phase3_get_indexed_many` is used instead of repeated `get_indexed()` calls.

For regression fixes:

- Prefer a failing reproducer assertion before or with the fix.
- Keep regression coverage focused on the reported failure mode.
