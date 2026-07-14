use gtk4::cairo;
use gtk4::prelude::*;

use crate::document::{Document, Layer, LayerKind, Mask, ModifyLayerCommand, RasterLayer};
use crate::image::pipeline::{CropRect, EditPipeline};
use crate::tile::snapshot::{
    build_render_frame, flatten_document_region_bgra, flatten_frame_bgra, PixelRect,
};
use crate::tools::tool::ToolKind;

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy)]
enum CropDragMode {
    New,
    Move,
    ResizeN,
    ResizeS,
    ResizeE,
    ResizeW,
    ResizeNe,
    ResizeNw,
    ResizeSe,
    ResizeSw,
}

#[derive(Debug, Clone, Copy)]
struct CropDragState {
    mode: CropDragMode,
    start_rect: CropRect,
    start_image: (f64, f64),
    start_canvas: (f64, f64),
}

#[allow(clippy::too_many_arguments)]
fn paint_brush_at(
    doc: &mut Document,
    zoom: f64,
    offset_x: f64,
    offset_y: f64,
    canvas_w: f64,
    canvas_h: f64,
    canvas_x: f64,
    canvas_y: f64,
    is_eraser: bool,
    brush_radius: f64,
    brush_opacity: f64,
    brush_hardness: f64,
    brush_flow: f64,
    brush_color: [f32; 4],
) -> Option<PixelRect> {
    let Some((img_x, img_y)) = canvas_to_image(
        doc, zoom, offset_x, offset_y, canvas_w, canvas_h, canvas_x, canvas_y,
    ) else {
        return None;
    };

    let target_id = doc.active_layer_id;
    let selection = doc.selection.clone();
    if let Some(id) = target_id {
        if let Some(layer) = doc.layer_mut(id) {
            if layer.locked {
                return None;
            }
            let selection_ref = selection.as_ref();
            if let Some(mask) = layer.mask.as_mut().filter(|mask| mask.editing) {
                paint_mask_dab(
                    mask,
                    selection_ref,
                    img_x,
                    img_y,
                    is_eraser,
                    brush_radius,
                    brush_opacity,
                    brush_hardness,
                    brush_flow,
                );
            } else if let LayerKind::Raster(raster) = &mut layer.kind {
                paint_brush_dab(
                    raster,
                    selection_ref,
                    img_x,
                    img_y,
                    is_eraser,
                    brush_radius,
                    brush_opacity,
                    brush_hardness,
                    brush_flow,
                    brush_color,
                );
            }
            return dirty_rect_for_segment(
                (img_x, img_y),
                (img_x, img_y),
                brush_radius,
                doc.canvas_width,
                doc.canvas_height,
            );
        }
    }

    None
}

#[allow(clippy::too_many_arguments)]
fn paint_brush_stroke(
    doc: &mut Document,
    zoom: f64,
    offset_x: f64,
    offset_y: f64,
    canvas_w: f64,
    canvas_h: f64,
    from: (f64, f64),
    to: (f64, f64),
    is_eraser: bool,
    brush_radius: f64,
    brush_opacity: f64,
    brush_hardness: f64,
    brush_flow: f64,
    brush_color: [f32; 4],
) -> Option<PixelRect> {
    let Some(start) = canvas_to_image(
        doc, zoom, offset_x, offset_y, canvas_w, canvas_h, from.0, from.1,
    ) else {
        return None;
    };
    let Some(end) = canvas_to_image(
        doc, zoom, offset_x, offset_y, canvas_w, canvas_h, to.0, to.1,
    ) else {
        return None;
    };

    let target_id = doc.active_layer_id;
    let selection = doc.selection.clone();
    if let Some(id) = target_id {
        if let Some(layer) = doc.layer_mut(id) {
            if layer.locked {
                return None;
            }
            let selection_ref = selection.as_ref();
            if let Some(mask) = layer.mask.as_mut().filter(|mask| mask.editing) {
                let dx = end.0 - start.0;
                let dy = end.1 - start.1;
                let dist = (dx * dx + dy * dy).sqrt();
                let spacing = (brush_radius * 0.25).max(1.0);
                let steps = (dist / spacing).ceil().max(1.0) as usize;

                for step in 1..=steps {
                    let t = step as f64 / steps as f64;
                    paint_mask_dab(
                        mask,
                        selection_ref,
                        start.0 + dx * t,
                        start.1 + dy * t,
                        is_eraser,
                        brush_radius,
                        brush_opacity,
                        brush_hardness,
                        brush_flow,
                    );
                }
            } else if let LayerKind::Raster(raster) = &mut layer.kind {
                let dx = end.0 - start.0;
                let dy = end.1 - start.1;
                let dist = (dx * dx + dy * dy).sqrt();
                let spacing = (brush_radius * 0.25).max(1.0);
                let steps = (dist / spacing).ceil().max(1.0) as usize;

                for step in 1..=steps {
                    let t = step as f64 / steps as f64;
                    paint_brush_dab(
                        raster,
                        selection_ref,
                        start.0 + dx * t,
                        start.1 + dy * t,
                        is_eraser,
                        brush_radius,
                        brush_opacity,
                        brush_hardness,
                        brush_flow,
                        brush_color,
                    );
                }
            }
            return dirty_rect_for_segment(
                start,
                end,
                brush_radius,
                doc.canvas_width,
                doc.canvas_height,
            );
        }
    }

    None
}

fn dirty_rect_for_segment(
    start: (f64, f64),
    end: (f64, f64),
    radius: f64,
    canvas_width: u32,
    canvas_height: u32,
) -> Option<PixelRect> {
    let left = (start.0.min(end.0) - radius).floor().max(0.0) as u32;
    let top = (start.1.min(end.1) - radius).floor().max(0.0) as u32;
    let right = (start.0.max(end.0) + radius)
        .ceil()
        .min(canvas_width as f64) as u32;
    let bottom = (start.1.max(end.1) + radius)
        .ceil()
        .min(canvas_height as f64) as u32;

    if right <= left || bottom <= top {
        return None;
    }
    Some(PixelRect::new(left, top, right - left, bottom - top))
}

fn pick_color_at(doc: &Document, img_x: f64, img_y: f64) -> Option<[f32; 4]> {
    let x = img_x.round() as i32;
    let y = img_y.round() as i32;
    if x < 0 || x >= doc.canvas_width as i32 || y < 0 || y >= doc.canvas_height as i32 {
        return None;
    }

    let mut pixels = vec![0u8; doc.canvas_width as usize * doc.canvas_height as usize * 4];
    flatten_document_region_bgra(
        doc,
        PixelRect::new(x as u32, y as u32, 1, 1),
        &mut pixels,
    );

    let idx = (y as usize * doc.canvas_width as usize + x as usize) * 4;
    let b = pixels[idx] as f32 / 255.0;
    let g = pixels[idx + 1] as f32 / 255.0;
    let r = pixels[idx + 2] as f32 / 255.0;
    let a = pixels[idx + 3] as f32 / 255.0;

    Some([r, g, b, a])
}

