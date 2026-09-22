## Goal
Decouple SVG edge-overlay geometry from the DOM layout pass so edges and node cards are positioned from the same projection source.

## Scope
File: `viewer-api/viewer-api/frontend/dioxus/src/graph3d/render.rs` (`position_dom_edges`, render.rs:724)

Change:
- Anchor SVG edge endpoints to the `world_to_screen` node-center projections already computed for node positioning, rather than reading node `getBoundingClientRect` values (`clip_edge_endpoint` fed by `rect_a`/`rect_b`). Retain endpoint clipping to card bounds using node screen position + known card width/height instead of a live reflow.
- This removes the remaining `getBoundingClientRect`-driven forced reflow in the edge pass.

## Dependency
Builds on the shared-VP/shared-rects refactor (sibling ticket). Coordinate so both land consistently.

## Acceptance Criteria
- [ ] SVG edge endpoints derived from `world_to_screen` node centers, not bounding rects
- [ ] Edge endpoints remain visually clipped to node card borders
- [ ] No `getBoundingClientRect` call inside `position_dom_edges`
- [ ] Edges stay attached to node cards during camera/drag with no trailing

## Validation
- wasm-pack graph3d tests
- Playwright orbit/drag screenshots in external Chromium