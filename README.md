Back to [workflow-tools](https://github.com/mankinskin/workflow-tools/blob/main/README.md).

# test

Validation evidence store: record what validation specs exist and what happened when they ran, and query that evidence back out by ticket, spec, or outcome.

## Primary Use Case

After running a check (a test, a manual verification, a benchmark), record its outcome as a linked, queryable execution instead of leaving evidence only in a chat transcript or a scrollback log.

## Usage

Build the desired transport from the `workflow-tools` workspace root:

```bash
cargo run -p test-cli --bin test -- --help
cargo run -p test-mcp --bin test-mcp
```

`test` finds the nearest `.test` workspace by walking up from the current directory.

## Examples

```bash
# Record a validation spec
cargo run -p test-cli --bin test -- record-spec --id vt-example --title "Example check"

# Record an execution outcome
cargo run -p test-cli --bin test -- record-execution --id exec-example --validation-spec-id vt-example --outcome passed

# List executions for a ticket
cargo run -p test-cli --bin test -- list-executions --ticket-id <ticket-id>
```

## Run a stored validation definition

From the `workflow-tools` repository root, use the existing `test` binary:

```bash
rtk bypass ./target/debug/test --workspace "$PWD" run \
  --spec-id mission-w3-package-gate \
  --id "mission-w3-package-gate-$(date -u +%Y%m%dT%H%M%SZ)" \
  --run-id "manual-$(date -u +%Y%m%dT%H%M%SZ)" \
  --domain mission --operation package-validation \
  --log-dir target/test-logs
```

`run` reads the definition's `command` from the selected workspace's test
store. It uses the existing harness to execute the command and record timing,
outcome and captured output. A missing definition or missing/blank command is
an error before launching a process or writing execution/log records.
Only run definitions whose commands you trust.

The stored definition is the sole source of its command, slow-run budget and
all links (Tickets, specifications, criteria and evidence). `run` no longer
accepts `--command`, `--slow-threshold-ms` or `--ticket`. Unknown spec IDs
cannot be used for ad hoc runs. Edit the definition first when its execution
contract must change.

Existing execution validation still requires provenance (`domain`,
`operation`, `run_id`) and a spec, criterion or Ticket link. Links must be
present in the definition. Provenance is inherited when stored; otherwise
supply it at launch. Supplying any already-stored provenance field again is
an error, even with the same value. The example assumes domain, operation and
run ID are absent from the definition.
Missing required execution metadata is rejected before the command starts.
Run identity, measured throughput and log destination remain per-run inputs.

Commands execute in the caller's current directory. `--workspace` selects the
entity store, not the command's working directory; check relative paths before
running older definitions. Use distinct `--id` or `--run-id` values to retain
separate executions; the existing default ID remains the spec ID.

To query results:

```bash
rtk bypass ./target/debug/test --workspace "$PWD" list \
  --spec-id mission-w3-package-gate --sort newest-first --limit 5
rtk bypass ./target/debug/test --workspace "$PWD" logs --execution <execution-id>
```

If needed, build the binary with `cargo build --locked -p test-cli --bin test`,
or invoke it through `cargo run --locked -p test-cli --bin test -- ...`.
The existing definition schema is unchanged.

### MCP parity

The `test_run` MCP tool executes the same stored definition through the
shared typed CLI harness (no subprocess CLI invocation or duplicated runner):

```json
{
  "workspace": "/home/linus/git/meta-workspace/workflow-tools",
  "spec_id": "mission-w3-package-gate",
  "run_id": "manual-20261028-01",
  "domain": "mission",
  "operation": "package-validation",
  "log_dir": "target/test-logs"
}
```

It accepts the same run-only inputs as the CLI: `id`, `run_id`, `throughput`,
`log_dir`, `source_path`, `test_id`, `domain`, `operation` and `transport`.
Definition overrides and unknown fields are rejected, not silently ignored.
MCP commands use the server process's current working directory. Work runs
on a blocking worker so shell execution does not block the async server.
Rebuild/restart the MCP server to expose the new tool; an already-running
server does not acquire it automatically.

## Related Crates

- [crates/test-api](crates/test-api): core domain library (specs, executions, storage).
- [crates/test-cli](crates/test-cli): CLI transport (binary `test`).
- [crates/test-mcp](crates/test-mcp): MCP server transport.
