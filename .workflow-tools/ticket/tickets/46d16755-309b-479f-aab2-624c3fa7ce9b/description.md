Implemented and validated canonical workspace resolution for ticket-vscode. The extension now maps detected local .ticket roots to canonical server workspace ids by label/path and otherwise prefers active_workspace over the first returned workspace.

Acceptance criteria:
- When the open VS Code folder contains a local .ticket root, the extension resolves the matching server workspace instead of defaulting to the first server-reported workspace.
- When the server exposes canonical ids such as path-based names or shared-- ids, the extension uses server labels and/or local .ticket paths to preserve the intended selection.
- Unit coverage exercises the mismatch case so the extension does not regress to listing tickets from a sibling or descendant workspace.

Validation:
- npm run compile
- npm run test:unit -- --runInBand --runTestsByPath test/unit/resolveServerLaunch.test.ts