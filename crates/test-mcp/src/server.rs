use std::path::PathBuf;

use chrono::{DateTime, Utc};
use memory_kernel::domain_store::StoreAccessMode;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars::{self, JsonSchema},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::{Deserialize, Serialize};
use test_api::{
    ExecutionQuery, ExecutionSort, TestError, TestRecordKind, TestStoreConfig, ValidationExecution,
    ValidationLinks, ValidationOutcome, ValidationProvenance, ValidationSpec,
};
use uuid::Uuid;

// ── Input types ───────────────────────────────────────────────────────────────

/// Run-only metadata. Definition-owned fields cannot be supplied here.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunInput {
    /// Explicit owning workspace/store selector, not the command working directory.
    pub workspace: String,
    /// Existing stored validation definition; it owns command, budget and links.
    pub spec_id: String,
    /// Execution ID; defaults to run_id-spec_id, otherwise spec_id.
    pub id: Option<String>,
    /// Run grouping, supplied only when absent from the definition.
    pub run_id: Option<String>,
    /// Measured throughput for this execution.
    pub throughput: Option<f64>,
    /// Log output directory relative to the MCP process working directory.
    pub log_dir: Option<PathBuf>,
    /// Supply provenance fields only when absent from the definition.
    pub source_path: Option<String>,
    pub test_id: Option<String>,
    pub domain: Option<String>,
    pub operation: Option<String>,
    pub transport: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RecordSpecInput {
    /// Concrete workspace path, repo root, .test store path, or path inside that store. Do not use omitted, empty, 'default', or '..' for entity creation; use '.' explicitly to target the MCP server process's current working directory.
    pub workspace: String,
    /// Stable validation spec id (e.g. `vt-core-tests`). Used as the file name.
    pub id: String,
    /// Human-readable title of the validation check.
    pub title: String,
    /// Command that performs the check (e.g. `cargo test -p ticket-vscode-core`).
    #[serde(default)]
    pub command: Option<String>,
    /// Additional detail or notes about the validation spec.
    #[serde(default)]
    pub detail: Option<String>,
    /// Slow-run budget threshold in milliseconds for this validation spec.
    #[serde(default)]
    pub slow_threshold_ms: Option<u64>,
    /// Ticket ids this spec is associated with.
    #[serde(default)]
    pub ticket_ids: Vec<String>,
    /// Related spec ids (spec-api) this validation spec covers.
    #[serde(default)]
    pub spec_ids: Vec<String>,
    /// Acceptance criterion ids this validation spec targets.
    #[serde(default)]
    pub acceptance_criterion_ids: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RecordExecutionInput {
    /// Concrete workspace path, repo root, .test store path, or path inside that store. Do not use omitted, empty, 'default', or '..' for entity creation; use '.' explicitly to target the MCP server process's current working directory.
    pub workspace: String,
    /// Stable execution id (e.g. `exec-vt-core-tests-20260615`). Used as file name.
    pub id: String,
    /// The validation spec id this execution is an outcome of.
    pub validation_spec_id: String,
    /// Outcome of the run: `passed`, `failed`, or `blocked`.
    pub outcome: String,
    /// RFC3339 timestamp of when the check ran. Defaults to now when omitted.
    #[serde(default)]
    pub executed_at: Option<String>,
    /// Wall time in milliseconds for the validated operation.
    #[serde(default)]
    pub duration_ms: Option<u64>,
    /// Optional throughput metric (ops/sec or items/sec).
    #[serde(default)]
    pub throughput: Option<f64>,
    /// Result summary or notes (command output highlights, blocker reason, etc.).
    #[serde(default)]
    pub detail: Option<String>,
    /// Ticket ids this execution provides evidence for.
    #[serde(default)]
    pub ticket_ids: Vec<String>,
    /// Related spec ids (spec-api) this execution provides evidence for.
    #[serde(default)]
    pub spec_ids: Vec<String>,
    /// Acceptance criterion ids this execution satisfies.
    #[serde(default)]
    pub acceptance_criterion_ids: Vec<String>,
    /// Doc-evidence ids linked to this execution.
    #[serde(default)]
    pub doc_evidence_ids: Vec<String>,
    /// Log ids linked to this execution.
    #[serde(default)]
    pub log_ids: Vec<String>,
    /// Source file path that produced this execution.
    #[serde(default)]
    pub source_path: Option<String>,
    /// Stable test/cell id inside source_path.
    #[serde(default)]
    pub test_id: Option<String>,
    /// Domain under test.
    #[serde(default)]
    pub domain: Option<String>,
    /// Operation under test.
    #[serde(default)]
    pub operation: Option<String>,
    /// Transport used (`cli`, `mcp`, `http`, `in-process`, ...).
    #[serde(default)]
    pub transport: Option<String>,
    /// Run id grouping executions from one harness invocation.
    #[serde(default)]
    pub run_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetSpecInput {
    /// Validation spec id to fetch.
    pub id: String,
    /// Concrete workspace path, repo root, .test store path, or path inside
    /// that store. When omitted, searches every `.test` store discoverable
    /// from the server's workspace root.
    #[serde(default)]
    pub workspace: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetExecutionInput {
    /// Validation execution id to fetch.
    pub id: String,
    /// Concrete workspace path, repo root, .test store path, or path inside
    /// that store. When omitted, searches every `.test` store discoverable
    /// from the server's workspace root.
    #[serde(default)]
    pub workspace: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListSpecsInput {
    /// Concrete workspace path, repo root, .test store path, or path inside
    /// that store. When omitted, aggregates every `.test` store discoverable
    /// from the server's workspace root.
    #[serde(default)]
    pub workspace: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListExecutionsInput {
    /// Concrete workspace path, repo root, .test store path, or path inside
    /// that store. When omitted, aggregates every `.test` store discoverable
    /// from the server's workspace root.
    #[serde(default)]
    pub workspace: Option<String>,
    /// Only return executions linked to this ticket id.
    #[serde(default)]
    pub ticket_id: Option<String>,
    /// Only return executions for this validation spec id.
    #[serde(default)]
    pub validation_spec_id: Option<String>,
    /// Only return executions with this outcome (`passed`, `failed`, `blocked`).
    #[serde(default)]
    pub outcome: Option<String>,
    /// Only return executions with duration >= this threshold.
    #[serde(default)]
    pub min_duration_ms: Option<u64>,
    /// Only return executions with duration <= this threshold.
    #[serde(default)]
    pub max_duration_ms: Option<u64>,
    /// Only return executions in this provenance domain.
    #[serde(default)]
    pub domain: Option<String>,
    /// Only return executions for this provenance operation.
    #[serde(default)]
    pub operation: Option<String>,
    /// Only return executions for this transport.
    #[serde(default)]
    pub transport: Option<String>,
    /// Only return executions for this run id.
    #[serde(default)]
    pub run_id: Option<String>,
    /// Sort order (`newest-first` or `slowest-first`).
    #[serde(default)]
    pub sort: Option<String>,
    /// Maximum number of executions to return (newest first).
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestMoveInput {
    /// Concrete workspace path, repo root, .test store path, or path inside
    /// that store. When omitted, uses the server's own fixed store.
    #[serde(default)]
    pub workspace: Option<String>,
    /// Record kind being moved: `spec` or `execution`.
    pub kind: String,
    /// Legacy identifier or canonical UUID of the record to move.
    pub id: String,
    /// Destination workspace root.
    pub to_workspace_root: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestMoveJournalInput {
    /// Concrete workspace path, repo root, .test store path, or path inside
    /// that store. When omitted, uses the server's own fixed store.
    #[serde(default)]
    pub workspace: Option<String>,
    /// Record kind being moved: `spec` or `execution`. Must match the kind
    /// used when the move was planned.
    pub kind: String,
    /// Move journal UUID.
    pub id: String,
}

// ── Server ───────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct TestServer {
    store_root: PathBuf,
    tool_router: ToolRouter<Self>,
}

impl TestServer {
    pub fn new(store_root: PathBuf) -> Self {
        Self {
            store_root,
            tool_router: Self::tool_router(),
        }
    }

    fn config(&self) -> TestStoreConfig {
        TestStoreConfig::new(self.store_root.clone())
    }

    fn config_for_optional_workspace(
        &self,
        workspace: Option<&str>,
    ) -> Result<TestStoreConfig, McpError> {
        match workspace {
            Some(selector) => self.config_for_workspace(selector),
            None => Ok(self.config()),
        }
    }

    fn parse_record_kind(raw: &str) -> Result<TestRecordKind, McpError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "spec" => Ok(TestRecordKind::Spec),
            "execution" => Ok(TestRecordKind::Execution),
            other => Err(McpError::invalid_params(
                format!("invalid kind `{other}` (expected spec or execution)"),
                None,
            )),
        }
    }

    fn config_for_workspace(&self, workspace_selector: &str) -> Result<TestStoreConfig, McpError> {
        let workspace = memory_kernel::workspace::normalize_explicit_workspace_selector(Some(
            workspace_selector,
        ))
        .map_err(|err| McpError::invalid_params(err.to_string(), None))?;
        let resolution =
            TestStoreConfig::resolve_workspace_store(&workspace, StoreAccessMode::ReadOnly)
                .map_err(Self::test_err)?;
        Ok(TestStoreConfig::new(resolution.store_root))
    }

    fn config_for_write_workspace(
        &self,
        workspace_selector: &str,
    ) -> Result<TestStoreConfig, McpError> {
        let workspace = memory_kernel::workspace::normalize_explicit_workspace_selector(Some(
            workspace_selector,
        ))
        .map_err(|err| McpError::invalid_params(err.to_string(), None))?;
        let resolution =
            TestStoreConfig::resolve_workspace_store(&workspace, StoreAccessMode::CreateOrOpen)
                .map_err(Self::test_err)?;
        Ok(TestStoreConfig::new(resolution.store_root))
    }

    fn config_for_optional_write_workspace(
        &self,
        workspace: Option<&str>,
    ) -> Result<TestStoreConfig, McpError> {
        match workspace {
            Some(selector) => self.config_for_write_workspace(selector),
            None => Ok(self.config()),
        }
    }

    /// Every `.test` store discoverable from the server's workspace root,
    /// deduped, with the server's own fixed store listed first. Used by read
    /// tools when no explicit `workspace` argument is supplied, so evidence
    /// recorded to a nested store (e.g. `memory-api/.test`) via an explicit
    /// `workspace` on write remains discoverable from the aggregated root.
    fn discover_configs(&self) -> Vec<TestStoreConfig> {
        let workspace_root = memory_kernel::workspace::resolve_workspace_root_from_store_root(
            &self.store_root,
            ".test",
        );

        let mut seen = std::collections::BTreeSet::new();
        let mut configs = Vec::new();

        if seen.insert(self.store_root.clone()) {
            configs.push(self.config());
        }

        for root in memory_kernel::workspace::discover_workspace_store_roots(
            &workspace_root,
            ".test",
            "executions",
        ) {
            if seen.insert(root.clone()) {
                configs.push(TestStoreConfig::new(root));
            }
        }

        configs
    }

    /// Resolve store configs for a read tool: a single explicit store when
    /// `workspace` is supplied, otherwise every discoverable store.
    fn configs_for_optional_workspace(
        &self,
        workspace: Option<&str>,
    ) -> Result<Vec<TestStoreConfig>, McpError> {
        match workspace {
            Some(selector) => Ok(vec![self.config_for_workspace(selector)?]),
            None => Ok(self.discover_configs()),
        }
    }

    /// Merge-sort executions gathered from multiple stores using the same
    /// ordering as `TestStoreConfig::list_executions`.
    fn merge_sort_executions(
        mut executions: Vec<ValidationExecution>,
        sort: ExecutionSort,
    ) -> Vec<ValidationExecution> {
        match sort {
            ExecutionSort::NewestFirst => {
                executions.sort_by(|a, b| b.executed_at.cmp(&a.executed_at).then(a.id.cmp(&b.id)));
            }
            ExecutionSort::SlowestFirst => {
                executions.sort_by(|a, b| {
                    b.duration_ms
                        .cmp(&a.duration_ms)
                        .then(b.executed_at.cmp(&a.executed_at))
                        .then(a.id.cmp(&b.id))
                });
            }
        }
        executions
    }

    fn json_result<T: Serialize>(value: &T) -> Result<CallToolResult, McpError> {
        let text = serde_json::to_string(value)
            .map_err(|err| McpError::internal_error(format!("serialization: {err}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    fn test_err(err: TestError) -> McpError {
        match &err {
            TestError::EmptyRoot
            | TestError::InvalidId(_)
            | TestError::InvalidWorkspaceSlug(_)
            | TestError::Workspace(_)
            | TestError::LegacyStoreWrite(_)
            | TestError::SpecNotFound(_)
            | TestError::ExecutionNotFound(_) => McpError::invalid_params(err.to_string(), None),
            _ => McpError::internal_error(format!("test error: {err}"), None),
        }
    }

    fn parse_outcome(raw: &str) -> Result<ValidationOutcome, McpError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "passed" | "pass" => Ok(ValidationOutcome::Passed),
            "failed" | "fail" => Ok(ValidationOutcome::Failed),
            "blocked" | "block" => Ok(ValidationOutcome::Blocked),
            other => Err(McpError::invalid_params(
                format!("invalid outcome `{other}` (expected passed, failed, or blocked)"),
                None,
            )),
        }
    }

    fn parse_timestamp(raw: Option<&str>) -> Result<DateTime<Utc>, McpError> {
        match raw {
            None => Ok(Utc::now()),
            Some(value) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Ok(Utc::now());
                }
                DateTime::parse_from_rfc3339(trimmed)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|err| {
                        McpError::invalid_params(
                            format!("invalid executed_at `{trimmed}`: {err} (expected RFC3339)"),
                            None,
                        )
                    })
            }
        }
    }

    fn parse_sort(raw: Option<&str>) -> Result<ExecutionSort, McpError> {
        match raw {
            None => Ok(ExecutionSort::NewestFirst),
            Some(value) => match value.trim().to_ascii_lowercase().as_str() {
                "newest-first" | "newest" => Ok(ExecutionSort::NewestFirst),
                "slowest-first" | "slowest" => Ok(ExecutionSort::SlowestFirst),
                other => Err(McpError::invalid_params(
                    format!("invalid sort `{other}` (expected newest-first or slowest-first)"),
                    None,
                )),
            },
        }
    }
}

// ── Tool implementations ──────────────────────────────────────────────────────

#[tool_router]
impl TestServer {
    #[tool(
        name = "test_run",
        description = "Execute a trusted stored validation definition. Command, budget and all links come only from the definition. Supply provenance only if absent from the definition. Commands run in the server process working directory, not the selected store. Captures timing, outcome and logs using the same harness as the CLI."
    )]
    pub async fn test_run(
        &self,
        Parameters(input): Parameters<RunInput>,
    ) -> Result<CallToolResult, McpError> {
        let value = tokio::task::spawn_blocking(move || {
            test_cli::run(test_cli::TestCli {
                json: true,
                toon: false,
                workspace_root: Some(PathBuf::from(input.workspace)),
                command: test_cli::TestCommand::Run(test_cli::RunArgs {
                    spec_id: input.spec_id,
                    id: input.id,
                    run_id: input.run_id,
                    throughput: input.throughput,
                    log_dir: input
                        .log_dir
                        .unwrap_or_else(|| PathBuf::from("target/test-logs")),
                    source_path: input.source_path,
                    test_id: input.test_id,
                    domain: input.domain,
                    operation: input.operation,
                    transport: input.transport,
                }),
            })
        })
        .await
        .map_err(|error| {
            McpError::internal_error(format!("test runner task failed: {error}"), None)
        })?
        .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
        match value {
            test_cli::CliOutput::Machine(value, _) => Self::json_result(&value),
            test_cli::CliOutput::Text(_) => Err(McpError::internal_error(
                "test runner returned unexpected text output",
                None,
            )),
        }
    }

    #[tool(
        name = "test_record_spec",
        description = "Record (create or overwrite) a validation spec describing a test or check to run."
    )]
    pub async fn test_record_spec(
        &self,
        Parameters(input): Parameters<RecordSpecInput>,
    ) -> Result<CallToolResult, McpError> {
        let spec = ValidationSpec {
            id: input.id,
            title: input.title,
            command: input.command,
            detail: input.detail,
            slow_threshold_ms: input.slow_threshold_ms,
            links: ValidationLinks {
                spec_ids: input.spec_ids,
                acceptance_criterion_ids: input.acceptance_criterion_ids,
                ticket_ids: input.ticket_ids,
                doc_evidence_ids: Vec::new(),
                log_ids: Vec::new(),
            },
            provenance: ValidationProvenance::default(),
        };
        let path = self
            .config_for_write_workspace(&input.workspace)?
            .record_spec(&spec)
            .map_err(Self::test_err)?;
        Self::json_result(&serde_json::json!({
            "status": "recorded",
            "kind": "validation-spec",
            "id": spec.id,
            "path": path.to_string_lossy(),
        }))
    }

    #[tool(
        name = "test_record_execution",
        description = "Record (create or overwrite) a validation execution capturing the outcome of a check, linked to tickets/specs/criteria."
    )]
    pub async fn test_record_execution(
        &self,
        Parameters(input): Parameters<RecordExecutionInput>,
    ) -> Result<CallToolResult, McpError> {
        let outcome = Self::parse_outcome(&input.outcome)?;
        let executed_at = Self::parse_timestamp(input.executed_at.as_deref())?;
        let execution = ValidationExecution {
            id: input.id,
            validation_spec_id: input.validation_spec_id,
            outcome,
            executed_at,
            duration_ms: input.duration_ms,
            throughput: input.throughput,
            detail: input.detail,
            links: ValidationLinks {
                spec_ids: input.spec_ids,
                acceptance_criterion_ids: input.acceptance_criterion_ids,
                ticket_ids: input.ticket_ids,
                doc_evidence_ids: input.doc_evidence_ids,
                log_ids: input.log_ids,
            },
            provenance: ValidationProvenance {
                source_path: input.source_path,
                test_id: input.test_id,
                domain: input.domain,
                operation: input.operation,
                transport: input.transport,
                run_id: input.run_id,
            },
        };
        let path = self
            .config_for_write_workspace(&input.workspace)?
            .record_execution(&execution)
            .map_err(Self::test_err)?;
        Self::json_result(&serde_json::json!({
            "status": "recorded",
            "kind": "validation-execution",
            "id": execution.id,
            "outcome": execution.outcome,
            "path": path.to_string_lossy(),
        }))
    }

    #[tool(
        name = "test_get_spec",
        description = "Fetch a single validation spec by id."
    )]
    pub async fn test_get_spec(
        &self,
        Parameters(input): Parameters<GetSpecInput>,
    ) -> Result<CallToolResult, McpError> {
        let configs = self.configs_for_optional_workspace(input.workspace.as_deref())?;
        let mut last_err = TestError::SpecNotFound(input.id.clone());
        for config in &configs {
            match config.get_spec(&input.id) {
                Ok(spec) => return Self::json_result(&spec),
                Err(err) => last_err = err,
            }
        }
        Err(Self::test_err(last_err))
    }

    #[tool(
        name = "test_get_execution",
        description = "Fetch a single validation execution by id."
    )]
    pub async fn test_get_execution(
        &self,
        Parameters(input): Parameters<GetExecutionInput>,
    ) -> Result<CallToolResult, McpError> {
        let configs = self.configs_for_optional_workspace(input.workspace.as_deref())?;
        let mut last_err = TestError::ExecutionNotFound(input.id.clone());
        for config in &configs {
            match config.get_execution(&input.id) {
                Ok(execution) => return Self::json_result(&execution),
                Err(err) => last_err = err,
            }
        }
        Err(Self::test_err(last_err))
    }

    #[tool(
        name = "test_list_specs",
        description = "List all validation specs, sorted by id. Aggregates across every discoverable `.test` store unless `workspace` pins to one."
    )]
    pub async fn test_list_specs(
        &self,
        Parameters(input): Parameters<ListSpecsInput>,
    ) -> Result<CallToolResult, McpError> {
        let configs = self.configs_for_optional_workspace(input.workspace.as_deref())?;
        let mut by_id: std::collections::BTreeMap<String, ValidationSpec> =
            std::collections::BTreeMap::new();
        for config in &configs {
            for spec in config.list_specs().map_err(Self::test_err)? {
                by_id.entry(spec.id.clone()).or_insert(spec);
            }
        }
        let specs: Vec<_> = by_id.into_values().collect();
        Self::json_result(&serde_json::json!({
            "count": specs.len(),
            "specs": specs,
        }))
    }

    #[tool(
        name = "test_list_executions",
        description = "Query validation executions by ticket id, validation spec id, and/or outcome (newest first). Aggregates across every discoverable `.test` store unless `workspace` pins to one."
    )]
    pub async fn test_list_executions(
        &self,
        Parameters(input): Parameters<ListExecutionsInput>,
    ) -> Result<CallToolResult, McpError> {
        let outcome = match input.outcome.as_deref() {
            Some(raw) => Some(Self::parse_outcome(raw)?),
            None => None,
        };
        let sort = Self::parse_sort(input.sort.as_deref())?;
        let query = ExecutionQuery {
            ticket_id: input.ticket_id,
            validation_spec_id: input.validation_spec_id,
            outcome,
            min_duration_ms: input.min_duration_ms,
            max_duration_ms: input.max_duration_ms,
            domain: input.domain,
            operation: input.operation,
            transport: input.transport,
            run_id: input.run_id,
            sort,
            limit: None,
        };
        let configs = self.configs_for_optional_workspace(input.workspace.as_deref())?;
        let mut seen_ids = std::collections::BTreeSet::new();
        let mut executions = Vec::new();
        for config in &configs {
            for execution in config.list_executions(&query).map_err(Self::test_err)? {
                if seen_ids.insert(execution.id.clone()) {
                    executions.push(execution);
                }
            }
        }
        let mut executions = Self::merge_sort_executions(executions, sort);
        if let Some(limit) = input.limit {
            executions.truncate(limit);
        }
        Self::json_result(&serde_json::json!({
            "count": executions.len(),
            "executions": executions,
        }))
    }

    #[tool(
        name = "test_move_preflight",
        description = "Read-only preflight plan for moving a validation spec or execution to another workspace store."
    )]
    pub async fn test_move_preflight(
        &self,
        Parameters(input): Parameters<TestMoveInput>,
    ) -> Result<CallToolResult, McpError> {
        let target_workspace_root =
            memory_kernel::workspace::normalize_explicit_workspace_selector(Some(
                &input.to_workspace_root,
            ))
            .map_err(|err| McpError::invalid_params(err.to_string(), None))?;
        let kind = Self::parse_record_kind(&input.kind)?;
        let config = self.config_for_optional_workspace(input.workspace.as_deref())?;
        let plan = config
            .plan_move_preflight(kind, &input.id, &target_workspace_root)
            .map_err(Self::test_err)?;
        Self::json_result(&serde_json::json!({
            "status": if plan.supported() { "ok" } else { "blocked" },
            "mode": "preflight",
            "kind": input.kind,
            "id": input.id,
            "supported": plan.supported(),
            "blockers": plan.blockers,
        }))
    }

    #[tool(
        name = "test_move_apply",
        description = "Execute a supported validation spec/execution move to another workspace store."
    )]
    pub async fn test_move_apply(
        &self,
        Parameters(input): Parameters<TestMoveInput>,
    ) -> Result<CallToolResult, McpError> {
        let target_workspace_root =
            memory_kernel::workspace::normalize_explicit_workspace_selector(Some(
                &input.to_workspace_root,
            ))
            .map_err(|err| McpError::invalid_params(err.to_string(), None))?;
        let kind = Self::parse_record_kind(&input.kind)?;
        let config = self.config_for_optional_write_workspace(input.workspace.as_deref())?;
        let plan = config
            .plan_move_preflight(kind, &input.id, &target_workspace_root)
            .map_err(Self::test_err)?;
        if !plan.supported() {
            return Err(McpError::invalid_params(
                "move preflight blocked; run test_move_preflight for details".to_string(),
                None,
            ));
        }
        let outcome = config
            .execute_move_with_journal(kind, &plan)
            .map_err(Self::test_err)?;
        Self::json_result(&serde_json::json!({
            "status": "ok",
            "mode": "apply",
            "kind": input.kind,
            "id": input.id,
            "journal_id": outcome.journal.id,
            "phase": outcome.journal.phase,
        }))
    }

    #[tool(
        name = "test_move_resume",
        description = "Resume an interrupted validation spec/execution move from a journal id."
    )]
    pub async fn test_move_resume(
        &self,
        Parameters(input): Parameters<TestMoveJournalInput>,
    ) -> Result<CallToolResult, McpError> {
        let kind = Self::parse_record_kind(&input.kind)?;
        let config = self.config_for_optional_write_workspace(input.workspace.as_deref())?;
        let journal = input.id.parse::<Uuid>().map_err(|error| {
            McpError::invalid_params(format!("invalid journal id: {error}"), None)
        })?;
        let outcome = config
            .resume_move_with_journal(kind, journal)
            .map_err(Self::test_err)?;
        Self::json_result(&serde_json::json!({
            "status": "ok",
            "mode": "resume",
            "journal_id": outcome.journal.id,
            "phase": outcome.journal.phase,
        }))
    }

    #[tool(
        name = "test_move_rollback",
        description = "Roll back a validation spec/execution move from a journal id."
    )]
    pub async fn test_move_rollback(
        &self,
        Parameters(input): Parameters<TestMoveJournalInput>,
    ) -> Result<CallToolResult, McpError> {
        let kind = Self::parse_record_kind(&input.kind)?;
        let config = self.config_for_optional_write_workspace(input.workspace.as_deref())?;
        let journal = input.id.parse::<Uuid>().map_err(|error| {
            McpError::invalid_params(format!("invalid journal id: {error}"), None)
        })?;
        let outcome = config
            .rollback_move_with_journal(kind, journal)
            .map_err(Self::test_err)?;
        Self::json_result(&serde_json::json!({
            "status": "ok",
            "mode": "rollback",
            "journal_id": outcome.journal.id,
            "phase": outcome.journal.phase,
        }))
    }
}

