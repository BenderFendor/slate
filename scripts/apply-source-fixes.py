#!/usr/bin/env python3
from pathlib import Path
import re


def replace_exact(text: str, old: str, new: str, expected: int = 1) -> str:
    count = text.count(old)
    if count != expected:
        raise RuntimeError(f"Expected {expected} occurrences, found {count}: {old[:80]!r}")
    return text.replace(old, new)


hardening_path = Path("src/ui/hardening.rs")
hardening = hardening_path.read_text()
hardening = replace_exact(hardening, "        &editor.zoom,\n", "")
hardening = replace_exact(hardening, "    let zoom = editor.zoom.clone();\n", "", expected=2)
hardening = replace_exact(hardening, "        *zoom.borrow_mut() = 1.0;\n", "")
hardening = replace_exact(
    hardening,
    "        show_open_dialog(&parent, &document, &pipeline, &zoom, &canvas);",
    "        show_open_dialog(&parent, &document, &pipeline, &canvas);",
)
hardening = replace_exact(
    hardening,
    "    zoom: &Rc<RefCell<f64>>,\n",
    "",
    expected=2,
)
hardening = replace_exact(hardening, "    let zoom = zoom.clone();\n", "")
hardening = replace_exact(
    hardening,
    "        if let Err(error) = load_path(&document, &pipeline, &zoom, &canvas, &path) {",
    "        if let Err(error) = load_path(&document, &pipeline, &canvas, &path) {",
)
hardening = replace_exact(
    hardening,
    "    queue_fit_to_screen(document, zoom, canvas);",
    "    canvas.queue_draw();",
)
hardening, count = re.subn(
    r"fn queue_fit_to_screen\([\s\S]*?\n}\n\n(?=fn install_title_watch)",
    "",
    hardening,
    count=1,
)
if count != 1:
    raise RuntimeError("Could not remove queue_fit_to_screen")
hardening_path.write_text(hardening)

canvas_path = Path("src/ui/canvas.rs")
canvas = canvas_path.read_text()
canvas = replace_exact(
    canvas,
    "paint_brush_dab(&mut raster, 4.0, 4.0, false, 2.0, 1.0, 1.0, 1.0);",
    "paint_brush_dab(\n"
    "            &mut raster,\n"
    "            None,\n"
    "            4.0,\n"
    "            4.0,\n"
    "            false,\n"
    "            2.0,\n"
    "            1.0,\n"
    "            1.0,\n"
    "            1.0,\n"
    "            [0.0, 0.0, 0.0, 1.0],\n"
    "        );",
)
canvas = replace_exact(
    canvas,
    "paint_brush_dab(&mut raster, 4.0, 4.0, true, 2.0, 0.5, 1.0, 1.0);",
    "paint_brush_dab(\n"
    "            &mut raster,\n"
    "            None,\n"
    "            4.0,\n"
    "            4.0,\n"
    "            true,\n"
    "            2.0,\n"
    "            0.5,\n"
    "            1.0,\n"
    "            1.0,\n"
    "            [0.0, 0.0, 0.0, 1.0],\n"
    "        );",
)
canvas = replace_exact(
    canvas,
    "paint_mask_dab(&mut mask, 4.0, 4.0, false, 2.0, 1.0, 1.0, 1.0);",
    "paint_mask_dab(\n"
    "            &mut mask, None, 4.0, 4.0, false, 2.0, 1.0, 1.0, 1.0,\n"
    "        );",
)
canvas = replace_exact(
    canvas,
    "paint_mask_dab(&mut mask, 4.0, 4.0, true, 2.0, 0.5, 1.0, 1.0);",
    "paint_mask_dab(\n"
    "            &mut mask, None, 4.0, 4.0, true, 2.0, 0.5, 1.0, 1.0,\n"
    "        );",
)
old_brush_at = (
    "            &mut doc, 1.0, 0.0, 0.0, 8.0, 8.0, 4.0, 4.0, false, 2.0, 1.0, 1.0, 1.0,\n"
    "        );"
)
new_brush_at = (
    "            &mut doc,\n"
    "            1.0,\n"
    "            0.0,\n"
    "            0.0,\n"
    "            8.0,\n"
    "            8.0,\n"
    "            4.0,\n"
    "            4.0,\n"
    "            false,\n"
    "            2.0,\n"
    "            1.0,\n"
    "            1.0,\n"
    "            1.0,\n"
    "            [0.0, 0.0, 0.0, 1.0],\n"
    "        );"
)
canvas = replace_exact(canvas, old_brush_at, new_brush_at, expected=2)
canvas_path.write_text(canvas)

print("Applied native source fixes")
