# Mission Review Agent manual walkthrough

This walkthrough evaluates the [Mission Review Agent](../../../.agents/agents/mission-review.agent.md)
against the [generated review-entry contract](../../../mission/README.md#generated-review-entry-and-verification)
and the approved [Mission reviewer-role Spec](../../../.workflow-tools/spec/specs/d6899bdd-657c-4f50-8475-8930d39b276c/body.md).
It is scoped to [Ticket bf85445e](../../../.workflow-tools/ticket/tickets/bf85445e-a6a2-4a64-9d9d-a2cf3ade5460/ticket.toml).
The registered gate is [`w8-mission-reviewer-role`](../../../.workflow-tools/test/specs/w8-mission-reviewer-role.json),
which targets `mission-review-role-evidence-citations` and
`mission-review-role-audit-compatibility`.

All records under [fixtures](fixtures/) are synthetic, self-contained examples.
This is role walkthrough evidence only: no real Mission was read or changed,
no Mission command or product test was executed, and no product acceptance or
readiness claim follows from these examples.
Each `accepted.json` uses the strict `AcceptedMission` envelope
(`mission_id`, `bundle`, `label_mapping`), with a schema-versioned manifest,
typed waypoint, and matching revision/label mapping. The complete fixture's
identity, revision, objective, waypoint, evidence reference, and receipt align
across its authority, entry, roadmap, Part, and receipt; the two negative
fixtures intentionally retain their documented artifact gap or result
contradiction.

## Review results

The strict fixture shape was checked against `AcceptedMission`,
`MissionBundle`, `BundleManifest`, `WaypointFragment`, `LabelMapping`, and the
related types in `mission-api/src/lib.rs`: required fields are present,
unrecognized-field strictness is respected, and the fixture values use the
declared UUID, schema-version, enum, relationship, and revision representations.
This is structural fixture validation, not a product test or a claim that the
synthetic records were published by Mission.

### Complete entry

**Input:** [complete generated entry](fixtures/complete/generated/README.md).

- **Disposition:** `complete`; no missing or contradictory fixture artifacts
  were found.
- **Evidence:** the entry's Promise matches the accepted Mission objective
  and both report revision 1
  ([entry](fixtures/complete/generated/README.md#promise),
  [strict accepted Mission](fixtures/complete/accepted.json)); its schema
  version is supported and its W1 label mapping resolves to the present Part;
  the roadmap and W1 Part are reachable
  ([roadmap](fixtures/complete/generated/ROADMAP.md),
  [Part](fixtures/complete/generated/01-review.md)); and the declared receipt
  link resolves
  ([entry](fixtures/complete/generated/README.md#declared-evidence),
  [receipt](fixtures/complete/evidence/receipt.md)).
- **Residual unknown:** the linked `passed` value is explicitly synthetic, not
  a registered product execution. Product readiness and historical acceptance
  remain `unknown`; the role does not claim either.

### Missing artifacts and unavailable reference

**Input:** [missing-artifact generated entry](fixtures/missing/generated/README.md).

- **Disposition:** `findings`; the entry itself is structurally complete.
- **Missing:** the entry and roadmap declare W1 and a validation receipt, but
  both targets are absent:
  [entry inventory](fixtures/missing/generated/README.md#delivery),
  [roadmap requirements](fixtures/missing/generated/ROADMAP.md#requirements),
  missing `fixtures/missing/generated/01-review.md`, and missing
  `fixtures/missing/evidence/receipt.md`.
- The strict accepted Mission's revision 3 and objective match the entry and
  roadmap; its `bundle.manifest.artifacts` retains the missing receipt and
  unavailable report references as authority data, not evidence that those
  files exist.
- **Unavailable:** the entry explicitly marks
  `unavailable-report.md: unavailable
  (mission-reference-context-unavailable)` under Declared evidence. The role
  preserves this as `unavailable`, not as the same finding as the absent W1
  and receipt.
- **Residual unknown:** no validation outcome is available, so the promised
  result and product readiness remain `unknown`.

The [incomplete-entry variant](fixtures/missing/generated/incomplete-entry.md)
is a separate blocked-input probe. It lacks the required generated inventory,
Declared evidence, Verification, and External status. The role returns
`blocked` at the entry-completeness check and does not use a linked Roadmap or
other Mission records to reconstruct those omissions.

### Contradictory evidence

**Input:** [contradictory generated entry](fixtures/contradictory/generated/README.md).

- **Disposition:** `findings`; the entry and all declared fixture paths exist.
- **Contradictory:** the entry's Verification says the gate passed
  ([entry](fixtures/contradictory/generated/README.md#verification)), while the
  strict accepted Mission's `operator_results` and linked receipt record
  `failed`
  ([receipt](fixtures/contradictory/evidence/receipt.md)); the roadmap says a
  failed outcome does not satisfy the gate
  ([roadmap](fixtures/contradictory/generated/ROADMAP.md#validation-gate)).
- **Residual unknown:** the fixture cannot establish real product acceptance.
  The recorded failure contradicts the entry's claim; no readiness claim is
  made.

## Compatibility and boundary

The existing [Audit Agent](../../../audit/.agents/agents/audit.agent.md)
remains present and unmodified; this role has no audit execution or mutation
grant. The Mission Review Agent reports only its cited Mission evidence.

The walkthrough covers the registered evidence-citation and audit-compatibility
criteria. It is not a product test, Mission acceptance, or a substitute for
review of this Ticket.