// ── MCP handler trait ─────────────────────────────────────────────────────────

#[tool_handler]
impl ServerHandler for TestServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                ..Default::default()
            },
            instructions: Some(
                "test-mcp provides direct access to the test-result store (test-api). Use named \
                 tools to run stored validation definitions, record validation specs and executions, fetch them by id, and query \
                 executions by ticket/spec/outcome. Link executions to tickets via ticket_ids so \
                 tickets can reference stored evidence instead of inlining results."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

// ── Server startup ────────────────────────────────────────────────────────────

pub async fn run_mcp_server(
    store_root: PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = TestServer::new(store_root);

    tracing::info!("Starting test-mcp server on stdio (direct store access)");

    let service = server.serve(stdio()).await.inspect_err(|err| {
        eprintln!("Server error: {err:?}");
    })?;

    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    fn run_input(workspace: &std::path::Path, command_id: &str, run_id: &str) -> RunInput {
        RunInput {
            workspace: workspace.to_string_lossy().into_owned(),
            spec_id: command_id.into(),
            id: None,
            run_id: Some(run_id.into()),
            throughput: None,
            log_dir: Some(workspace.join("logs")),
            source_path: None,
            test_id: None,
            domain: Some("test".into()),
            operation: Some("run".into()),
            transport: None,
        }
    }

    #[test]
    fn run_schema_rejects_all_definition_overrides_and_unknown_fields() {
        assert!(
            serde_json::from_str::<RunInput>(
                r#"{"workspace":".","spec_id":"first","spec_id":"second"}"#
            )
            .is_err()
        );
        let schema = schemars::schema_for!(RunInput);
        let schema = serde_json::to_value(schema).unwrap();
        assert_eq!(schema["additionalProperties"], false);
        for field in [
            "command",
            "ticket_ids",
            "spec_ids",
            "acceptance_criterion_ids",
            "slow_threshold_ms",
            "unknown",
        ] {
            let mut input = serde_json::json!({"workspace": ".", "spec_id": "vt"});
            input[field] = serde_json::json!("override");
            assert!(
                serde_json::from_value::<RunInput>(input).is_err(),
                "{field}"
            );
            assert!(schema["properties"].get(field).is_none());
        }
    }

    #[tokio::test]
    async fn stored_definition_run_has_cli_mcp_parity() {
        let dir = tempdir().unwrap();
        let config = TestStoreConfig::for_workspace(dir.path());
        let server = TestServer::new(config.root.clone());
        for (spec_id, command, expected) in [
            ("vt-pass", "echo shared-harness", "passed"),
            ("vt-fail", "exit 7", "failed"),
        ] {
            let mut spec = ValidationSpec::new(spec_id, "Parity");
            spec.command = Some(command.into());
            spec.links.ticket_ids = vec!["ticket-owner".into()];
            spec.links.acceptance_criterion_ids = vec!["criterion-owner".into()];
            spec.slow_threshold_ms = Some(0);
            config.record_spec(&spec).unwrap();
            let result = server
                .test_run(Parameters(run_input(dir.path(), spec_id, "mcp")))
                .await
                .unwrap();
            let mcp: serde_json::Value =
                serde_json::from_str(&result.content[0].as_text().unwrap().text).unwrap();
            let cli = test_cli::TestCli {
                json: true,
                toon: false,
                workspace_root: Some(dir.path().into()),
                command: test_cli::TestCommand::Run(test_cli::RunArgs {
                    spec_id: spec_id.into(),
                    id: None,
                    run_id: Some("cli".into()),
                    throughput: None,
                    log_dir: dir.path().join("logs"),
                    source_path: None,
                    test_id: None,
                    domain: Some("test".into()),
                    operation: Some("run".into()),
                    transport: None,
                }),
            };
            let test_cli::CliOutput::Machine(cli, _) = test_cli::run(cli).unwrap() else {
                panic!("expected machine result")
            };
            assert_eq!(mcp["outcome"], expected);
            for field in [
                "status",
                "kind",
                "spec_id",
                "outcome",
                "exit_code",
                "slow_threshold_ms",
                "verified_spec_ids",
                "spec_verification",
            ] {
                assert_eq!(mcp[field], cli[field], "{field}");
            }
            for payload in [&mcp, &cli] {
                let execution = config
                    .get_execution(payload["execution_id"].as_str().unwrap())
                    .unwrap();
                assert_eq!(execution.links.ticket_ids, spec.links.ticket_ids);
                assert_eq!(
                    execution.links.acceptance_criterion_ids,
                    spec.links.acceptance_criterion_ids
                );
                assert!(std::fs::read_to_string(payload["log_locator"].as_str().unwrap()).is_ok());
            }
        }
    }

    #[tokio::test]
    async fn run_rejects_missing_definition_command_and_duplicate_provenance_without_logs() {
        let dir = tempdir().unwrap();
        let config = TestStoreConfig::for_workspace(dir.path());
        let server = TestServer::new(config.root.clone());
        assert!(
            server
                .test_run(Parameters(run_input(dir.path(), "vt-missing", "missing")))
                .await
                .is_err()
        );
        for command in [None, Some(""), Some("  ")] {
            let mut spec = ValidationSpec::new("vt-empty", "Missing command");
            spec.command = command.map(String::from);
            config.record_spec(&spec).unwrap();
            assert!(
                server
                    .test_run(Parameters(run_input(dir.path(), "vt-empty", "empty")))
                    .await
                    .is_err()
            );
        }
        let mut spec = ValidationSpec::new("vt-duplicate", "Duplicate provenance");
        spec.command = Some("echo should-not-run".into());
        spec.links.ticket_ids = vec!["ticket-owner".into()];
        spec.provenance.domain = Some("test".into());
        config.record_spec(&spec).unwrap();
        assert!(
            server
                .test_run(Parameters(run_input(
                    dir.path(),
                    "vt-duplicate",
                    "duplicate"
                )))
                .await
                .is_err()
        );
        assert!(!dir.path().join("logs").exists());
        assert!(
            config
                .list_executions(&ExecutionQuery::default())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn explicit_dot_workspace_uses_absolute_canonical_test_store() {
        let server = TestServer::new(PathBuf::from("."));
        let config = server
            .config_for_write_workspace(".")
            .expect("normalize current workspace");
        let current_dir = std::env::current_dir().unwrap();

        assert!(config.root.is_absolute());
        assert_eq!(
            config.root,
            memory_kernel::workspace::canonical_store_root(&current_dir, ".test")
        );
    }

    #[tokio::test]
    async fn explicit_dot_workspace_creates_and_reads_back_from_canonical_store() {
        const CHILD_ENV: &str = "TEST_MCP_DOT_SELECTOR_CHILD";
        if std::env::var_os(CHILD_ENV).is_none() {
            let temp = tempdir().unwrap();
            let parent = temp.path().join("parent");
            let selected = parent.join("selected");
            let sibling = parent.join("sibling");
            std::fs::create_dir_all(&selected).unwrap();
            std::fs::create_dir_all(&sibling).unwrap();

            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .arg("explicit_dot_workspace_creates_and_reads_back_from_canonical_store")
                .arg("--nocapture")
                .env(CHILD_ENV, "1")
                .current_dir(&selected)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "child regression failed: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );

            let selected_store = TestStoreConfig::for_workspace(&selected);
            let spec = selected_store
                .get_spec("vt-dot-selector")
                .expect("read selected workspace validation spec");
            assert_eq!(spec.title, "Dot selector read-back");
            assert!(
                selected_store
                    .root
                    .join("specs/vt-dot-selector.json")
                    .is_file()
            );
            assert!(!parent.join(".workflow-tools/test").exists());
            assert!(!sibling.join(".workflow-tools/test").exists());
            return;
        }

        let selected = std::env::current_dir().unwrap();
        let server = TestServer::new(selected.parent().unwrap().join("sibling-store"));
        let result = server
            .test_record_spec(Parameters(RecordSpecInput {
                workspace: ".".to_string(),
                id: "vt-dot-selector".to_string(),
                title: "Dot selector read-back".to_string(),
                command: Some("cargo test".to_string()),
                detail: None,
                slow_threshold_ms: None,
                ticket_ids: vec![],
                spec_ids: vec![],
                acceptance_criterion_ids: vec![],
            }))
            .await
            .expect("record in selected workspace");
        assert!(!result.is_error.unwrap_or(false));
    }

    #[tokio::test]
    async fn mcp_producer_reads_back_selected_workspace_without_cross_writes() {
        let parent = tempdir().unwrap();
        let selected = parent.path().join("selected");
        let sibling = parent.path().join("sibling");
        std::fs::create_dir_all(&selected).unwrap();
        std::fs::create_dir_all(&sibling).unwrap();
        let server = TestServer::new(parent.path().join("ambient/.workflow-tools/test"));
        let workspace = selected.display().to_string();

        server
            .test_record_spec(Parameters(RecordSpecInput {
                workspace: workspace.clone(),
                id: "vt-mcp-domain-store".to_string(),
                title: "MCP domain-store fixture".to_string(),
                command: None,
                detail: None,
                slow_threshold_ms: None,
                ticket_ids: vec![],
                spec_ids: vec![],
                acceptance_criterion_ids: vec![],
            }))
            .await
            .expect("record MCP spec");
        server
            .test_record_execution(Parameters(RecordExecutionInput {
                workspace: workspace.clone(),
                id: "exec-mcp-domain-store".to_string(),
                validation_spec_id: "vt-mcp-domain-store".to_string(),
                outcome: "passed".to_string(),
                executed_at: Some("2026-10-07T12:00:00Z".to_string()),
                duration_ms: None,
                throughput: None,
                detail: None,
                ticket_ids: vec!["b36ad4ef-77b9-4225-ab5e-abd0ba13b0b8".to_string()],
                spec_ids: vec![],
                acceptance_criterion_ids: vec![],
                doc_evidence_ids: vec![],
                log_ids: vec![],
                source_path: None,
                test_id: None,
                domain: Some("test-mcp".to_string()),
                operation: Some("record".to_string()),
                transport: Some("in-process".to_string()),
                run_id: Some("fixture".to_string()),
            }))
            .await
            .expect("record MCP execution");

        let spec = server
            .test_get_spec(Parameters(GetSpecInput {
                id: "vt-mcp-domain-store".to_string(),
                workspace: Some(workspace.clone()),
            }))
            .await
            .expect("read MCP spec");
        let execution = server
            .test_get_execution(Parameters(GetExecutionInput {
                id: "exec-mcp-domain-store".to_string(),
                workspace: Some(workspace),
            }))
            .await
            .expect("read MCP execution");

        let spec: serde_json::Value =
            serde_json::from_str(&spec.content[0].as_text().unwrap().text).unwrap();
        let execution: serde_json::Value =
            serde_json::from_str(&execution.content[0].as_text().unwrap().text).unwrap();
        assert_eq!(spec["title"], "MCP domain-store fixture");
        assert_eq!(execution["validation_spec_id"], "vt-mcp-domain-store");
        let store = selected.join(".workflow-tools/test");
        assert!(store.join("specs/vt-mcp-domain-store.json").is_file());
        assert!(
            store
                .join("executions/exec-mcp-domain-store.json")
                .is_file()
        );
        assert!(!parent.path().join(".workflow-tools/test").exists());
        assert!(!sibling.join(".workflow-tools/test").exists());
        assert!(!parent.path().join("ambient/.workflow-tools/test").exists());
    }

    #[tokio::test]
    async fn record_spec_rejects_explicit_legacy_store_path() {
        for legacy_exists in [false, true] {
            let dir = tempdir().unwrap();
            let legacy_store = dir.path().join(".test");
            if legacy_exists {
                std::fs::create_dir_all(&legacy_store).unwrap();
            }
            let canonical_store = dir.path().join(".workflow-tools").join("test");
            let server = TestServer::new(canonical_store.clone());

            let result = server
                .test_record_spec(Parameters(RecordSpecInput {
                    workspace: legacy_store.display().to_string(),
                    id: "vt-legacy-write".to_string(),
                    title: "Legacy override must be rejected".to_string(),
                    command: None,
                    detail: None,
                    slow_threshold_ms: None,
                    ticket_ids: vec![],
                    spec_ids: vec![],
                    acceptance_criterion_ids: vec![],
                }))
                .await;

            assert!(result.is_err());
            assert_eq!(legacy_store.exists(), legacy_exists);
            assert!(!canonical_store.exists());
        }
    }

    #[tokio::test]
    async fn move_targets_reject_ambient_aliases_before_store_access() {
        let dir = tempdir().unwrap();
        let store_root = dir.path().join(".workflow-tools/test");
        let server = TestServer::new(store_root.clone());

        for selector in ["", "  ", "default", ".."] {
            let input = TestMoveInput {
                kind: "spec".to_string(),
                id: "vt-selector".to_string(),
                workspace: None,
                to_workspace_root: selector.to_string(),
            };
            assert!(server.test_move_preflight(Parameters(input)).await.is_err());

            let input = TestMoveInput {
                kind: "spec".to_string(),
                id: "vt-selector".to_string(),
                workspace: None,
                to_workspace_root: selector.to_string(),
            };
            assert!(server.test_move_apply(Parameters(input)).await.is_err());
            assert!(!store_root.exists());
        }
    }

    #[tokio::test]
    async fn get_spec_reads_explicit_legacy_store_when_canonical_is_absent() {
        let dir = tempdir().unwrap();
        let legacy_store = dir.path().join(".test");
        let spec = ValidationSpec {
            id: "vt-legacy-read".to_string(),
            title: "Legacy read remains supported".to_string(),
            command: None,
            detail: None,
            slow_threshold_ms: None,
            links: ValidationLinks {
                spec_ids: vec![],
                acceptance_criterion_ids: vec![],
                ticket_ids: vec![],
                doc_evidence_ids: vec![],
                log_ids: vec![],
            },
            provenance: ValidationProvenance::default(),
        };
        TestStoreConfig::new(legacy_store.clone())
            .record_spec(&spec)
            .unwrap();
        let server = TestServer::new(dir.path().join(".workflow-tools").join("test"));

        let result = server
            .test_get_spec(Parameters(GetSpecInput {
                id: spec.id,
                workspace: Some(legacy_store.display().to_string()),
            }))
            .await
            .expect("read legacy spec");

        assert!(!result.is_error.unwrap_or(false));
    }

    #[tokio::test]
    async fn record_spec_then_execution_and_query() {
        let dir = tempdir().unwrap();
        let store_root = dir.path().join(".workflow-tools").join("test");
        let server = TestServer::new(store_root.clone());

        let spec = server
            .test_record_spec(Parameters(RecordSpecInput {
                workspace: store_root.display().to_string(),
                id: "vt-core-tests".to_string(),
                title: "Core unit tests".to_string(),
                command: Some("cargo test -p ticket-vscode-core".to_string()),
                detail: None,
                slow_threshold_ms: Some(1000),
                ticket_ids: vec!["ticket-parity".to_string()],
                spec_ids: vec![],
                acceptance_criterion_ids: vec![],
            }))
            .await
            .expect("record spec");
        assert!(!spec.is_error.unwrap_or(false));

        let exec = server
            .test_record_execution(Parameters(RecordExecutionInput {
                workspace: store_root.display().to_string(),
                id: "exec-1".to_string(),
                validation_spec_id: "vt-core-tests".to_string(),
                outcome: "passed".to_string(),
                executed_at: Some("2026-06-15T00:00:00Z".to_string()),
                duration_ms: Some(240),
                throughput: Some(4.2),
                detail: Some("16 passed".to_string()),
                ticket_ids: vec!["ticket-parity".to_string()],
                spec_ids: vec![],
                acceptance_criterion_ids: vec![],
                doc_evidence_ids: vec![],
                log_ids: vec![],
                source_path: Some("crates/memory-matrix/tests/matrix.rs".to_string()),
                test_id: Some("ticket.get".to_string()),
                domain: Some("ticket".to_string()),
                operation: Some("get".to_string()),
                transport: Some("in-process".to_string()),
                run_id: Some("run-1".to_string()),
            }))
            .await
            .expect("record execution");
        assert!(!exec.is_error.unwrap_or(false));

        let listed = server
            .test_list_executions(Parameters(ListExecutionsInput {
                workspace: None,
                ticket_id: Some("ticket-parity".to_string()),
                validation_spec_id: None,
                outcome: None,
                min_duration_ms: None,
                max_duration_ms: None,
                domain: Some("ticket".to_string()),
                operation: Some("get".to_string()),
                transport: Some("in-process".to_string()),
                run_id: Some("run-1".to_string()),
                sort: Some("slowest-first".to_string()),
                limit: None,
            }))
            .await
            .expect("list executions");
        assert!(!listed.is_error.unwrap_or(false));

        let specs = server
            .test_list_specs(Parameters(ListSpecsInput { workspace: None }))
            .await
            .expect("list specs");
        assert!(!specs.is_error.unwrap_or(false));
    }

    #[tokio::test]
    async fn invalid_outcome_is_rejected() {
        let dir = tempdir().unwrap();
        let store_root = dir.path().join(".test");
        let server = TestServer::new(store_root.clone());

        let result = server
            .test_record_execution(Parameters(RecordExecutionInput {
                workspace: store_root.display().to_string(),
                id: "exec-bad".to_string(),
                validation_spec_id: "vt-core-tests".to_string(),
                outcome: "nope".to_string(),
                executed_at: None,
                duration_ms: None,
                throughput: None,
                detail: None,
                ticket_ids: vec![],
                spec_ids: vec![],
                acceptance_criterion_ids: vec![],
                doc_evidence_ids: vec![],
                log_ids: vec![],
                source_path: None,
                test_id: None,
                domain: None,
                operation: None,
                transport: None,
                run_id: None,
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn missing_spec_reports_not_found() {
        let dir = tempdir().unwrap();
        let store_root = dir.path().join(".test");
        let server = TestServer::new(store_root);

        let result = server
            .test_get_spec(Parameters(GetSpecInput {
                id: "missing".to_string(),
                workspace: None,
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_executions_aggregates_nested_descendant_store() {
        let dir = tempdir().unwrap();
        let workspace_root = dir.path();
        let root_store = workspace_root.join(".workflow-tools").join("test");
        let nested_workspace = workspace_root.join("nested");
        std::fs::create_dir_all(&nested_workspace).unwrap();

        // Server launched with its fixed root at the workspace root.
        let server = TestServer::new(root_store.clone());

        // Record straight into the root store.
        server
            .test_record_execution(Parameters(RecordExecutionInput {
                workspace: root_store.display().to_string(),
                id: "exec-root".to_string(),
                validation_spec_id: "vt-root".to_string(),
                outcome: "passed".to_string(),
                executed_at: Some("2026-07-13T00:00:00Z".to_string()),
                duration_ms: None,
                throughput: None,
                detail: None,
                ticket_ids: vec!["ticket-x".to_string()],
                spec_ids: vec![],
                acceptance_criterion_ids: vec![],
                doc_evidence_ids: vec![],
                log_ids: vec![],
                source_path: None,
                test_id: None,
                domain: Some("test-mcp".to_string()),
                operation: Some("aggregation-test".to_string()),
                transport: Some("in-process".to_string()),
                run_id: Some("run-root".to_string()),
            }))
            .await
            .expect("record root execution");

        // Record into a nested descendant store via an explicit workspace,
        // mirroring how a submodule-scoped caller records evidence today.
        let nested_store = nested_workspace.join(".workflow-tools").join("test");
        server
            .test_record_execution(Parameters(RecordExecutionInput {
                workspace: nested_store.display().to_string(),
                id: "exec-nested".to_string(),
                validation_spec_id: "vt-nested".to_string(),
                outcome: "passed".to_string(),
                executed_at: Some("2026-07-14T00:00:00Z".to_string()),
                duration_ms: None,
                throughput: None,
                detail: None,
                ticket_ids: vec!["ticket-x".to_string()],
                spec_ids: vec![],
                acceptance_criterion_ids: vec![],
                doc_evidence_ids: vec![],
                log_ids: vec![],
                source_path: None,
                test_id: None,
                domain: Some("test-mcp".to_string()),
                operation: Some("aggregation-test".to_string()),
                transport: Some("in-process".to_string()),
                run_id: Some("run-nested".to_string()),
            }))
            .await
            .expect("record nested execution");

        assert!(nested_store.is_dir());

        // Reading with no explicit workspace must aggregate both stores.
        let result = server
            .test_list_executions(Parameters(ListExecutionsInput {
                workspace: None,
                ticket_id: Some("ticket-x".to_string()),
                validation_spec_id: None,
                outcome: None,
                min_duration_ms: None,
                max_duration_ms: None,
                domain: None,
                operation: None,
                transport: None,
                run_id: None,
                sort: Some("newest-first".to_string()),
                limit: None,
            }))
            .await
            .expect("list executions");
        let text = result.content[0].as_text().unwrap().text.clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["count"], 2);
        let ids: Vec<&str> = parsed["executions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|exec| exec["id"].as_str().unwrap())
            .collect();
        // newest-first: exec-nested (2026-07-14) before exec-root (2026-07-13).
        assert_eq!(ids, vec!["exec-nested", "exec-root"]);

        // A limit is applied after the global merge, not per store.
        let limited = server
            .test_list_executions(Parameters(ListExecutionsInput {
                workspace: None,
                ticket_id: Some("ticket-x".to_string()),
                validation_spec_id: None,
                outcome: None,
                min_duration_ms: None,
                max_duration_ms: None,
                domain: None,
                operation: None,
                transport: None,
                run_id: None,
                sort: Some("newest-first".to_string()),
                limit: Some(1),
            }))
            .await
            .expect("list executions limited");
        let text = limited.content[0].as_text().unwrap().text.clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["count"], 1);
        assert_eq!(parsed["executions"][0]["id"], "exec-nested");

        // Pinning to the nested workspace explicitly must exclude the root.
        let pinned = server
            .test_list_executions(Parameters(ListExecutionsInput {
                workspace: Some(nested_workspace.display().to_string()),
                ticket_id: Some("ticket-x".to_string()),
                validation_spec_id: None,
                outcome: None,
                min_duration_ms: None,
                max_duration_ms: None,
                domain: None,
                operation: None,
                transport: None,
                run_id: None,
                sort: None,
                limit: None,
            }))
            .await
            .expect("list executions pinned");
        let text = pinned.content[0].as_text().unwrap().text.clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["count"], 1);
        assert_eq!(parsed["executions"][0]["id"], "exec-nested");

        // get_execution with no workspace must also find the nested record.
        let fetched = server
            .test_get_execution(Parameters(GetExecutionInput {
                id: "exec-nested".to_string(),
                workspace: None,
            }))
            .await
            .expect("get nested execution");
        assert!(!fetched.is_error.unwrap_or(false));
    }
}
