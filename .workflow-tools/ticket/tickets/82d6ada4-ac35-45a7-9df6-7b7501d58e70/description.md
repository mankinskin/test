Implement URN-based cross-store reference model and resolver APIs.

Scope:
- define canonical URN format ce://workspace/store/entity
- implement parser/formatter/validation
- add resolver interfaces for cross-store lookup

Acceptance criteria:
- parser/formatter tests pass including invalid inputs
- resolver interfaces integrate with store API crates