fn show_brush_popover(
    parent: &gtk4::Widget,
    x: f64,
    y: f64,
    size: &Rc<RefCell<f64>>,
    hardness: &Rc<RefCell<f64>>,
    _opacity: &Rc<RefCell<f64>>,
    color: &Rc<RefCell<[f32; 4]>>,
) {
    let popover = gtk4::Popover::new();
    popover.set_parent(parent);
    popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));

    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);

    // Size Slider
    let size_label = gtk4::Label::new(Some("Size"));
    size_label.set_halign(gtk4::Align::Start);
    vbox.append(&size_label);
    let size_adj = gtk4::Adjustment::new(*size.borrow(), 1.0, 500.0, 1.0, 10.0, 0.0);
    let size_scale = gtk4::Scale::new(gtk4::Orientation::Horizontal, Some(&size_adj));
    size_scale.set_hexpand(true);
    size_scale.set_draw_value(true);
    {
        let size = size.clone();
        let p = parent.clone();
        size_adj.connect_value_changed(move |adj| {
            *size.borrow_mut() = adj.value();
            p.queue_draw();
        });
    }
    vbox.append(&size_scale);

    // Hardness Slider
    let hardness_label = gtk4::Label::new(Some("Hardness"));
    hardness_label.set_halign(gtk4::Align::Start);
    vbox.append(&hardness_label);
    let hardness_adj = gtk4::Adjustment::new(*hardness.borrow() * 100.0, 0.0, 100.0, 1.0, 10.0, 0.0);
    let hardness_scale = gtk4::Scale::new(gtk4::Orientation::Horizontal, Some(&hardness_adj));
    hardness_scale.set_hexpand(true);
    hardness_scale.set_draw_value(true);
    {
        let hardness = hardness.clone();
        let p = parent.clone();
        hardness_adj.connect_value_changed(move |adj| {
            *hardness.borrow_mut() = adj.value() / 100.0;
            p.queue_draw();
        });
    }
    vbox.append(&hardness_scale);

    // Quick Colors
    let colors_label = gtk4::Label::new(Some("Swatches"));
    colors_label.set_halign(gtk4::Align::Start);
    vbox.append(&colors_label);
    let grid = gtk4::FlowBox::new();
    grid.set_max_children_per_line(8);
    grid.set_selection_mode(gtk4::SelectionMode::None);

    let swatches = [
        [0.0, 0.0, 0.0, 1.0], // Black
        [1.0, 1.0, 1.0, 1.0], // White
        [1.0, 0.0, 0.0, 1.0], // Red
        [0.0, 1.0, 0.0, 1.0], // Green
        [0.0, 0.0, 1.0, 1.0], // Blue
        [1.0, 1.0, 0.0, 1.0], // Yellow
        [1.0, 0.0, 1.0, 1.0], // Magenta
        [0.0, 1.0, 1.0, 1.0], // Cyan
    ];

    for c in swatches {
        let btn = gtk4::Button::new();
        btn.add_css_class("flat");
        btn.set_size_request(24, 24);
        let draw = gtk4::DrawingArea::new();
        draw.set_draw_func(move |_area, cr, w, h| {
            cr.set_source_rgba(c[0] as f64, c[1] as f64, c[2] as f64, c[3] as f64);
            cr.rectangle(0.0, 0.0, w as f64, h as f64);
            cr.fill().ok();
        });
        btn.set_child(Some(&draw));
        {
            let color = color.clone();
            let p = parent.clone();
            btn.connect_clicked(move |_| {
                *color.borrow_mut() = c;
                p.queue_draw();
            });
        }
        grid.insert(&btn, -1);
    }
    vbox.append(&grid);

    popover.set_child(Some(&vbox));
    popover.popup();
}

