## Objective
Improve targeted coverage for thinly tested crates.

## Context
Audit `tmp/test-coverage-audit/01b-coverage-verification.md`: spec-viewer 2.4/KLOC, audit-api 2.7, peek-api 2.9, doc-viewer 3.5, memory-matrix 4.8; context-api is 23.9/KLOC.

## Acceptance criteria
- Add behavior-focused coverage plan per named crate.
- Prioritize public contract and failure boundaries.

## Out of scope
- Density quotas or broad refactors.