use std::path::{Path, PathBuf};

use memory_kernel::workspace::{ConsumerWorkspaceError, resolve_consumer_store_root_from};
use test_mcp::run_mcp_server;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("test_mcp=info".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    let store_root = resolve_store_root();

    eprintln!("test-mcp starting (store: {})", store_root.display());

    if let Err(err) = run_mcp_server(store_root).await {
        eprintln!("Fatal error: {err}");
        std::process::exit(1);
    }
}

fn resolve_store_root() -> PathBuf {
    if let Ok(path) = std::env::var("TEST_STORE_ROOT") {
        return PathBuf::from(path);
    }
    let cwd = memory_kernel::workspace::working_dir();
    resolve_startup_anchor(cwd.as_deref())
}

/// Read tools aggregate every discoverable `.test` store, so an ambiguous
/// superproject only needs a deterministic anchor, not a fatal exit.
fn resolve_startup_anchor(cwd: Option<&Path>) -> PathBuf {
    match resolve_consumer_store_root_from(None, None, None, cwd, ".test") {
        Ok(store_root) => store_root,
        Err(ConsumerWorkspaceError::AmbiguousSuperproject { workspace, stores }) => {
            let anchor = stores
                .first()
                .cloned()
                .unwrap_or_else(|| workspace.join(".workflow-tools").join("test"));
            eprintln!(
                "test-mcp: {} contains {} .test stores; anchoring at {}",
                workspace.display(),
                stores.len(),
                anchor.display()
            );
            anchor
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ambiguous_superproject_uses_deterministic_store_anchor() {
        let dir = tempfile::tempdir().unwrap();
        let superproject = dir.path().join("meta-workspace");
        let first = superproject.join("a").join(".workflow-tools").join("test");
        let second = superproject.join("b").join(".workflow-tools").join("test");
        std::fs::create_dir_all(&second).unwrap();
        std::fs::create_dir_all(&first).unwrap();

        assert!(
            resolve_consumer_store_root_from(None, None, None, Some(&superproject), ".test")
                .is_err()
        );
        assert_eq!(resolve_startup_anchor(Some(&superproject)), first);
    }
}
