Back to [workflow-tools](..).

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

## Related Crates

- [crates/test-api](crates/test-api): core domain library (specs, executions, storage).
- [crates/test-cli](crates/test-cli): CLI transport (binary `test`).
- [crates/test-mcp](crates/test-mcp): MCP server transport.