#[allow(clippy::too_many_arguments)]
fn paint_mask_dab(
    mask: &mut Mask,
    selection: Option<&Mask>,
    img_x: f64,
    img_y: f64,
    is_eraser: bool,
    brush_radius: f64,
    brush_opacity: f64,
    brush_hardness: f64,
    brush_flow: f64,
) {
    let radius = brush_radius.max(0.5);
    let ir = radius.ceil() as i32;
    let opacity = (brush_opacity * brush_flow).clamp(0.0, 1.0);
    let hardness = brush_hardness.clamp(0.0, 1.0);

    for dy in -ir..=ir {
        for dx in -ir..=ir {
            let dist = ((dx * dx + dy * dy) as f64).sqrt();
            if dist > radius {
                continue;
            }

            let px = img_x.round() as i32 + dx;
            let py = img_y.round() as i32 + dy;
            if px < 0 || px >= mask.width as i32 || py < 0 || py >= mask.height as i32 {
                continue;
            }

            let idx = py as usize * mask.width as usize + px as usize;
            if idx >= mask.data.len() {
                continue;
            }

            let normalized = dist / radius;
            let falloff = if hardness >= 1.0 || normalized <= hardness {
                1.0
            } else {
                let soft_span = (1.0 - hardness).max(0.001);
                (1.0 - ((normalized - hardness) / soft_span)).clamp(0.0, 1.0)
            };
            let mut selection_amount = 1.0f32;
            if let Some(sel) = selection {
                if px >= 0 && px < sel.width as i32 && py >= 0 && py < sel.height as i32 {
                    let s_idx = py as usize * sel.width as usize + px as usize;
                    selection_amount = sel.data[s_idx] as f32 / 255.0;
                } else {
                    selection_amount = 0.0;
                }
            }

            let amount = (opacity * falloff * selection_amount as f64).clamp(0.0, 1.0) as f32;
            let target = if is_eraser { 255.0 } else { 0.0 };
            mask.data[idx] = (mask.data[idx] as f32 * (1.0 - amount) + target * amount) as u8;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn canvas_to_image(
    doc: &Document,
    zoom: f64,
    offset_x: f64,
    offset_y: f64,
    canvas_w: f64,
    canvas_h: f64,
    canvas_x: f64,
    canvas_y: f64,
) -> Option<(f64, f64)> {
    if zoom <= 0.0 {
        return None;
    }

    let img_w = doc.canvas_width as f64;
    let img_h = doc.canvas_height as f64;
    let draw_w = img_w * zoom;
    let draw_h = img_h * zoom;
    let cx = canvas_w / 2.0 + offset_x;
    let cy = canvas_h / 2.0 + offset_y;
    Some((
        (canvas_x - (cx - draw_w / 2.0)) / zoom,
        (canvas_y - (cy - draw_h / 2.0)) / zoom,
    ))
}

#[allow(clippy::too_many_arguments)]
fn paint_brush_dab(
    raster: &mut RasterLayer,
    selection: Option<&Mask>,
    img_x: f64,
    img_y: f64,
    is_eraser: bool,
    brush_radius: f64,
    brush_opacity: f64,
    brush_hardness: f64,
    brush_flow: f64,
    brush_color: [f32; 4],
) {
    let radius = brush_radius.max(0.5);
    let ir = radius.ceil() as i32;
    let opacity = (brush_opacity * brush_flow).clamp(0.0, 1.0);
    let hardness = brush_hardness.clamp(0.0, 1.0);

    for dy in -ir..=ir {
        for dx in -ir..=ir {
            let dist = ((dx * dx + dy * dy) as f64).sqrt();
            if dist > radius {
                continue;
            }

            let px = img_x.round() as i32 + dx;
            let py = img_y.round() as i32 + dy;

            if px < 0 || px >= raster.width as i32 || py < 0 || py >= raster.height as i32 {
                continue;
            }

            let idx = (py as usize * raster.width as usize + px as usize) * 4;
            if idx + 3 >= raster.data.len() {
                continue;
            }

            let normalized = dist / radius;
            let falloff = if hardness >= 1.0 || normalized <= hardness {
                1.0
            } else {
                let soft_span = (1.0 - hardness).max(0.001);
                (1.0 - ((normalized - hardness) / soft_span)).clamp(0.0, 1.0)
            };
            let amount_base = (opacity * falloff).clamp(0.0, 1.0) as f32;
            let mut selection_amount = 1.0f32;
            if let Some(sel) = selection {
                if px >= 0 && px < sel.width as i32 && py >= 0 && py < sel.height as i32 {
                    let s_idx = py as usize * sel.width as usize + px as usize;
                    selection_amount = sel.data[s_idx] as f32 / 255.0;
                } else {
                    selection_amount = 0.0;
                }
            }
            let amount = amount_base * selection_amount;

            if is_eraser {
                raster.data[idx + 3] = (raster.data[idx + 3] as f32 * (1.0 - amount)) as u8;
            } else {
                let src_a = brush_color[3] * amount;
                let dst_a = raster.data[idx + 3] as f32 / 255.0;
                let out_a = src_a + dst_a * (1.0 - src_a);
                
                if out_a > 0.0 {
                    let blend = |s: f32, d: f32| (s * src_a + d * dst_a * (1.0 - src_a)) / out_a;
                    raster.data[idx] = (blend(brush_color[0] * 255.0, raster.data[idx] as f32)).clamp(0.0, 255.0) as u8;
                    raster.data[idx + 1] = (blend(brush_color[1] * 255.0, raster.data[idx + 1] as f32)).clamp(0.0, 255.0) as u8;
                    raster.data[idx + 2] = (blend(brush_color[2] * 255.0, raster.data[idx + 2] as f32)).clamp(0.0, 255.0) as u8;
                    raster.data[idx + 3] = (out_a * 255.0).clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

fn crop_hit_mode(crop: CropRect, point: (f64, f64), tolerance: f64) -> CropDragMode {
    let left = (point.0 - crop.x).abs() <= tolerance;
    let right = (point.0 - (crop.x + crop.width)).abs() <= tolerance;
    let top = (point.1 - crop.y).abs() <= tolerance;
    let bottom = (point.1 - (crop.y + crop.height)).abs() <= tolerance;

    match (left, right, top, bottom) {
        (true, _, true, _) => CropDragMode::ResizeNw,
        (_, true, true, _) => CropDragMode::ResizeNe,
        (true, _, _, true) => CropDragMode::ResizeSw,
        (_, true, _, true) => CropDragMode::ResizeSe,
        (_, _, true, _) => CropDragMode::ResizeN,
        (_, _, _, true) => CropDragMode::ResizeS,
        (true, _, _, _) => CropDragMode::ResizeW,
        (_, true, _, _) => CropDragMode::ResizeE,
        _ => {
            let inside_x = point.0 >= crop.x && point.0 <= crop.x + crop.width;
            let inside_y = point.1 >= crop.y && point.1 <= crop.y + crop.height;
            if inside_x && inside_y {
                CropDragMode::Move
            } else {
                CropDragMode::New
            }
        }
    }
}

fn update_crop_drag(
    mode: CropDragMode,
    start_rect: CropRect,
    start_image: (f64, f64),
    current_image: (f64, f64),
    canvas_width: f64,
    canvas_height: f64,
) -> CropRect {
    let min_size = 1.0;
    let mut left = start_rect.x;
    let mut top = start_rect.y;
    let mut right = start_rect.x + start_rect.width;
    let mut bottom = start_rect.y + start_rect.height;

    match mode {
        CropDragMode::New => {
            left = start_image.0.min(current_image.0);
            top = start_image.1.min(current_image.1);
            right = start_image.0.max(current_image.0);
            bottom = start_image.1.max(current_image.1);
        }
        CropDragMode::Move => {
            let dx = current_image.0 - start_image.0;
            let dy = current_image.1 - start_image.1;
            left = start_rect.x + dx;
            top = start_rect.y + dy;
            right = left + start_rect.width;
            bottom = top + start_rect.height;
        }
        CropDragMode::ResizeN => top = current_image.1,
        CropDragMode::ResizeS => bottom = current_image.1,
        CropDragMode::ResizeE => right = current_image.0,
        CropDragMode::ResizeW => left = current_image.0,
        CropDragMode::ResizeNe => {
            top = current_image.1;
            right = current_image.0;
        }
        CropDragMode::ResizeNw => {
            top = current_image.1;
            left = current_image.0;
        }
        CropDragMode::ResizeSe => {
            bottom = current_image.1;
            right = current_image.0;
        }
        CropDragMode::ResizeSw => {
            bottom = current_image.1;
            left = current_image.0;
        }
    }

    if left > right {
        std::mem::swap(&mut left, &mut right);
    }
    if top > bottom {
        std::mem::swap(&mut top, &mut bottom);
    }

    left = left.clamp(0.0, canvas_width);
    right = right.clamp(0.0, canvas_width);
    top = top.clamp(0.0, canvas_height);
    bottom = bottom.clamp(0.0, canvas_height);

    if right - left < min_size {
        right = (left + min_size).min(canvas_width);
    }
    if bottom - top < min_size {
        bottom = (top + min_size).min(canvas_height);
    }

    CropRect::new(left, top, right - left, bottom - top)
}

fn layer_paint_state_changed(before: &Layer, after: &Layer) -> bool {
    let mask_changed =
        before.mask.as_ref().map(|mask| &mask.data) != after.mask.as_ref().map(|mask| &mask.data);
    let pixels_changed = match (&before.kind, &after.kind) {
        (LayerKind::Raster(before), LayerKind::Raster(after)) => before.data != after.data,
        _ => false,
    };
    mask_changed || pixels_changed
}

fn commit_prepaint_layer_state(doc: &mut Document, before: Layer) {
    let layer_id = before.id;
    let Some(index) = doc.layers.iter().position(|layer| layer.id == layer_id) else {
        return;
    };
    let after = doc.layers[index].clone();
    if !layer_paint_state_changed(&before, &after) {
        return;
    }

    doc.layers[index] = before;
    let mut undo_stack = std::mem::take(&mut doc.undo_stack);
    undo_stack.execute(Box::new(ModifyLayerCommand::new(layer_id, after)), doc);
    doc.undo_stack = undo_stack;
}

pub struct CanvasWidget {
    root: gtk4::Overlay,
    widget: gtk4::DrawingArea,
    overlay: gtk4::DrawingArea,
    empty_state: adw::StatusPage,
    document: Rc<RefCell<Document>>,
    active_tool: Rc<RefCell<ToolKind>>,
    zoom: Rc<RefCell<f64>>,
    pipeline: Rc<RefCell<EditPipeline>>,
    offset_x: Rc<RefCell<f64>>,
    offset_y: Rc<RefCell<f64>>,
    dragging: Rc<RefCell<bool>>,
    last_mouse: Rc<RefCell<(f64, f64)>>,
    painting: Rc<RefCell<bool>>,
    space_pressed: Rc<RefCell<bool>>,
    surface_cache: Rc<RefCell<Option<(u64, cairo::ImageSurface)>>>,
    dirty_regions: Rc<RefCell<Vec<PixelRect>>>,
    brush_size: Rc<RefCell<f64>>,
    brush_hardness: Rc<RefCell<f64>>,
    brush_opacity: Rc<RefCell<f64>>,
    brush_flow: Rc<RefCell<f64>>,
    brush_color: Rc<RefCell<[f32; 4]>>,
    last_paint_point: Rc<RefCell<Option<(f64, f64)>>>,
    paint_before_layer: Rc<RefCell<Option<Layer>>>,
    crop_drag: Rc<RefCell<Option<CropDragState>>>,
    lasso_path: Rc<RefCell<Vec<(f64, f64)>>>,
}

impl CanvasWidget {
    pub fn new(
        document: Rc<RefCell<Document>>,
        active_tool: Rc<RefCell<ToolKind>>,
        zoom: Rc<RefCell<f64>>,
        pipeline: Rc<RefCell<EditPipeline>>,
        brush_size: Rc<RefCell<f64>>,
        brush_hardness: Rc<RefCell<f64>>,
        brush_opacity: Rc<RefCell<f64>>,
        brush_flow: Rc<RefCell<f64>>,
        brush_color: Rc<RefCell<[f32; 4]>>,
    ) -> Self {
        let offset_x = Rc::new(RefCell::new(0.0));
        let offset_y = Rc::new(RefCell::new(0.0));
        let dragging = Rc::new(RefCell::new(false));
        let last_mouse = Rc::new(RefCell::new((0.0, 0.0)));
        let painting = Rc::new(RefCell::new(false));
        let last_paint_point = Rc::new(RefCell::new(None));
        let paint_before_layer = Rc::new(RefCell::new(None));
        let space_pressed = Rc::new(RefCell::new(false));
        let surface_cache = Rc::new(RefCell::new(None));
        let dirty_regions = Rc::new(RefCell::new(Vec::new()));
        let crop_drag = Rc::new(RefCell::new(None));
        let lasso_path = Rc::new(RefCell::new(Vec::new()));

        let area = gtk4::DrawingArea::new();
        area.set_hexpand(true);
        area.set_vexpand(true);
        area.set_focusable(true);
        area.set_can_focus(true);

        let overlay = gtk4::DrawingArea::new();
        overlay.set_hexpand(true);
        overlay.set_vexpand(true);
        overlay.set_can_target(false);

        let empty_state = adw::StatusPage::builder()
            .title("Open an Image")
            .description("Drag a file here or choose one from disk to start editing")
            .icon_name("insert-image-symbolic")
            .build();
        
        let empty_open = gtk4::Button::builder()
            .label("Open Image")
            .halign(gtk4::Align::Center)
            .css_classes(vec!["suggested-action".to_string(), "pill".to_string()])
            .build();
        
        empty_state.set_child(Some(&empty_open));

        {
            empty_open.connect_clicked(move |button| {
                if let Some(window) = button.root().and_downcast::<gtk4::Window>() {
                    if let Err(error) = window.activate_action("win.open", None) {
                        log::warn!("Open image action failed: {}", error);
                    }
                }
            });
        }

        let root = gtk4::Overlay::new();
        root.set_hexpand(true);
        root.set_vexpand(true);
        root.set_child(Some(&area));
        root.add_overlay(&overlay);
        root.add_overlay(&empty_state);

        let widget = Self {
            root,
            widget: area.clone(),
            overlay: overlay.clone(),
            empty_state: empty_state.clone(),
            document,
            active_tool,
            zoom,
            pipeline,
            offset_x,
            offset_y,
            dragging,
            last_mouse,
            painting,
            space_pressed,
            surface_cache,
            dirty_regions,
            brush_size,
            brush_hardness,
            brush_opacity,
            brush_flow,
            brush_color,
            last_paint_point,
            paint_before_layer,
            crop_drag,
            lasso_path,
        };

        widget.connect_draw();
        widget.connect_events();
        widget.connect_empty_state_watch();

        widget
    }

    pub fn widget(&self) -> &gtk4::DrawingArea {
        &self.widget
    }

    pub fn root(&self) -> &gtk4::Overlay {
        &self.root
    }

    fn connect_empty_state_watch(&self) {
        let doc = self.document.clone();
        let empty_state = self.empty_state.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            empty_state.set_visible(doc.borrow().layers.is_empty());
            glib::ControlFlow::Continue
        });
    }

    fn connect_draw(&self) {
        let doc = self.document.clone();
        let zoom = self.zoom.clone();
        let ox = self.offset_x.clone();
        let oy = self.offset_y.clone();
        let cache = self.surface_cache.clone();
        let dirty_regions = self.dirty_regions.clone();

        self.widget.set_draw_func(move |_area, cr, width, height| {
            Self::on_draw_backing(
                cr,
                width as f64,
                height as f64,
                &doc,
                *zoom.borrow(),
                *ox.borrow(),
                *oy.borrow(),
                &mut cache.borrow_mut(),
                &mut dirty_regions.borrow_mut(),
            );
        });

        let doc = self.document.clone();
        let zoom = self.zoom.clone();
        let ox = self.offset_x.clone();
        let oy = self.offset_y.clone();
        let pip = self.pipeline.clone();

        let lp = self.lasso_path.clone();
        self.overlay.set_draw_func(move |_area, cr, width, height| {
            Self::on_draw_overlays(
                cr,
                width as f64,
                height as f64,
                &doc.borrow(),
                *zoom.borrow(),
                *ox.borrow(),
                *oy.borrow(),
                &pip.borrow(),
                &lp.borrow(),
            );
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn on_draw_backing(
        cr: &cairo::Context,
        width: f64,
        height: f64,
        document: &Rc<RefCell<Document>>,
        zoom: f64,
        offset_x: f64,
        offset_y: f64,
        surface_cache: &mut Option<(u64, cairo::ImageSurface)>,
        dirty_regions: &mut Vec<PixelRect>,
    ) {
        // Dark background
        cr.set_source_rgb(0.12, 0.12, 0.13);
        cr.paint().ok();

        let doc = document.borrow();

        if doc.layers.is_empty() {
            return;
        }

        let canvas_w = doc.canvas_width as f64;
        let canvas_h = doc.canvas_height as f64;
        let draw_w = canvas_w * zoom;
        let draw_h = canvas_h * zoom;

        let cx = width / 2.0 + offset_x;
        let cy = height / 2.0 + offset_y;
        let x = cx - draw_w / 2.0;
        let y = cy - draw_h / 2.0;

        // Checkerboard
        cr.save().ok();
        cr.rectangle(x, y, draw_w, draw_h);
        cr.clip();
        let check_size = 16.0;
        for r in 0..(draw_h / check_size).ceil() as i32 {
            for c in 0..(draw_w / check_size).ceil() as i32 {
                if (r + c) % 2 == 0 {
                    cr.set_source_rgb(0.18, 0.18, 0.19);
                } else {
                    cr.set_source_rgb(0.15, 0.15, 0.16);
                }
                cr.rectangle(
                    x + c as f64 * check_size,
                    y + r as f64 * check_size,
                    check_size,
                    check_size,
                );
                cr.fill().ok();
            }
        }
        cr.restore().ok();

        let surface_dimensions_changed = surface_cache
            .as_ref()
            .map(|(_, surface)| {
                surface.width() != doc.canvas_width as i32
                    || surface.height() != doc.canvas_height as i32
            })
            .unwrap_or(true);
        let revision_changed = surface_cache
            .as_ref()
            .map(|(revision, _)| *revision != doc.revision)
            .unwrap_or(true);
        let needs_full_surface_update =
            surface_dimensions_changed || (revision_changed && dirty_regions.is_empty());

        if needs_full_surface_update {
            let frame = build_render_frame(&doc);
            let flattened = flatten_frame_bgra(&frame);
            *surface_cache = cairo::ImageSurface::create_for_data(
                flattened.pixels_bgra,
                cairo::Format::ARgb32,
                flattened.width as i32,
                flattened.height as i32,
                flattened.width as i32 * 4,
            )
            .ok()
            .map(|surface| (frame.revision, surface));
        } else if revision_changed {
            if let Some((revision, surface)) = surface_cache.as_mut() {
                surface.flush();
                let region = dirty_regions
                    .drain(..)
                    .reduce(PixelRect::union)
                    .unwrap_or_else(|| PixelRect::new(0, 0, 0, 0));
                let updated = {
                    let Ok(mut data) = surface.data() else {
                        return;
                    };
                    flatten_document_region_bgra(&doc, region, &mut data);
                    true
                };
                if updated {
                    surface.mark_dirty_rectangle(
                        region.x as i32,
                        region.y as i32,
                        region.width as i32,
                        region.height as i32,
                    );
                    *revision = doc.revision;
                }
            }
        }

        if let Some((_, surface)) = surface_cache.as_ref() {
            cr.save().ok();
            cr.translate(x, y);
            cr.scale(zoom, zoom);
            cr.set_source_surface(surface, 0.0, 0.0).ok();
            cr.paint().ok();
            cr.restore().ok();
        }
    }

    fn on_draw_overlays(
        cr: &cairo::Context,
        width: f64,
        height: f64,
        doc: &Document,
        zoom: f64,
        offset_x: f64,
        offset_y: f64,
        pipeline: &EditPipeline,
        lasso_path: &[(f64, f64)],
    ) {
        if doc.layers.is_empty() {
            return;
        }

        let canvas_w = doc.canvas_width as f64;
        let canvas_h = doc.canvas_height as f64;
        let draw_w = canvas_w * zoom;
        let draw_h = canvas_h * zoom;

        let cx = width / 2.0 + offset_x;
        let cy = height / 2.0 + offset_y;
        let x = cx - draw_w / 2.0;
        let y = cy - draw_h / 2.0;

        cr.set_source_rgba(0.0, 0.0, 0.0, 0.4);
        cr.set_line_width(1.0);
        cr.rectangle(x - 1.0, y - 1.0, draw_w + 2.0, draw_h + 2.0);
        cr.stroke().ok();

        if let Some(crop) = &pipeline.crop {
            let cx = x + crop.x * zoom;
            let cy = y + crop.y * zoom;
            let cw = crop.width * zoom;
            let ch = crop.height * zoom;

            cr.save().ok();
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
            cr.rectangle(0.0, 0.0, width, cy);
            cr.rectangle(0.0, cy + ch, width, height - cy - ch);
            cr.rectangle(0.0, cy, cx, ch);
            cr.rectangle(cx + cw, cy, width - cx - cw, ch);
            cr.fill().ok();

            cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
            cr.set_line_width(2.0);
            cr.rectangle(cx, cy, cw, ch);
            cr.stroke().ok();

            // Grid lines
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.25);
            cr.set_line_width(1.0);
            for i in 1..3 {
                let fx = cx + cw * i as f64 / 3.0;
                cr.move_to(fx, cy);
                cr.line_to(fx, cy + ch);
                let fy = cy + ch * i as f64 / 3.0;
                cr.move_to(cx, fy);
                cr.line_to(cx + cw, fy);
            }
            cr.stroke().ok();

            let handle_size = 12.0;
            for (hx, hy) in &[
                (cx, cy),
                (cx + cw, cy),
                (cx, cy + ch),
                (cx + cw, cy + ch),
                (cx + cw / 2.0, cy),
                (cx + cw / 2.0, cy + ch),
                (cx, cy + ch / 2.0),
                (cx + cw, cy + ch / 2.0),
            ] {
                cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                cr.rectangle(
                    hx - handle_size / 2.0,
                    hy - handle_size / 2.0,
                    handle_size,
                    handle_size,
                );
                cr.fill().ok();
                cr.set_source_rgba(0.2, 0.2, 0.2, 0.5);
                cr.set_line_width(1.0);
                cr.rectangle(
                    hx - handle_size / 2.0,
                    hy - handle_size / 2.0,
                    handle_size,
                    handle_size,
                );
                cr.stroke().ok();
            }

            // Readout overlay when resize target is set
            if let Some(resize) = &pipeline.resize {
                let ar_text = format!("{}:{}", resize.width, resize.height);

                let readout_text = format!(
                    "Crop: {:.0} x {:.0}\nOutput: {} x {}\nScale: {:.1}%\nKernel: {}",
                    crop.width,
                    crop.height,
                    resize.width,
                    resize.height,
                    (resize.width as f64 / crop.width * 100.0),
                    pipeline.kernel.as_str()
                );

                cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
                cr.set_font_size(11.0);

                let lines: Vec<&str> = readout_text.split('\n').collect();
                let mut max_w = 0.0;
                let mut total_h = 0.0;
                for line in &lines {
                    let ext = cr.text_extents(line).unwrap();
                    if ext.width() > max_w {
                        max_w = ext.width();
                    }
                    total_h += ext.height() + 2.0;
                }

                let pad = 6.0;
                let bg_x = cx + 8.0 - pad;
                let bg_y = cy + 8.0 - pad;
                let bg_w = max_w + pad * 2.0;
                let bg_h = total_h + pad * 2.0;

                cr.set_source_rgba(0.0, 0.0, 0.0, 0.7);
                cr.rectangle(bg_x, bg_y, bg_w, bg_h);
                cr.fill().ok();

                cr.set_source_rgba(1.0, 1.0, 1.0, 0.9);
                let mut ly = cy + 8.0 + 11.0;
                for line in &lines {
                    cr.move_to(cx + 8.0, ly);
                    cr.show_text(line).ok();
                    let ext = cr.text_extents(line).unwrap();
                    ly += ext.height() + 2.0;
                }

                // Aspect ratio label top-right of crop rect
                cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                cr.set_font_size(11.0);
                let ar_ext = cr.text_extents(&ar_text).unwrap();
                let ar_pad = 4.0;
                let ar_bg_x = cx + cw - ar_ext.width() - ar_pad * 2.0 - 8.0;
                let ar_bg_y = cy + 8.0 - ar_pad;
                cr.set_source_rgba(0.0, 0.0, 0.0, 0.7);
                cr.rectangle(
                    ar_bg_x,
                    ar_bg_y,
                    ar_ext.width() + ar_pad * 2.0,
                    ar_ext.height() + ar_pad * 2.0,
                );
                cr.fill().ok();
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.9);
                cr.move_to(ar_bg_x + ar_pad, ar_bg_y + ar_pad + 9.0);
                cr.show_text(&ar_text).ok();
            }

            // Hint text at bottom center of crop rect
            let hint = "Enter to apply \u{00b7} Esc to cancel";
            cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(10.0);
            let hint_ext = cr.text_extents(hint).unwrap();
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.35);
            cr.move_to(cx + cw / 2.0 - hint_ext.width() / 2.0, cy + ch - 6.0);
            cr.show_text(hint).ok();

            cr.restore().ok();
        }

        // Draw Lasso path
        if !lasso_path.is_empty() {
            cr.save().ok();
            cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            cr.set_line_width(1.0);
            cr.set_dash(&[4.0, 4.0], 0.0);
            
            for (i, p) in lasso_path.iter().enumerate() {
                if i == 0 {
                    cr.move_to(p.0, p.1);
                } else {
                    cr.line_to(p.0, p.1);
                }
            }
            cr.stroke().ok();

            // Black dash offset
            cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
            cr.set_dash(&[4.0, 4.0], 4.0);
            for (i, p) in lasso_path.iter().enumerate() {
                if i == 0 {
                    cr.move_to(p.0, p.1);
                } else {
                    cr.line_to(p.0, p.1);
                }
            }
            cr.stroke().ok();
            cr.restore().ok();
        }
    }

    fn connect_events(&self) {
        let zoom_val = self.zoom.clone();
        let zoom_ox = self.offset_x.clone();
        let zoom_oy = self.offset_y.clone();
        let zoom_widget = self.widget.clone();
        let zoom_overlay = self.overlay.clone();
        let scroll_last = self.last_mouse.clone();

        let scroll_ctrl =
            gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::BOTH_AXES);
        scroll_ctrl.connect_scroll(move |_ctrl, _dx, dy| {
            let mut z = *zoom_val.borrow();
            let old_z = z;
            let factor = if dy > 0.0 { 0.9 } else { 1.1 };
            z = (z * factor).clamp(0.01, 64.0);

            // Zoom under cursor logic
            let (mx, my) = *scroll_last.borrow();
            #[allow(deprecated)]
            let alloc = zoom_widget.allocation();
            let cw = alloc.width() as f64;
            let ch = alloc.height() as f64;

            // Point in canvas space relative to center + offset
            let rx = mx - (cw / 2.0 + *zoom_ox.borrow());
            let ry = my - (ch / 2.0 + *zoom_oy.borrow());

            // Adjust offsets to keep (rx, ry) under cursor
            *zoom_ox.borrow_mut() -= rx * (z / old_z - 1.0);
            *zoom_oy.borrow_mut() -= ry * (z / old_z - 1.0);

            *zoom_val.borrow_mut() = z;
            zoom_widget.queue_draw();
            zoom_overlay.queue_draw();
            gtk4::glib::Propagation::Stop
        });
        self.widget.add_controller(scroll_ctrl);

        let motion_last = self.last_mouse.clone();
        let motion = gtk4::EventControllerMotion::new();
        {
            let last = motion_last.clone();
            motion.connect_motion(move |_ctrl, x, y| {
                *last.borrow_mut() = (x, y);
            });
        }
        self.widget.add_controller(motion);

        let key_space = self.space_pressed.clone();
        let key_tool = self.active_tool.clone();
        let key_bs = self.brush_size.clone();
        let key_bh = self.brush_hardness.clone();
        let key_bo = self.brush_opacity.clone();
        let key_ctrl = gtk4::EventControllerKey::new();
        {
            let space = key_space.clone();
            let widget = self.widget.clone();
            let tool = key_tool.clone();
            let bs = key_bs.clone();
            let bh = key_bh.clone();
            let bo = key_bo.clone();
            let pipeline = self.pipeline.clone();
            key_ctrl.connect_key_pressed(move |_ctrl, key, _code, mods| {
                if key == gtk4::gdk::Key::space && mods.is_empty() {
                    *space.borrow_mut() = true;
                    widget.set_cursor_from_name(Some("grab"));
                    return gtk4::glib::Propagation::Stop;
                }

                if mods.intersects(
                    gtk4::gdk::ModifierType::CONTROL_MASK | gtk4::gdk::ModifierType::ALT_MASK,
                ) {
                    return gtk4::glib::Propagation::Proceed;
                }

                let shift = mods.contains(gtk4::gdk::ModifierType::SHIFT_MASK);

                match key {
                    // Tool selection (Photoshop style)
                    gtk4::gdk::Key::v => {
                        *tool.borrow_mut() = ToolKind::Move;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::c => {
                        *tool.borrow_mut() = ToolKind::Crop;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::b => {
                        *tool.borrow_mut() = ToolKind::Brush;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::e => {
                        *tool.borrow_mut() = ToolKind::Eraser;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::z => {
                        *tool.borrow_mut() = ToolKind::Zoom;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::i => {
                        *tool.borrow_mut() = ToolKind::ColorPicker;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }

                    // Brush adjustments
                    gtk4::gdk::Key::bracketleft if !shift => {
                        let mut s = *bs.borrow();
                        s = (s - 5.0).max(1.0);
                        *bs.borrow_mut() = s;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::bracketleft if shift => {
                        let mut h = *bh.borrow();
                        h = (h - 0.1).max(0.0);
                        *bh.borrow_mut() = h;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::bracketright if !shift => {
                        let mut s = *bs.borrow();
                        s = (s + 5.0).min(500.0);
                        *bs.borrow_mut() = s;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::bracketright if shift => {
                        let mut h = *bh.borrow();
                        h = (h + 0.1).min(1.0);
                        *bh.borrow_mut() = h;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }

                    // Tool actions (Escape/Enter)
                    gtk4::gdk::Key::Escape => {
                        let t = *tool.borrow();
                        if t == ToolKind::Crop {
                            pipeline.borrow_mut().crop = None;
                            widget.queue_draw();
                        }
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::Return | gtk4::gdk::Key::KP_Enter => {
                        let t = *tool.borrow();
                        if t == ToolKind::Crop {
                            // In this implementation, crop is applied immediately to the pipeline
                            // so we just return to the move tool to "confirm"
                            *tool.borrow_mut() = ToolKind::Move;
                            widget.queue_draw();
                        }
                        return gtk4::glib::Propagation::Stop;
                    }

                    gtk4::gdk::Key::_0 => {
                        *bo.borrow_mut() = 1.0;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::_1 => {
                        *bo.borrow_mut() = 0.1;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::_2 => {
                        *bo.borrow_mut() = 0.2;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::_3 => {
                        *bo.borrow_mut() = 0.3;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::_4 => {
                        *bo.borrow_mut() = 0.4;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::_5 => {
                        *bo.borrow_mut() = 0.5;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::_6 => {
                        *bo.borrow_mut() = 0.6;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::_7 => {
                        *bo.borrow_mut() = 0.7;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::_8 => {
                        *bo.borrow_mut() = 0.8;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::_9 => {
                        *bo.borrow_mut() = 0.9;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::KP_0 => {
                        *bo.borrow_mut() = 1.0;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::KP_1 => {
                        *bo.borrow_mut() = 0.1;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::KP_2 => {
                        *bo.borrow_mut() = 0.2;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::KP_3 => {
                        *bo.borrow_mut() = 0.3;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::KP_4 => {
                        *bo.borrow_mut() = 0.4;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::KP_5 => {
                        *bo.borrow_mut() = 0.5;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::KP_6 => {
                        *bo.borrow_mut() = 0.6;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::KP_7 => {
                        *bo.borrow_mut() = 0.7;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::KP_8 => {
                        *bo.borrow_mut() = 0.8;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::KP_9 => {
                        *bo.borrow_mut() = 0.9;
                        widget.queue_draw();
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::bracketleft | gtk4::gdk::Key::bracketright => {
                        gtk4::glib::Propagation::Proceed
                    }
                    _ => gtk4::glib::Propagation::Proceed,
                }
            });
        }
        {
            let space = key_space.clone();
            let widget = self.widget.clone();
            key_ctrl.connect_key_released(move |_ctrl, key, _code, _mods| {
                if key == gtk4::gdk::Key::space {
                    *space.borrow_mut() = false;
                    widget.set_cursor_from_name(None);
                }
            });
        }
        self.widget.add_controller(key_ctrl);

        let drag_dragging = self.dragging.clone();
        let drag_last = self.last_mouse.clone();
        let drag_ox = self.offset_x.clone();
        let drag_oy = self.offset_y.clone();
        let drag_widget = self.widget.clone();
        let drag_overlay = self.overlay.clone();
        let drag_space = self.space_pressed.clone();
        let drag_tool = self.active_tool.clone();
        let drag_doc = self.document.clone();
        let drag_zoom = self.zoom.clone();
        let drag_pipeline = self.pipeline.clone();
        let drag_crop = self.crop_drag.clone();

        let drag_ctrl = gtk4::GestureDrag::new();
        {
            let d = drag_dragging.clone();
            let l = drag_last.clone();
            let space = drag_space.clone();
            let tool = drag_tool.clone();
            let widget = drag_widget.clone();
            let overlay = drag_overlay.clone();
            let doc = drag_doc.clone();
            let zoom = drag_zoom.clone();
            let ox = drag_ox.clone();
            let oy = drag_oy.clone();
            let pipeline = drag_pipeline.clone();
            let crop_drag = drag_crop.clone();
            drag_ctrl.connect_drag_begin(move |_gesture, start_x, start_y| {
                *d.borrow_mut() = true;
                *l.borrow_mut() = (0.0, 0.0);
                if *space.borrow() {
                    widget.set_cursor_from_name(Some("grabbing"));
                }
                if *tool.borrow() == ToolKind::Crop && !*space.borrow() {
                    #[allow(deprecated)]
                    let alloc = widget.allocation();
                    let doc_ref = doc.borrow();
                    if let Some(point) = canvas_to_image(
                        &doc_ref,
                        *zoom.borrow(),
                        *ox.borrow(),
                        *oy.borrow(),
                        alloc.width() as f64,
                        alloc.height() as f64,
                        start_x,
                        start_y,
                    ) {
                        drop(doc_ref);
                        let mut pipeline_ref = pipeline.borrow_mut();
                        let existing_crop = pipeline_ref.crop;
                        let start_rect = existing_crop
                            .unwrap_or_else(|| CropRect::new(point.0, point.1, 1.0, 1.0));
                        let tolerance = (8.0 / *zoom.borrow()).max(1.0);
                        let mode = existing_crop
                            .map(|crop| crop_hit_mode(crop, point, tolerance))
                            .unwrap_or(CropDragMode::New);
                        pipeline_ref.crop = Some(start_rect);
                        *crop_drag.borrow_mut() = Some(CropDragState {
                            mode,
                            start_rect,
                            start_image: point,
                            start_canvas: (start_x, start_y),
                        });
                        overlay.queue_draw();
                    }
                }
                widget.grab_focus();
            });
        }
        {
            let l = drag_last.clone();
            let ox = drag_ox.clone();
            let oy = drag_oy.clone();
            let w = drag_widget.clone();
            let overlay = drag_overlay.clone();
            let space = drag_space.clone();
            let tool = drag_tool.clone();
            let doc = drag_doc.clone();
            let zoom = drag_zoom.clone();
            let pipeline = drag_pipeline.clone();
            let crop_drag = drag_crop.clone();
            drag_ctrl.connect_drag_update(move |_gesture, offset_x, offset_y| {
                let (lx, ly) = *l.borrow();

                if let Some(state) = *crop_drag.borrow() {
                    #[allow(deprecated)]
                    let alloc = w.allocation();
                    let current_canvas = (
                        state.start_canvas.0 + offset_x,
                        state.start_canvas.1 + offset_y,
                    );
                    let d = doc.borrow();
                    if let Some(current_image) = canvas_to_image(
                        &d,
                        *zoom.borrow(),
                        *ox.borrow(),
                        *oy.borrow(),
                        alloc.width() as f64,
                        alloc.height() as f64,
                        current_canvas.0,
                        current_canvas.1,
                    ) {
                        let crop = update_crop_drag(
                            state.mode,
                            state.start_rect,
                            state.start_image,
                            current_image,
                            d.canvas_width as f64,
                            d.canvas_height as f64,
                        );
                        drop(d);
                        pipeline.borrow_mut().crop = Some(crop);
                        overlay.queue_draw();
                    }
                    return;
                }

                if *space.borrow() || *tool.borrow() == ToolKind::Zoom {
                    *ox.borrow_mut() += offset_x - lx;
                    *oy.borrow_mut() += offset_y - ly;
                    *l.borrow_mut() = (offset_x, offset_y);
                    w.queue_draw();
                    overlay.queue_draw();
                }
            });
        }
        {
            let d = drag_dragging.clone();
            let space = drag_space.clone();
            let widget = drag_widget.clone();
            let crop_drag = drag_crop.clone();
            drag_ctrl.connect_drag_end(move |_gesture, _offset_x, _offset_y| {
                *d.borrow_mut() = false;
                *crop_drag.borrow_mut() = None;
                if *space.borrow() {
                    widget.set_cursor_from_name(Some("grab"));
                } else {
                    widget.set_cursor_from_name(None);
                }
            });
        }
        self.widget.add_controller(drag_ctrl);

        let brush_tool = self.active_tool.clone();
        let brush_doc = self.document.clone();
        let brush_zoom = self.zoom.clone();
        let brush_ox = self.offset_x.clone();
        let brush_oy = self.offset_y.clone();
        let brush_painting = self.painting.clone();
        let brush_widget = self.widget.clone();
        let brush_space = self.space_pressed.clone();
        let brush_size = self.brush_size.clone();
        let brush_hardness = self.brush_hardness.clone();
        let brush_opacity = self.brush_opacity.clone();
        let brush_flow = self.brush_flow.clone();
        let brush_last = self.last_paint_point.clone();
        let brush_before_layer = self.paint_before_layer.clone();
        let brush_dirty_regions = self.dirty_regions.clone();

        let click = gtk4::GestureClick::new();
        {
            let tool = brush_tool.clone();
            let doc = brush_doc.clone();
            let zoom = brush_zoom.clone();
            let ox = brush_ox.clone();
            let oy = brush_oy.clone();
            let painting = brush_painting.clone();
            let widget = brush_widget.clone();
            let space = brush_space.clone();
            let bs = brush_size.clone();
            let bh = brush_hardness.clone();
            let bo = brush_opacity.clone();
            let bf = brush_flow.clone();
            let bc = self.brush_color.clone();
            let last = brush_last.clone();
            let before_layer = brush_before_layer.clone();
            let lasso_path = self.lasso_path.clone();
            click.connect_pressed(move |_gesture, n_press, gx, gy| {
                if n_press != 1 || *space.borrow() {
                    return;
                }
                let t = *tool.borrow();
                if t != ToolKind::Brush && t != ToolKind::Eraser && t != ToolKind::ColorPicker && t != ToolKind::Lasso {
                    return;
                }
                
                if t == ToolKind::Lasso {
                    lasso_path.borrow_mut().clear();
                    lasso_path.borrow_mut().push((gx, gy));
                }

                *painting.borrow_mut() = true;
                #[allow(deprecated)]
                let alloc = widget.allocation();
                let canvas_w = alloc.width() as f64;
                let canvas_h = alloc.height() as f64;
                let z = *zoom.borrow();
                let off_x = *ox.borrow();
                let off_y = *oy.borrow();
                let mut d = doc.borrow_mut();
                let is_eraser = t == ToolKind::Eraser;
                let radius = (*bs.borrow() / 2.0).max(0.5);
                *before_layer.borrow_mut() = d.active_layer_id.and_then(|id| d.layer(id).cloned());
                
                if t == ToolKind::ColorPicker {
                    if let Some((ix, iy)) = canvas_to_image(
                        &d, z, off_x, off_y, canvas_w, canvas_h, gx, gy,
                    ) {
                        if let Some(color) = pick_color_at(&d, ix, iy) {
                            *bc.borrow_mut() = color;
                        }
                    }
                } else if let Some(region) = paint_brush_at(
                    &mut d,
                    z,
                    off_x,
                    off_y,
                    canvas_w,
                    canvas_h,
                    gx,
                    gy,
                    is_eraser,
                    radius,
                    *bo.borrow(),
                    *bh.borrow(),
                    *bf.borrow(),
                    *bc.borrow(),
                ) {
                    brush_dirty_regions.borrow_mut().push(region);
                }
                *last.borrow_mut() = Some((gx, gy));
                widget.queue_draw();
            });
        }
        {
            let painting = brush_painting.clone();
            let last = self.last_paint_point.clone();
            let before_layer = self.paint_before_layer.clone();
            let doc = self.document.clone();
            let widget = self.widget.clone();
            let tool = brush_tool.clone();
            let lasso_path = self.lasso_path.clone();
            let zoom = brush_zoom.clone();
            let ox = brush_ox.clone();
            let oy = brush_oy.clone();
            click.connect_released(move |_gesture, _n, _gx, _gy| {
                *painting.borrow_mut() = false;
                *last.borrow_mut() = None;
                if let Some(before) = before_layer.borrow_mut().take() {
                    commit_prepaint_layer_state(&mut doc.borrow_mut(), before);
                    widget.queue_draw();
                }

                if *tool.borrow() == ToolKind::Lasso {
                    let mut d = doc.borrow_mut();
                    let path = lasso_path.borrow();
                    if path.len() > 2 {
                        let mut mask = Mask::new("Selection", d.canvas_width, d.canvas_height);
                        mask.kind = crate::document::MaskKind::Selection;
                        mask.data.fill(0); // Start with nothing selected
                        
                        // Use cairo to fill the path
                        let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, d.canvas_width as i32, d.canvas_height as i32).unwrap();
                        let cr = cairo::Context::new(&surface).unwrap();
                        
                        cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                        let z = *zoom.borrow();
                        let off_x = *ox.borrow();
                        let off_y = *oy.borrow();
                        #[allow(deprecated)]
                        let alloc = widget.allocation();
                        let cw = alloc.width() as f64;
                        let ch = alloc.height() as f64;

                        for (i, p) in path.iter().enumerate() {
                            let Some((ix, iy)) = canvas_to_image(&d, z, off_x, off_y, cw, ch, p.0, p.1) else { continue; };
                            if i == 0 {
                                cr.move_to(ix, iy);
                            } else {
                                cr.line_to(ix, iy);
                            }
                        }
                        cr.close_path();
                        cr.fill().ok();
                        
                        // Copy from surface to mask data
                        let data = surface.data().unwrap();
                        for i in 0..mask.data.len() {
                            // Cairo ARgb32 is pre-multiplied, but since we painted white (1,1,1,1),
                            // we just need the alpha or any channel.
                            mask.data[i] = data[i * 4 + 3];
                        }
                        d.selection = Some(mask);
                    }
                    drop(path);
                    lasso_path.borrow_mut().clear();
                    widget.queue_draw();
                }
            });
        }
        self.widget.add_controller(click);

        let secondary_click = gtk4::GestureClick::new();
        secondary_click.set_button(3);
        {
            let tool = brush_tool.clone();
            let widget = brush_widget.clone();
            let size = self.brush_size.clone();
            let hardness = self.brush_hardness.clone();
            let opacity = self.brush_opacity.clone();
            let color = self.brush_color.clone();
            secondary_click.connect_pressed(move |_gesture, _n, x, y| {
                let t = *tool.borrow();
                if t != ToolKind::Brush && t != ToolKind::Eraser {
                    return;
                }
                show_brush_popover(widget.upcast_ref(), x, y, &size, &hardness, &opacity, &color);
            });
        }
        self.widget.add_controller(secondary_click);

        let motion_tool = self.active_tool.clone();
        let motion_doc = self.document.clone();
        let motion_zoom = self.zoom.clone();
        let motion_ox = self.offset_x.clone();
        let motion_oy = self.offset_y.clone();
        let motion_painting = self.painting.clone();
        let motion_widget = self.widget.clone();
        let motion_space = self.space_pressed.clone();
        let motion_bs = self.brush_size.clone();
        let motion_bh = self.brush_hardness.clone();
        let motion_bo = self.brush_opacity.clone();
        let motion_bf = self.brush_flow.clone();
        let motion_bc = self.brush_color.clone();
        let motion_last = self.last_paint_point.clone();
        let motion_dirty_regions = self.dirty_regions.clone();
        let motion_lasso_path = self.lasso_path.clone();

        let motion_ev = gtk4::EventControllerMotion::new();
        motion_ev.connect_motion(move |_ctrl, gx, gy| {
            if !*motion_painting.borrow() || *motion_space.borrow() {
                return;
            }
            let t = *motion_tool.borrow();
            if t != ToolKind::Brush && t != ToolKind::Eraser && t != ToolKind::ColorPicker && t != ToolKind::Lasso {
                return;
            }
            
            if t == ToolKind::Lasso {
                motion_lasso_path.borrow_mut().push((gx, gy));
                motion_widget.queue_draw();
                return;
            }

            #[allow(deprecated)]
            let alloc = motion_widget.allocation();
            let canvas_w = alloc.width() as f64;
            let canvas_h = alloc.height() as f64;
            let z = *motion_zoom.borrow();
            let off_x = *motion_ox.borrow();
            let off_y = *motion_oy.borrow();
            let mut d = motion_doc.borrow_mut();

            if t == ToolKind::ColorPicker {
                if let Some((ix, iy)) = canvas_to_image(
                    &d, z, off_x, off_y, canvas_w, canvas_h, gx, gy,
                ) {
                    if let Some(color) = pick_color_at(&d, ix, iy) {
                        *motion_bc.borrow_mut() = color;
                    }
                }
            } else {
                let is_eraser = t == ToolKind::Eraser;
                let radius = (*motion_bs.borrow() / 2.0).max(0.5);
                let previous = motion_last.borrow().unwrap_or((gx, gy));
                if let Some(region) = paint_brush_stroke(
                    &mut d,
                    z,
                    off_x,
                    off_y,
                    canvas_w,
                    canvas_h,
                    previous,
                    (gx, gy),
                    is_eraser,
                    radius,
                    *motion_bo.borrow(),
                    *motion_bh.borrow(),
                    *motion_bf.borrow(),
                    *motion_bc.borrow(),
                ) {
                    motion_dirty_regions.borrow_mut().push(region);
                }
            }
            *motion_last.borrow_mut() = Some((gx, gy));
            motion_widget.queue_draw();
        });
        self.widget.add_controller(motion_ev);
    }

    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn queue_redraw(&self) {
        self.widget.queue_draw();
        self.overlay.queue_draw();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_drag_new_normalizes_and_clamps_to_canvas() {
        let crop = update_crop_drag(
            CropDragMode::New,
            CropRect::new(0.0, 0.0, 1.0, 1.0),
            (80.0, 70.0),
            (-10.0, 120.0),
            100.0,
            100.0,
        );

        assert_eq!(crop.x, 0.0);
        assert_eq!(crop.y, 70.0);
        assert_eq!(crop.width, 80.0);
        assert_eq!(crop.height, 30.0);
    }

    #[test]
    fn crop_hit_mode_prefers_handles_before_move() {
        let crop = CropRect::new(10.0, 20.0, 100.0, 80.0);

        assert!(matches!(
            crop_hit_mode(crop, (10.0, 20.0), 3.0),
            CropDragMode::ResizeNw
        ));
        assert!(matches!(
            crop_hit_mode(crop, (50.0, 50.0), 3.0),
            CropDragMode::Move
        ));
        assert!(matches!(
            crop_hit_mode(crop, (200.0, 200.0), 3.0),
            CropDragMode::New
        ));
    }

    #[test]
    fn brush_dab_blends_pixels_and_eraser_reduces_alpha() {
        let mut raster = RasterLayer {
            width: 8,
            height: 8,
            data: vec![255; 8 * 8 * 4],
            offset_x: 0,
            offset_y: 0,
        };
        let idx = (4 * 8 + 4) * 4;

        paint_brush_dab(&mut raster, 4.0, 4.0, false, 2.0, 1.0, 1.0, 1.0);
        assert_eq!(raster.data[idx], 0);
        assert_eq!(raster.data[idx + 1], 0);
        assert_eq!(raster.data[idx + 2], 0);
        assert_eq!(raster.data[idx + 3], 255);

        paint_brush_dab(&mut raster, 4.0, 4.0, true, 2.0, 0.5, 1.0, 1.0);
        assert!(raster.data[idx + 3] < 255);
    }

    #[test]
    fn mask_dab_paints_black_and_eraser_restores_white() {
        let mut mask = Mask::new("Mask", 8, 8);
        let idx = 4 * 8 + 4;

        paint_mask_dab(&mut mask, 4.0, 4.0, false, 2.0, 1.0, 1.0, 1.0);
        assert_eq!(mask.data[idx], 0);

        paint_mask_dab(&mut mask, 4.0, 4.0, true, 2.0, 0.5, 1.0, 1.0);
        assert!(mask.data[idx] > 0);
    }

    #[test]
    fn active_editing_mask_receives_brush_instead_of_layer_pixels() {
        let mut doc = Document::new(8, 8);
        let mut layer = crate::document::Layer::new_raster("Layer", 8, 8, vec![255; 8 * 8 * 4]);
        let mut mask = Mask::new("Mask", 8, 8);
        mask.editing = true;
        layer.mask = Some(mask);
        let layer_id = layer.id;
        doc.add_layer(layer);

        paint_brush_at(
            &mut doc, 1.0, 0.0, 0.0, 8.0, 8.0, 4.0, 4.0, false, 2.0, 1.0, 1.0, 1.0,
        );

        let layer = doc.layer(layer_id).unwrap();
        let mask = layer.mask.as_ref().unwrap();
        assert_eq!(mask.data[4 * 8 + 4], 0);
        let LayerKind::Raster(raster) = &layer.kind else {
            panic!("expected raster layer");
        };
        assert_eq!(raster.data[(4 * 8 + 4) * 4], 255);
    }

    #[test]
    fn committed_brush_change_is_undoable() {
        let mut doc = Document::new(8, 8);
        let layer = crate::document::Layer::new_raster("Layer", 8, 8, vec![255; 8 * 8 * 4]);
        let layer_id = layer.id;
        doc.add_layer(layer);
        let before = doc.layer(layer_id).unwrap().clone();

        paint_brush_at(
            &mut doc, 1.0, 0.0, 0.0, 8.0, 8.0, 4.0, 4.0, false, 2.0, 1.0, 1.0, 1.0,
        );
        commit_prepaint_layer_state(&mut doc, before);

        assert!(doc.undo_stack.can_undo());
        {
            let layer = doc.layer(layer_id).unwrap();
            let LayerKind::Raster(raster) = &layer.kind else {
                panic!("expected raster layer");
            };
            assert_eq!(raster.data[(4 * 8 + 4) * 4], 0);
        }

        let mut undo_stack = std::mem::take(&mut doc.undo_stack);
        undo_stack.undo(&mut doc);
        doc.undo_stack = undo_stack;

        let layer = doc.layer(layer_id).unwrap();
        let LayerKind::Raster(raster) = &layer.kind else {
            panic!("expected raster layer");
        };
        assert_eq!(raster.data[(4 * 8 + 4) * 4], 255);
    }
}
