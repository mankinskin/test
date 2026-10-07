use std::path::PathBuf;

/// Errors produced by the test-result store.
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error("domain store error: {0}")]
    DomainStore(#[from] memory_kernel::domain_store::DomainStoreError),

    #[error("workspace error: {0}")]
    Workspace(#[from] memory_kernel::workspace::ConsumerWorkspaceError),

    #[error("legacy store write override: {0}")]
    LegacyStoreWrite(#[from] memory_kernel::workspace::LegacyStoreWriteOverride),

    #[error("test store root cannot be empty")]
    EmptyRoot,

    #[error("identifier contains invalid path characters: {0}")]
    InvalidId(String),

    #[error("workspace slug contains invalid path characters: {0}")]
    InvalidWorkspaceSlug(String),

    #[error("validation spec not found: {0}")]
    SpecNotFound(String),

    #[error("validation execution not found: {0}")]
    ExecutionNotFound(String),

    #[error("benchmark execution not found: {0}")]
    BenchmarkNotFound(String),

    #[error("interoperability contract violation for {record_kind}: {detail}")]
    InteroperabilityContract { record_kind: String, detail: String },

    #[error("failed to parse budget table {path}: {detail}")]
    BudgetParse { path: PathBuf, detail: String },

    #[error("failed to ingest Criterion estimates {path}: {detail}")]
    CriterionIngest { path: PathBuf, detail: String },

    #[error("failed to serialize test data for {path}: {source}")]
    Serialize {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("failed to deserialize test data from {path}: {source}")]
    Deserialize {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("io error for {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("migration error: {0}")]
    Migration(String),

    #[error("move error: {0}")]
    Move(String),
}
