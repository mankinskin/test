<!-- aligned-structure:v2 -->
# Cross-Repository Move Regression And Benchmark Matrix

## Target Code Location
- [Memory-kernel tests](../../workflow-tools/memory-kernel)
- [Spec API tests](../../workflow-tools/spec/crates/spec-api)
- [Move benchmark tickets](../../workflow-tools/.workflow-tools/ticket/tickets)

## Naming Conventions
Scenarios are keyed by topology (`same`, `parent-to-submodule`, `submodule-to-parent`, `unrelated`), operation (`single`, `set`, `resume`, `rollback`), and outcome (`supported`, `blocked`, `slow`).

## Requester Input
> Cross-repository moves must be tested intensively and redundantly, and benchmarks must cover these cases and identify slow operations.

## Reading Order
1. [Parent safety contract](../../workflow-tools/.workflow-tools/spec/specs/7487a6b5-39c6-4114-a697-6ed0556e888e/body.md)
2. [Kernel child contract](../../workflow-tools/.workflow-tools/spec/specs/c095bb6b-f343-4ae9-9282-0d51a06d099a/body.md)
3. [Spec-domain child contract](../../workflow-tools/.workflow-tools/spec/specs/867cd511-5bb6-494f-8dff-a150f4953f02/body.md)
4. [Benchmark instructions](../../workflow-tools/test/.agents/instructions/testing/benchmarks.instructions.md)

## Responsibility
Provide executable regression and performance evidence for every accepted cross-repository move path and every safety blocker.

## Interfaces And Dependencies
The matrix drives memory-kernel and spec-api tests, uses isolated repositories and stores, measures preflight/apply/recovery/read-back separately, and records benchmark budgets and slow-path diagnostics.

## Behavior
Every topology is tested for single and set moves, visibility failures, missing code references, incomplete batch closure, dirty tracked files, filesystem incompatibility, sequential batches, reconciliation, and fixed commit order. Benchmarks cover successful and blocked paths and report operation-level latency.

## Boundaries And Failure Cases
A passing happy-path test never substitutes for negative coverage. A benchmark without scenario labels, warm/cold state, or failure-path timing is incomplete.

## Provider/Consumer Contract
This contract consumes the kernel and Spec-domain contracts and provides release evidence to the staged migration waypoint.

## Examples
A sibling-repository set move must pass the same assertions as a parent/submodule move plus the explicit cross-repository safeguards, and its benchmark result must identify any filesystem or scan bottleneck.

## Evidence
The acceptance artifact is a complete scenario matrix with passing test output and benchmark reports linked to the implementation tickets.

## Scope
Owner: `test`; implementation is ticket-backed and remains partial until the matrix and benchmark suite pass.
