# Slate Editor Hardening Plan

## Goal

Turn Slate from a promising prototype into a credible native GTK4 editor shell:

- every visible control has real behavior
- the first screen reads like a serious editor, not a demo
- painting, panning, zooming, and layer work stay responsive on normal images
- the command model is stable enough to support a Photoshop-style shortcut profile
- screenshots, logs, and tests can prove progress after each pass

## Baseline Rules

- Do not expose tools that are not implemented enough to use.
- Do not keep placeholder buttons or decorative controls in the shipping UI.
- Keep crop, resize, and export as separate workflows.
- Keep renderer decisions evidence-based. Measure before replacing Cairo with another path.
- Treat user-visible lag, dead menu items, stale panels, and misleading status text as product bugs.

## Phase 1: Reproduction And Instrumentation

### Deliverables

- Keep `scripts/capture-ui.sh` as the repeatable screenshot path.
- Maintain a screenshot set for:
  - empty state
  - loaded image
  - brush active
  - crop active
  - multi-layer document
- Add lightweight timing points for:
  - image open
  - full flatten
  - brush dab/stroke mutation
  - redraw preparation
- Keep a warning log from a live run that includes image open, brush use, crop use, and layer actions.

### Tests

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- Manual smoke:
  - launch empty
  - open image from CLI and dialog
  - capture screenshots
  - confirm no dead controls are visible

### Exit Criteria

- We can reproduce every reported issue on demand.
- Screenshots and logs are captured from the current binary, not old runs.
- The app has no known misleading diagnostics such as a fake renderer label.

## Phase 2: Command Spine And Action Coverage

### Deliverables

- Keep all editor commands in one discoverable action layer.
- Inventory every visible menu item, toolbar button, layer-panel button, and shortcut.
- For each visible control, choose one of:
  - fully wire it
  - disable it with a truthful reason
  - remove it until the feature exists
- Make menus and panel buttons invoke the same command path.
- Ensure command history, undo, redo, cut, copy, paste, duplicate, and delete all use shared behavior.

### Tests

- Add unit coverage for command round trips where missing.
- Add a control inventory test or helper that asserts menu actions exist.
- Manual smoke:
  - each visible action fires without warnings
  - undo/redo restores document state
  - copy/cut/paste/duplicate operate on active layers

### Exit Criteria

- No visible control logs `Action does not exist`.
- The menus never present a fake capability.
- History reflects real executed commands instead of static sample rows.

## Phase 3: Keyboard Model

### Deliverables

- Introduce a centralized shortcut registry instead of scattering accelerator setup.
- Ship one default profile biased toward Photoshop conventions:
  - `Ctrl+N`, `Ctrl+O`, `Ctrl+S`, `Ctrl+Shift+S`
  - `Ctrl+Z`, `Ctrl+Shift+Z`, `Ctrl+Y`
  - `Ctrl+X`, `Ctrl+C`, `Ctrl+V`
  - `V`, `C`, `B`, `E`, `Z`
  - `Ctrl+0`, `Ctrl++`, `Ctrl+-`
- Document where Slate intentionally differs from GIMP or Photoshop.
- Keep the shortcut dialog generated from the same registry.
- Leave room for a later editable keymap profile without rewriting actions.

### Tests

- Unit-test action-to-shortcut registry entries.
- Manual smoke:
  - every displayed shortcut works
  - no duplicate shortcut silently shadows another command
  - shortcut dialog matches runtime behavior

### Exit Criteria

- There is one source of truth for default accelerators.
- Common editor muscle memory works without hidden exceptions.

## Phase 4: Canvas And Brush Performance

### Deliverables

- Keep the current dirty-region recomposition path for brush edits.
- Stop rebuilding non-pixel UI, such as layer rows, on every brush-motion revision.
- Measure Cairo full redraw cost and brush stroke cost on at least:
  - 1920x1080 single-layer raster
  - 4K single-layer raster
  - multi-layer document
- Decide the next renderer from evidence:
  - remain on Cairo plus cache improvements if frame cost is acceptable
  - or build a real GPU path with a persistent texture and dirty uploads if it is not
