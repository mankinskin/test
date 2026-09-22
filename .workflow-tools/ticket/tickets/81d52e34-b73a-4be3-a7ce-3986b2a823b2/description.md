# Regression triage for latent failures

Add a `Regression Triage` section to `.agents/instructions/testing/test-debugging.instructions.md` immediately after `Debug Workflow` (currently line 15 in the 22-line instruction). The section must require historical comparison, isolated reproduction, and repeated execution before a failing test can be waived. A waived failure must be recorded as `LATENT`, never `ACCEPTABLE`.

The policy is needed because two ticket-route failures were waived as pre-existing based on source equivalence. Repeated execution later exposed a genuine ordering defect. Source-hash equality proves provenance only; source-hash equality does not prove correctness or absence of a flake.

The exact target is the existing test-debugging instruction because the requirement governs investigation of observed test failures. Adding the rule to `.agents/instructions/testing/test-execution.instructions.md` would emphasize command selection but leave the waiver decision undocumented at the debugging point where the failure is evaluated.