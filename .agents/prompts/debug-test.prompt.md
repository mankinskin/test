---
agent: agent
description: "Debug a failing test using log-first diagnosis, ticket context checks, and focused validation."
---

# Debug Test Workflow

Use this workflow when a test is failing or behavior regressed unexpectedly.

## Steps

1. Identify scope
- Determine failing crate, test name, and expected behavior from nearby tests.

2. Inspect logs first
- Read the relevant file in `target/test-logs/`, per [AGENTS.md](../../AGENTS.md#operating-principles)'s test-log rule.
- Use log-viewer tooling to search for errors and key spans.

3. Gather known context
- Check existing tickets for similar symptoms or known limitations, per [AGENTS.md](../../AGENTS.md#discovery-protocol-before-editing)'s known-issues rule.
- Read crate docs and nearby tests, per [AGENTS.md](../../AGENTS.md#operating-principles)'s context-gathering rule, before changing code.

4. Reproduce minimally
- Run focused test commands for the target crate/test.
- Avoid broad test runs until the local failure is understood.

5. Fix with minimal scope
- Change only what is needed for the failing contract.
- Add or adjust regression tests when behavior changes.

6. Validate
- Re-run focused tests, then relevant crate-level tests.
- If public behavior/docs changed, run documentation validation workflows.

## Escalation

Ask the user when:
- evidence is conflicting
- architecture tradeoffs are required
- behavior cannot be reproduced after focused investigation
