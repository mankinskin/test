# Problem

Ticket state transitions are currently constrained by schema edges that only cover the forward workflow path in practice, which prevents moving a ticket back to an earlier valid state through the same `update` / `close` interface.

# Scope

Expand the ticket state transition schema so tickets can move backward to any schema-allowed prior state using the same validated transition path machinery.

# Acceptance Criteria

- The ticket schema includes reverse edges for supported workflow states.
- `ticket update` can move a ticket back to an earlier valid state when the schema allows it.
- Transition validation continues to reject edges that are not declared in the schema.
- Focused ticket-api regression coverage proves at least one backward transition through the public store API.