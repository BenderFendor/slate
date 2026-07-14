# Slate

Slate is a native Linux raster image editor built with Rust, GTK4, libadwaita, libvips, and Little CMS. It is designed as a focused desktop editor rather than an Electron or Tauri wrapper: image decoding, compositing, brush work, filters, project persistence, and export all run locally.

The repository originally contained a substantial but unfinished editor behind a one-line work-in-progress README. The current v1 hardens that native application into a maintainable product with layered project files, truthful Save versus Export behavior, automated tests, CI, desktop integration, and release packaging.

## Implemented features

- Raster layer stack with selection, visibility, opacity, blend modes, locking, duplication, removal, reordering, merge-down, cut, copy, and paste
- Layer masks with add, remove, apply, edit, enable/disable, and canvas-preview controls
- Brush and eraser strokes with size, hardness, opacity, flow, color, interpolation, selection clipping, and undoable stroke commits
- Move, crop, lasso, color-picker, and zoom tools
- Crop handles, move/resize interactions, keyboard confirmation/cancellation, and output preview information
- Undo and redo command stack for document, layer, mask, transform, filter, and paint operations
- Resize image, resize canvas, upscale, rotate, horizontal/vertical flip, blur, sharpen, noise, invert, and grayscale actions
- Quick Edit, Full Edit, and canvas-only workspaces
- Flattened PNG, JPEG, and WebP export through the image pipeline
- Versioned `.slate` project files that preserve layers, masks, selections, color configuration, and editable state
- Validated project loading, bounded project-file size, malformed-pixel rejection, and atomic project writes
- Open images through the file dialog, drag and drop, or command line; open `.slate` projects through File/Open or the command line
- Native desktop entry, AppStream metadata, MIME registration, icons, install targets, release archives, CI, and tag-driven GitHub releases
- No accounts, telemetry, mock data, cloud upload, or external runtime service

## Build requirements

Slate targets a current Linux desktop with:

- Rust stable and Cargo
- GTK 4.14 or newer development files
- libadwaita development files
- libvips development files
- Little CMS 2 development files
- Cairo, Pango, pkg-config, and a C build toolchain

The CI workflow documents the exact Ubuntu package set used by the project:

```bash
sudo apt-get install \
  build-essential pkg-config \
  libgtk-4-dev libadwaita-1-dev \
  libvips-dev liblcms2-dev \
  libcairo2-dev libpango1.0-dev
```

## Run from source

```bash
cargo run --locked
```

Open an image or project directly:

```bash
cargo run --locked -- /path/to/image.png
cargo run --locked -- /path/to/project.slate
```

## Verify the repository

```bash
make check
```

That runs:

```bash
cargo fmt --check
cargo test --all-targets --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
```

The test suite covers command undo/redo, masks, project encoding and validation, rendering/compositing helpers, transforms, filters, crop behavior, action inventories, and export metadata. GitHub Actions repeats the same checks on pushes and pull requests.

## Install locally

Build and install under `/usr/local`:

```bash
sudo make install
```

Install under `/usr` for a staged package:

```bash
make install DESTDIR="$PWD/pkgroot" PREFIX=/usr
```

Uninstall files installed by the Makefile:

```bash
sudo make uninstall
```

## Build a release archive

```bash
make package
```

This creates a reproducible archive and SHA-256 checksum in `dist/` containing:

- `/usr/bin/slate`
- desktop launcher
- AppStream metadata
- `.slate` MIME registration
- application and tool icons

Pushing a tag such as `v0.1.0` runs the native release workflow, verifies the source, builds the archive, uploads it as an Actions artifact, and publishes it to the matching GitHub release.

## Project files and export

**Save** and **Save As** write a layered `.slate` project. **Export** produces a flattened PNG, JPEG, or WebP image. These are deliberately separate operations.

A `.slate` file is versioned JSON with a format identifier and a serialized document. Before loading, Slate validates the format version, dimensions, layer identifiers, opacity values, raster byte counts, mask byte counts, active-layer reference, and selection data. Runtime-only undo state is not serialized. Writes go to a temporary file and are renamed only after the data has been flushed.

See [`docs/project-format.md`](docs/project-format.md) for the compatibility contract.

## Main shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl+N` | New document |
| `Ctrl+O` | Open image or Slate project |
| `Ctrl+S` | Save Slate project |
| `Ctrl+Shift+S` | Save project as |
| `Ctrl+Shift+E` | Export flattened image |
| `Ctrl+Z` | Undo |
| `Ctrl+Shift+Z` / `Ctrl+Y` | Redo |
| `V` | Move |
| `L` | Lasso |
| `C` | Crop |
| `B` | Brush |
| `E` | Eraser |
| `I` | Color picker |
| `Z` | Zoom |
| `Space` | Temporary pan |
| `[` / `]` | Brush size |
| `Shift+[` / `Shift+]` | Brush hardness |
| `Ctrl+0` | Fit to screen |
| `Ctrl+1` / `Ctrl+2` / `Ctrl+3` | Quick / Full / Canvas-only workspace |

## Architecture

```text
src/document/   document, layer, mask, command history, filters, project format
src/image/      color conversion, edit pipeline, previews, export, upscaling
src/tile/       immutable render snapshots and flattening
src/tools/      tool types and brush behavior
src/ui/         GTK widgets, canvas interaction, panels, actions, persistence wiring
assets/icons/   symbolic tool icons
data/           desktop, AppStream, MIME, and application icon metadata
scripts/        UI capture and deterministic Linux packaging
```

The mutable document remains the source of truth. Render snapshots are immutable. User edits enter through commands so undo and redo can restore prior state. The project module owns compatibility and input validation, while the UI hardening layer replaces misleading prototype actions without rewriting the established editor window.

More detail is in [`docs/architecture.md`](docs/architecture.md).

## Scope

Slate is a completed focused v1, not a claim to match every GIMP subsystem. It does not currently provide PSD/XCF import, CMYK prepress workflows, plug-ins, vector paths as a full authoring system, nondestructive adjustment layers for every filter, or cross-platform binaries. Controls exposed by the application are expected to perform real work; future tools should remain hidden until their interaction, history, and rendering paths are complete.

## License

MIT. See [`LICENSE`](LICENSE).
