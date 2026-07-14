#![allow(dead_code)]

use crate::document::{LayerId, LayerKind};
use crate::tools::tool::{PointerButton, PointerEvent, Tool, ToolContext};
use std::collections::VecDeque;

pub struct BrushTool {
    pub radius: f64,
    pub opacity: f32,
    pub hardness: f32,
    pub color: [f32; 4],
    active: bool,
    stroke_points: VecDeque<(f64, f64, f32)>,
    current_layer: Option<LayerId>,
}

impl BrushTool {
    pub fn new() -> Self {
        Self {
            radius: 10.0,
            opacity: 1.0,
            hardness: 0.8,
            color: [0.0, 0.0, 0.0, 1.0],
            active: false,
            stroke_points: VecDeque::new(),
            current_layer: None,
        }
    }
}

impl Tool for BrushTool {
    fn id(&self) -> &'static str {
        "brush"
    }

    fn name(&self) -> &'static str {
        "Brush"
    }

    fn icon_name(&self) -> &'static str {
        "brush-tool-symbolic"
    }

    fn cursor_name(&self) -> &'static str {
        "crosshair"
    }

    fn pointer_down(&mut self, ctx: &mut ToolContext, event: &PointerEvent) {
        if event.button != PointerButton::Left {
            return;
        }
        self.active = true;
        self.current_layer = ctx.active_layer;
        self.stroke_points.clear();
        self.stroke_points
            .push_back((event.x, event.y, event.pressure));

        if let Some(layer_id) = ctx.active_layer {
            if let Some(layer) = ctx.document.layer_mut(layer_id) {
                match &mut layer.kind {
                    LayerKind::Raster(raster) => {
                        let px = event.x as i32;
                        let py = event.y as i32;
                        let r = self.radius as i32;
                        for dy in -r..=r {
                            for dx in -r..=r {
                                let dist = ((dx * dx + dy * dy) as f64).sqrt();
                                if dist > self.radius {
                                    continue;
                                }
                                let sx = px + dx;
                                let sy = py + dy;
                                if sx >= 0
                                    && sy >= 0
                                    && sx < raster.width as i32
                                    && sy < raster.height as i32
                                {
                                    let idx = ((sy * raster.width as i32 + sx) * 4) as usize;
                                    if idx + 3 < raster.data.len() {
                                        let alpha = if self.hardness >= 1.0 {
                                            1.0
                                        } else {
                                            let t = dist / self.radius;
                                            (1.0 - t.powf(1.0 / (self.hardness as f64 + 0.001)))
                                                .max(0.0)
                                        };
                                        let a = self.color[3] * self.opacity * alpha as f32;
                                        raster.data[idx] = (raster.data[idx] as f32 * (1.0 - a)
                                            + self.color[0] * 255.0 * a)
                                            as u8;
                                        raster.data[idx + 1] = (raster.data[idx + 1] as f32
                                            * (1.0 - a)
                                            + self.color[1] * 255.0 * a)
                                            as u8;
                                        raster.data[idx + 2] = (raster.data[idx + 2] as f32
                                            * (1.0 - a)
                                            + self.color[2] * 255.0 * a)
                                            as u8;
                                        raster.data[idx + 3] =
                                            (raster.data[idx + 3] as f32 * (1.0 - a) + 255.0 * a)
                                                as u8;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn pointer_move(&mut self, ctx: &mut ToolContext, event: &PointerEvent) {
        if !self.active {
            return;
        }
        self.stroke_points
            .push_back((event.x, event.y, event.pressure));
        let _last = self.stroke_points.back().copied();

        if let Some(layer_id) = ctx.active_layer {
            if let Some(layer) = ctx.document.layer_mut(layer_id) {
                match &mut layer.kind {
                    LayerKind::Raster(raster) => {
                        let px = event.x as i32;
                        let py = event.y as i32;
                        let r = self.radius as i32;
                        for dy in -r..=r {
                            for dx in -r..=r {
                                let dist = ((dx * dx + dy * dy) as f64).sqrt();
                                if dist > self.radius {
                                    continue;
                                }
                                let sx = px + dx;
                                let sy = py + dy;
                                if sx >= 0
                                    && sy >= 0
                                    && sx < raster.width as i32
                                    && sy < raster.height as i32
                                {
                                    let idx = ((sy * raster.width as i32 + sx) * 4) as usize;
                                    if idx + 3 < raster.data.len() {
                                        let alpha = if self.hardness >= 1.0 {
                                            1.0
                                        } else {
                                            let t = dist / self.radius;
                                            (1.0 - t.powf(1.0 / (self.hardness as f64 + 0.001)))
                                                .max(0.0)
                                        };
                                        let a = self.color[3] * self.opacity * alpha as f32;
                                        raster.data[idx] = (raster.data[idx] as f32 * (1.0 - a)
                                            + self.color[0] * 255.0 * a)
                                            as u8;
                                        raster.data[idx + 1] = (raster.data[idx + 1] as f32
                                            * (1.0 - a)
                                            + self.color[1] * 255.0 * a)
                                            as u8;
                                        raster.data[idx + 2] = (raster.data[idx + 2] as f32
                                            * (1.0 - a)
                                            + self.color[2] * 255.0 * a)
                                            as u8;
                                        raster.data[idx + 3] =
                                            (raster.data[idx + 3] as f32 * (1.0 - a) + 255.0 * a)
                                                as u8;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn pointer_up(&mut self, _ctx: &mut ToolContext, _event: &PointerEvent) {
        self.active = false;
    }

    fn cancel(&mut self) {
        self.active = false;
        self.stroke_points.clear();
    }

    fn is_active(&self) -> bool {
        self.active
    }
}
