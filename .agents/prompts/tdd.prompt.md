---
description: "Plan and execute work with a tight test-driven loop and explicit validation evidence."
name: "tdd"
argument-hint: "<ticket, behavior, or slice>"
agent: "agent"
---

# Test-Driven Development

Drive the requested work with the smallest useful red-green-refactor loop available in this repository.

Reference [AGENTS](https://github.com/mankinskin/workflow-tools/blob/main/AGENTS.md), [ticket workflow](https://github.com/mankinskin/ticket/blob/main/.agents/instructions/ticket/workflow.instructions.md), [spec workflow](https://github.com/mankinskin/spec/blob/main/.agents/instructions/spec/spec-system.instructions.md), [test evidence](../instructions/testing/validation-evidence.instructions.md), and [log API](https://github.com/mankinskin/log/blob/main/README.md).

## Workflow

1. Anchor the work to a ticket, failing behavior, or concrete code path.
2. Define the smallest test or validation that can fail first.
3. Describe the expected `ValidationSpec` and the first `ValidationExecution` you intend to record.
4. Run the failing check before widening scope.
5. Make the smallest implementation change that should satisfy the check.
6. Re-run the same focused validation before proceeding.
7. Capture the result in terms of test evidence and any generated logs.
8. If the check cannot be automated yet, say what shallow coverage is possible now and what stronger coverage should be added next.

## Response

Return:
- the chosen slice and failing behavior
- the first red check
- the expected green check
- the evidence plan for tests and logs
- the next refactor or follow-up step
