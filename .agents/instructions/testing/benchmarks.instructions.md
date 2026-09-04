---
description: "Use when running Criterion benchmarks or adding performance measurements. Covers benchmark commands and fixture details."
---

## Criterion Benchmarks

The BFS graph query pipeline is benchmarked in `crates/ticket-api/benches/graph_ops.rs`.
Run with: `cargo bench --bench graph_ops -p ticket-api`

| Benchmark | What it measures |
|---|---|
| `phase1_list_all_edges` | ReDB edge table scan (~630 edges) |
| `phase2_bfs_in_memory` | Pure in-memory BFS, no DB |
| `phase3_get_indexed_many` | Batch metadata fetch (1 ReDB transaction, 39 nodes) |
| `phase3_get_indexed_one_by_one` | Per-node fetch baseline (39 separate transactions) |
| `pipeline_full` | All 3 phases end-to-end |
| `pipeline_concurrent/{2,4,8,16,32}` | N threads barrier-synchronized |

The fixture builds 360 tickets + ~630 edges once per process (via `OnceLock`).

When adding a new storage-layer optimization, add a matching Criterion benchmark that shows the before/after comparison.

## Related Guidance

- [benchmarks-timeout.instructions.md](benchmarks-timeout.instructions.md) — estimate expected wall time, set a hard timeout, and handle overruns for any `cargo bench` run.
- [benchmarks-criterion-calibration.instructions.md](benchmarks-criterion-calibration.instructions.md) — per-scenario Criterion configuration for a benchmark group whose scenarios vary widely in cost (entity count, link density, etc.).