- If GPU work is justified, implement a real renderer. Do not add another decorative GL probe.
- Consider brush-stamp caching if dab math becomes the next hotspot after redraw work is fixed.

### Tests

- Keep pixel-correctness tests for dirty-region flattening.
- Add benchmark-style measurement scripts or repeatable timing logs.
- Manual smoke:
  - long diagonal brush strokes are continuous, not dotted
  - brush input remains visually attached to the pointer
  - panning and zooming remain smooth while a large image is open

### Exit Criteria

- The slow path is measured, not assumed.
- Brush interaction is smooth enough that the next bottleneck is known.
- Any GL path used by Slate is a real renderer with fallback behavior, not a status guess.

## Phase 5: Layers, Masks, And Properties

### Deliverables

- Make empty, single-layer, and multi-layer states read correctly.
- Disable layer-specific controls when there is no active layer.
- Keep layer selection, properties, visibility, opacity, mask state, and history synchronized.
- Make masks visibly understandable in both the layer row and properties panel.
- Add practical layer operations expected from a real editor:
  - add
  - delete
  - duplicate
  - group only when group behavior exists
  - mask add/remove/apply/toggle

### Tests

- Add unit tests for mask and layer command round trips.
- Manual smoke:
  - no active layer means no active layer controls
  - selecting layers updates properties immediately
  - mask toggles redraw the canvas and the panel consistently

### Exit Criteria

- The right rail never lies about current document state.
- Core layer workflows are usable without reading logs.

## Phase 6: Crop And Transform Workflow

### Deliverables

- Keep crop as a dedicated tool flow.
- Remove resize, upscaling, or export concerns from crop UI.
- Support:
  - free crop
  - common aspect presets
  - direct dimensions when appropriate
  - apply, cancel, reset
  - keyboard completion and escape
- Keep crop handles, hit testing, and overlay readable at all zoom levels.

### Tests

- Keep crop hit and normalization unit tests.
- Add tests for apply/cancel behavior and bounds clamping.
- Manual smoke:
  - crop creation
  - move
  - each edge/corner resize
  - apply
  - cancel
  - aspect preset switch

### Exit Criteria

- Crop feels like one focused operation, not a mixed export panel.
- Its result is predictable from the overlay.

## Phase 7: Professional UI Pass

### Deliverables

- Keep real app-owned toolbar icons for every exposed tool.
- Improve hierarchy among:
  - menubar
  - headerbar
  - options bar
  - canvas
  - right rail
  - status bar
- Keep text compact in tool surfaces and panels.
- Replace dead air with useful states:
  - empty document
  - no layers
  - no history
- Make filenames and labels truncate intentionally instead of being clipped.
- Use the system/libadwaita look as the base instead of fighting it with a custom pseudo-theme.

### Screenshot Review Checklist

- First screenshot tells the user what to do next.
- Toolbar symbols are distinct at a glance.
- Active tool and active layer are obvious.
- Canvas remains the dominant work surface.
- The right rail is dense but not cramped.
- No text overlaps or clips.
- The app still looks coherent at narrower window widths.

### Exit Criteria

- A fresh screenshot reads as a professional desktop editor without explanation.
- No visible area looks like placeholder UI.

## Phase 8: Release Gate

### Automated Gate

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

### Manual Gate

- Launch empty state.
- Open image from menu, button, drag/drop, and CLI argument.
- Exercise:
  - tool switching
  - brush stroke
  - eraser
  - crop apply/cancel
  - layer add/delete/duplicate
  - mask add/remove/apply
  - undo/redo
  - zoom and fit-to-screen
- Capture current screenshots for empty, loaded, brush, crop, and layers.
- Review logs for GTK warnings and app warnings.

### Exit Criteria

- No known critical runtime warning remains unexplained.
- Every visible workflow has at least one direct verification path.
- The strongest available test gate passes on the current tree.

## Current Work Order

1. Finish the loaded-image warning investigation.
2. Keep reducing brush latency by removing avoidable work from the live stroke path.
3. Centralize the shortcut registry and generate the shortcut dialog from it.
4. Audit the remaining visible controls and remove or finish anything decorative.
5. Continue screenshot-driven visual passes after each behavioral pass.
