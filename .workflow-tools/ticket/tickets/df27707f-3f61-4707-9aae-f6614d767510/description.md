# Order-sensitive API assertion rule

Extend `.agents/instructions/testing/assertions.instructions.md`, whose only heading is `Assertions` at line 5, with a rule for APIs whose contract includes ordering. Tests must use stable ordered fixtures and assert the returned order explicitly. Tests must not use random identifiers such as v4 UUIDs where ordering is the contract, because a sorted container can then appear correct by accident.

The rule is needed because a `BTreeSet` silently sorted v4 UUIDs in board aggregation even though the required result order was first-seen insertion order. The random identifiers made the sorted output indistinguishable from the intended behaviour during the original test.

The exact target is `.agents/instructions/testing/assertions.instructions.md`; the rule is assertion design rather than test invocation strategy, so `.agents/instructions/testing/test-execution.instructions.md` is not the correct home.