1. Resolve the linked spec store using the selected workspace and the canonical workspace-resolution helper; do not infer it by appending `.spec` to the test-store parent.
2. Add a regression fixture where canonical and legacy spec stores contain conflicting states, and prove that test-cli reads and updates only the canonical store.
3. Make guard recomputation safe to repeat after a spec has already reached `verified`.
4. Preserve the canonical W3 spec and its history; do not apply a state change as a workaround for the wrong-root lookup.