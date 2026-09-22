## Goal
Remove the redundant per-frame work in `render_frame` that thrashes layout and inflates the rAF callback, causing DOM node cards to lag behind GPU edges.

## Scope
File: `viewer-api/viewer-api/frontend/dioxus/src/graph3d/render.rs`

Changes:
- Compute the view-projection matrix ONCE in `render_frame` and pass it into `position_dom_nodes` (render.rs:570), `position_dom_layout_anchors` (render.rs:388), and `position_dom_edges` (render.rs:724) instead of each recomputing it.
- Call `collect_dom_node_rects` (render.rs:220) at most ONCE per frame and share the resulting rects between `position_dom_layout_anchors` (render.rs:413) and `position_dom_edges` (render.rs:776), eliminating the second forced synchronous reflow.
- Keep all DOM style writes batched and separated from rect reads to avoid write→read→write thrashing.

## Acceptance Criteria
- [ ] VP matrix computed once per frame, reused by GPU + all DOM helpers
- [ ] `collect_dom_node_rects` invoked at most once per frame
- [ ] No behavioral change to node/anchor/edge positions other than reduced latency
- [ ] `cargo check --target wasm32-unknown-unknown -p viewer-api-dioxus` passes

## Validation
- wasm-pack graph3d tests for viewer-api
- Playwright drag/orbit in external Chromium: node cards track edges without trailing