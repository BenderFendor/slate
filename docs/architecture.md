# Slate architecture

## Product boundary

Slate is a native Linux desktop application. GTK4 and libadwaita own the interface, Cairo draws the interactive canvas, libvips handles high-quality image processing and export work, Little CMS provides the color-management foundation, and Rust owns document state and editing logic. There is no browser runtime, web server, account system, or remote data layer.

## Document model

`Document` is the mutable source of truth. It contains canvas dimensions, an ordered layer stack, active-layer identity, color configuration, selection state, revision information, current project path, and unsaved state.

Layer data is typed:

- raster layers contain RGBA pixels and offsets
- text and fill layers retain structured parameters
- groups reference child layer identifiers
- adjustment layers contain filter stacks
- every layer may own a grayscale mask

Identifiers are UUID-backed and persist in `.slate` files. Project loading validates their uniqueness and verifies that the active layer exists.

## Commands and history

User-visible mutations should enter through the `Command` trait. The undo stack executes commands, stores them in bounded history, and moves commands between undo and redo stacks. Complex whole-document changes use `ReplaceDocumentCommand`; targeted layer and mask changes use narrower commands.

Paint gestures capture the active layer before the stroke, update pixels interactively, compare the before and after paint state, then commit one `ModifyLayerCommand`. This keeps a full pointer gesture as one undo step instead of one command per brush dab.

## Rendering

The UI never treats the live document as a drawing surface. `tile::snapshot` builds immutable render frames from the current document, resolves masks, opacity, blend modes, offsets, and visibility, and flattens BGRA output for Cairo or export.

The canvas caches a surface by document revision and can update dirty pixel regions during brush work. Overlay drawing is separate from the backing image and handles crop controls, lasso feedback, brush cursors, and edit metadata.

## Image pipeline

`image::pipeline` holds noninteractive crop, resize, rotation, adjustment, and export parameters. Quick Edit controls update this pipeline and previews. Export builds a flattened document frame, converts the channel order expected by the pipeline, applies output operations, and encodes PNG, JPEG, or WebP.

High-quality upscale and selected filters use libvips. Small deterministic pixel transforms remain in Rust where they are easy to test.

## UI composition

`MainWindow` constructs the established application shell:

- header and menu actions
- vertical tool bar
- context-sensitive options bar
- canvas and overlays
- Full Edit and Quick Edit workspaces
- layer, mask, property, and color panels
- status output and command palette

`ui::hardening` is intentionally narrow. It replaces prototype actions whose labels did not match their behavior, adds project/image routing, installs project saving, updates document titles, and audits exposed actions. Keeping this layer separate avoids a risky rewrite of the mature window and canvas code.

## Persistence

`document::project` owns the `.slate` compatibility boundary. It wraps the serialized document in a format/version envelope, validates all pixel buffers and references, excludes runtime undo state, and writes atomically. Save and Save As use this path. Export remains a separate flattened-image operation.

## Packaging and release

The Makefile is the local contract for build, verification, installation, and packaging. Linux integration data lives under `data/`. The packaging script stages an `/usr` tree and creates a deterministic archive plus checksum. GitHub Actions verifies every branch and publishes release archives only from version tags after the same checks pass.

## Invariants for future changes

1. Do not expose a tool or action before it has a complete interaction, history, and render path.
2. Do not mutate image state outside a command unless the gesture is later committed as one command.
3. Do not deserialize pixel buffers without checked dimensions and exact length validation.
4. Keep Save for editable projects and Export for flattened images.
5. Preserve local-only operation unless a network feature is explicit, optional, and documented.
6. Add regression tests for every repaired failure mode.
