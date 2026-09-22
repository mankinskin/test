## Problem

`tools/model-prices/__pycache__/cost_gate.cpython-314.pyc` is tracked in git. Running `test_cost_gate.py` or `sync_model_prices.py` regenerates the bytecode cache, producing a spurious binary diff on every contributor's checkout (observed during review of ticket b0d6bb1c).

## Objective

Remove `tools/model-prices/__pycache__/` from git tracking and ensure `__pycache__/` is covered by `.gitignore` for this path (repo-wide `.gitignore` may already cover `__pycache__` elsewhere but this directory is still tracked — check for a stale exception or a pre-.gitignore commit).

## Acceptance criteria

- `tools/model-prices/__pycache__/` is removed from git tracking (`git rm -r --cached`).
- `.gitignore` covers `__pycache__/` for this path so it does not get re-added.
- Running `test_cost_gate.py` after the fix produces no git diff.

## Context

Found during review of ticket b0d6bb1c ("Extend sync_model_prices.py with GitHub Copilot pricing source"). Not caused by that ticket's changes, but surfaced as noise in its diff.