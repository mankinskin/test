---
description: "Use when verifying an acceptance criterion that asserts data was captured, collected, populated, persisted, or recorded. Covers the artifact read-back requirement and forbidden proxy-evidence forms."
---

## Data-Capture Acceptance Criteria Require Artifact Read-Back

If an acceptance criterion's wording asserts that data is **captured**, **collected**,
**populated**, **persisted**, or **recorded**, it can only be verified by **reading the
produced file or record and observing the actual value**. The reviewer must state the
artifact path read and the observed value in the ticket/spec evidence trail.

### Forbidden evidence forms for these ACs

None of the following satisfy a data-capture AC, even when true:

- Citing source code that *should* produce the data (code-existence tracing).
- Naming a test that exercises the code path, without reading that test's output artifact.
- A passing-test count or unit-test summary (for example "148 tests passed") with no
  artifact read-back — unit tests frequently hand-construct inputs the real producer
  never emits, so a green suite does not prove the real pipeline populates the artifact.

### Required evidence form

- Run the real pipeline (binary, hook, or service) against a producer-shaped input.
- Read the actual output artifact (file, record, or store entry) it produces.
- Record the artifact path and the observed value (e.g. a non-empty map, a specific
  field value) in the validation execution or ticket status summary.

This rule exists because proxy evidence was accepted in place of outcome evidence for
months: a ticket was marked `done` via code-existence tracing while the artifact its
acceptance criteria described was actually empty, and a per-module unit-test count
(`.test/default/specs/val-session-api-lib-suite.json`) counted tests that hand-construct
inputs (`role: Tool` turns) the real producer never emits. See ticket `ce7b7bde` for the
full recurrence analysis and the mandatory e2e validation spec
(`val-session-api-tool-metrics-e2e`) that replaced the proxy evidence.
