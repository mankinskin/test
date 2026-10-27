1. With a canonical `.workflow-tools/test` store and canonical `.workflow-tools/spec` store, test-cli resolves the same spec store as the spec CLI; it never redirects to `.workflow-tools/.spec`.
2. A regression fixture with conflicting state in a legacy `.spec` index proves verification uses the canonical manifest and does not mutate the legacy store.
3. Passing all declared guards transitions an `implemented` spec to `verified` through the canonical store.
4. Re-running recomputation for an already-verified spec returns a successful verified outcome without a `verified -> verified` transition error.
5. The regression tests pass without changing the unrelated W3 repair implementation.