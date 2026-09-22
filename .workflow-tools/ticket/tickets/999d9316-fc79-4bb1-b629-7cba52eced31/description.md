Migrate ticket-api internal usage to neutral shared APIs and define alias retirement gate.

Scope:
- adopt neutral shared API symbols in ticket-api internals
- retain compatibility where needed for transition period
- define measurable exit criteria for removing legacy aliases

Acceptance criteria:
- ticket-api compiles/tests pass on neutral shared symbols
- phase-E alias retirement gate is recorded and actionable
