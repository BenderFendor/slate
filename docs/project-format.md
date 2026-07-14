# Slate project format

## Compatibility contract

Slate project files use the `.slate` suffix and contain UTF-8 JSON. The top-level object is an envelope:

```json
{
  "format": "com.slate.editor.project",
  "version": 1,
  "document": {}
}
```

The `format` marker prevents arbitrary JSON from being accepted as a project. `version` controls compatibility. A reader must reject a project whose version is newer than the version it supports instead of attempting a partial load.

Version 1 serializes the document dimensions, ordered layer stack, active layer identifier, color configuration, revision, current file path, unsaved marker, and optional selection mask. Layers preserve identifiers, names, visibility, opacity, blend mode, optional masks, lock state, and their typed content.

Runtime undo and redo commands are intentionally omitted. Loading a project starts a fresh history stack and marks the document clean.

## Validation

Before a decoded document becomes active, Slate checks:

- project marker and supported version
- nonzero dimensions no larger than 32,768 pixels per side
- no more than 1,024 layers
- unique layer identifiers
- active-layer reference points to an existing layer
- finite opacity in the inclusive `0.0..=1.0` range
- exact `width × height × 4` byte count for raster layers
- exact `width × height` byte count for masks and selections
- checked integer multiplication for every pixel allocation
- project file no larger than 2 GiB

These checks prevent malformed projects from reaching rendering code with inconsistent buffers or invalid references.

## Writing

Slate serializes a validated snapshot, writes it to a hidden sibling temporary file, flushes the file to storage, and then renames it over the destination. An interrupted serialization therefore does not truncate the last successfully saved project.

## Evolution rules

Changes that add optional fields with safe defaults may retain version 1. Changes that alter field meaning, remove required data, or require a migration must increment the project version and add a tested migration path. Unknown future versions must remain a hard error.

Project fixtures intended for regression testing should be small, checked into `tests/fixtures/`, and free of personal image data.
