//! `test move` — cross-workspace validation-spec/execution move, mirroring
//! the spec/audit move CLI surfaces.

use std::path::Path;

use serde_json::{
    Value,
    json,
};
use test_api::{
    TestRecordKind,
    TestStoreConfig,
};
use uuid::Uuid;

use crate::{
    CliRunError,
    MoveArgs,
};

pub(crate) fn cmd_move(
    args: MoveArgs,
    config: &TestStoreConfig,
) -> Result<Value, CliRunError> {
    if args.resume.is_some() && args.rollback.is_some() {
        return Err(CliRunError::BadRequest(
            "move accepts only one of --resume or --rollback".to_string(),
        ));
    }

    let kind: TestRecordKind = args
        .kind
        .ok_or_else(|| {
            CliRunError::BadRequest(
                "move requires --kind <spec|execution>".to_string(),
            )
        })?
        .into();

    if let Some(journal_id) = args.resume.as_deref() {
        let journal_id = journal_id.parse::<Uuid>().map_err(|error| {
            CliRunError::BadRequest(format!(
                "invalid --resume journal UUID: {error}"
            ))
        })?;
        let outcome = config.resume_move_with_journal(kind, journal_id)?;
        return Ok(json!({
            "command": "move",
            "status": "ok",
            "mode": "resume",
            "outcome": move_outcome_json(&outcome),
            "recovery": recovery_hint(),
        }));
    }

    if let Some(journal_id) = args.rollback.as_deref() {
        let journal_id = journal_id.parse::<Uuid>().map_err(|error| {
            CliRunError::BadRequest(format!(
                "invalid --rollback journal UUID: {error}"
            ))
        })?;
        let outcome = config.rollback_move_with_journal(kind, journal_id)?;
        return Ok(json!({
            "command": "move",
            "status": "ok",
            "mode": "rollback",
            "outcome": move_outcome_json(&outcome),
            "recovery": recovery_hint(),
        }));
    }

    let id = args.id.as_deref().ok_or_else(|| {
        CliRunError::BadRequest(
            "move requires <id> unless --resume/--rollback is used".to_string(),
        )
    })?;
    let to_workspace_root =
        args.to_workspace_root.as_deref().ok_or_else(|| {
            CliRunError::BadRequest(
                "move requires --to-workspace-root in plan/execute mode"
                    .to_string(),
            )
        })?;

    let plan =
        config.plan_move_preflight(kind, id, Path::new(to_workspace_root))?;

    if args.dry_run || !plan.supported() {
        return Ok(json!({
            "command": "move",
            "status": if plan.supported() { "ok" } else { "blocked" },
            "mode": "plan",
            "dry_run": true,
            "kind": kind_label(kind),
            "id": id,
            "plan": move_plan_json(&plan),
            "recovery": recovery_hint(),
        }));
    }

    let outcome = config.execute_move_with_journal(kind, &plan)?;
    Ok(json!({
        "command": "move",
        "status": "ok",
        "mode": "execute",
        "kind": kind_label(kind),
        "id": id,
        "plan": move_plan_json(&plan),
        "outcome": move_outcome_json(&outcome),
        "recovery": recovery_hint(),
    }))
}

fn kind_label(kind: TestRecordKind) -> &'static str {
    match kind {
        TestRecordKind::Spec => "spec",
        TestRecordKind::Execution => "execution",
    }
}

fn move_plan_json(
    plan: &memory_kernel::storage::move_kernel::MovePlan
) -> Value {
    json!({
        "supported": plan.supported(),
        "source_workspace_root": disp(&plan.source_workspace_root),
        "target_workspace_root": disp(&plan.target_workspace_root),
        "blockers": plan.blockers,
        "captured_at": plan.captured_at,
    })
}

fn move_outcome_json(
    outcome: &memory_kernel::storage::move_kernel::MoveOutcome
) -> Value {
    json!({
        "resumed": outcome.resumed,
        "rolled_back": outcome.rolled_back,
        "journal_id": outcome.journal.id,
        "phase": outcome.journal.phase,
    })
}

fn recovery_hint() -> Value {
    json!({
        "resume": "test move --kind <spec|execution> --resume <journal-uuid>",
        "rollback": "test move --kind <spec|execution> --rollback <journal-uuid>",
    })
}

fn disp(path: &std::path::Path) -> String {
    memory_kernel::workspace::normalize_path_for_display(path)
}
