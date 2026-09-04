---
description: "Use before starting or while waiting on any cargo bench invocation. Covers estimating expected wall time, setting a hard timeout, and handling overruns."
---

## Timeout Discipline for Benchmark Runs

Before starting any `cargo bench` invocation (including `--test` smoke mode), estimate its expected wall time from the scenario count and sample/measurement settings (e.g. `sample_size × measurement_time × scenario_count`), and set a hard timeout at that estimate plus a modest buffer. Use an explicit `timeout` on the run, or background it and poll on a schedule bounded by that same budget. Never wait unboundedly on a benchmark process — see [tool-output.instructions.md](../orchestration/tool-output.instructions.md#long-running-process-ownership) for the general long-running-process rules this specializes.

When a run exceeds its budgeted timeout:
- Stop waiting on it (kill or detach) instead of continuing to poll indefinitely.
- Register whatever evidence it produced up to that point (partial scenario results, log tail) as the validation record, and name exactly which scenarios were not reached.
- Treat the remaining coverage gap as a follow-up, not a reason to silently re-launch the same exhaustive run hoping it finishes faster.

`--test` (fast smoke) mode is a correctness proxy, not a substitute for an acceptance criterion that literally requires Criterion statistical output. State that distinction explicitly in the validation evidence rather than treating smoke-mode success as sufficient on its own — see [benchmarks-criterion-calibration.instructions.md](benchmarks-criterion-calibration.instructions.md) for why `--test` produces zero timing data at all.
