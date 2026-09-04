---
description: "Use when running HTTP stress tests, concurrency sweeps, or deploying ticket-viewer for HTTP-level validation."
---

## HTTP-Level Stress Testing

`tools/http/stress_graph.py` — concurrency sweep (phases 1–3, with optional soak):

```bash
python tools/http/stress_graph.py                    # default workspace, depth=4
python tools/http/stress_graph.py --base-url http://127.0.0.1:3002 --depth 4
```

`tools/http/bench2.py` — verbose single-run timing including server-side phase breakdown from the `stats` field in the response body.

**Windows note**: always use `127.0.0.1` (not `localhost`) in `--base-url`. Windows resolves `localhost` to IPv6 (`::1`) first; the server only binds IPv4, causing ~2s connection timeout per request before fallback.

## Deploying ticket-viewer for HTTP Testing

```bash
# Build the binary (must build the viewer, not just the library)
cargo build -p ticket-viewer --release

# Deploy and restart
viewer-ctl stop ticket-viewer
viewer-ctl install ticket-viewer
viewer-ctl start ticket-viewer
```

The binary is `~/.cargo/bin/ticket-viewer.exe`. Building only `-p ticket-http` produces
the library but not the binary; the server will be stale until the viewer crate is rebuilt.
