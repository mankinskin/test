## Problem

Node cards still trail GPU-drawn edges during orbit/pan/drag. Root cause confirmed by reading graph3d/render.rs: the per-frame render loop performs read-after-write DOM layout thrashing.

Per frame render_frame does:
1. position_dom_nodes writes transform/width/height/display on every [data-node-idx] card.
2. collect_dom_node_rects (render.rs ~220) then calls getBoundingClientRect() per node — a forced synchronous reflow immediately after the writes.
3. sync_node_detail_tier runs a nested querySelectorAll per node and rewrites detail-child display every frame even when the LOD tier did not change.

The GPU edge pass presents immediately; the DOM cards stall behind the forced reflow. That gap is the visible lag.

## Fix (contained to render.rs)

1. Replace collect_dom_node_rects (DOM read) with an analytic compute_node_screen_rects that derives each NodeScreenRect from the projected layout using the same math position_dom_edges already uses (card_w * pixel_scale * 0.5). No getBoundingClientRect -> no forced reflow.
2. Guard sync_node_detail_tier to skip the nested query + writes when the node's data-node-lod attribute already equals the target tier.

## Non-goals

- No change to GPU edge pipeline (already buffer-driven).
- No change to the Rc<RefCell> + rAF interaction decoupling.
- Anchor-pass offsetWidth reads (layout-anchor elements only, absent in default force layout) are out of scope for this slice.

## Acceptance criteria

- render_frame performs zero per-node getBoundingClientRect/offsetWidth reads on the node-card path.
- Detail-tier DOM writes only occur when a node's LOD tier changes.
- wasm build compiles for the dioxus frontend.
- Manual external-Chromium validation: orbit + drag with node cards staying locked to edge endpoints; drag/orbit screenshots captured.

## Parent

Umbrella: 111510f4-c74b-4819-800b-d68ab013a73c