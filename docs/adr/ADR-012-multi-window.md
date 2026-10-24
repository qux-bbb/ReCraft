<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# ADR-012: Detached panes — in-app floating windows now, real OS windows later

- Status: Accepted (spike complete, implementation deferred)
- Date: 2026-10-24
- Related: `docs/M4.md`, `docs/ROADMAP.md`, `examples/viewport_spike.rs`

## Context

The IDA-style workspace uses `egui_dock`. Dragging a tab out of the dock produces an
`egui::Window` inside the same OS window (`TabViewer::allowed_in_windows`). IDA allows a
view to become a real top-level window that can be moved to another monitor or appear in
the taskbar. We need to know whether the pinned Glow renderer and egui 0.36 can support
that before investing in the UI rework.

## Spike

`crates/recraft-ui/examples/viewport_spike.rs` opens a second viewport with
`Context::show_viewport_immediate` + `ViewportBuilder`. Running it on Windows 10 with the
pinned Glow backend:

```
titles: "ReCraft viewport spike - main" | "ReCraft detached pane (spike)" | ...
second viewport class: Immediate
```

- Two native top-level windows exist, each with its own title/frame and taskbar entry.
- The child viewport reports `ViewportClass::Immediate`, i.e. the Glow integration
  installed the immediate-viewport renderer — not the `EmbeddedWindow` fallback.
- The detached window can be raised to the foreground and captured independently
  (`docs/images/detached-spike.png`).
- Immediate viewports render inside the parent update loop, so the pane callback can
  borrow app state directly (no `Arc<Mutex<…>>` refactor needed for a prototype).

## Options considered

1. **Keep egui_dock's in-app floating windows only** (status quo). Cheap, but the window
   cannot leave the main OS window or be maximized independently.
2. **Promote popped-out tabs to immediate OS viewports** (spike proves feasibility).
   Requires stable viewport ids per pane, removing the pane from `DockState` while
   detached, re-docking on window close, rendering each detached pane's content exactly
   once, and persisting detached panes/window geometry.
3. **Deferred OS viewports** (`show_viewport_deferred`): better multi-window performance,
   but the callback is `Send + Sync + 'static`; our app state (SQLite session, jobs) is not
   structured for that today, so this would be a much larger refactor than option 2.

## Decision

Ship v0.1 with **option 1**: dragged-out tabs remain floating windows inside the main
window. Implement **option 2** after the preview as a dedicated feature, starting from the
checked-in spike. Deferred viewports (option 3) are not planned before M5.

Open work for option 2, in order:

1. `DetachedPane` map: pane → stable `ViewportId`; remove the tab from `DockState` on
   detach and re-add on close/re-dock (reuse `focus_pane`).
2. Render a detached pane through the existing `DockViewer` content methods, with a
   `TabViewer`-compatible wrapper that does not require a dock tab.
3. Window lifecycle: `close_requested`, minimize/restore, focus (keyboard shortcuts must
   target the focused viewport), title = pane name + project.
4. Persistence: which panes are detached and their outer positions/sizes in
   `SavedLayout` (version bump), restoring on startup without flash.
5. Per-monitor DPI: verify Glow viewport scaling on mixed-DPI setups.
6. Only then make it the default drag-out behaviour; keep in-app floating as fallback.

## Consequences

- No user-visible change in v0.1; documented limitation ("floating tabs are in-app, not
  OS windows").
- The spike example stays in the repo (and in CI's `--all-targets` build) so the
  feasibility result is reproducible.
