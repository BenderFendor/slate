use crate::document::{BlendMode, Document, Layer, LayerKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlattenedFrame {
    pub width: u32,
    pub height: u32,
    pub pixels_bgra: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RenderLayerSnapshot {
    pub width: u32,
    pub height: u32,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub pixels_bgra: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RenderFrameSnapshot {
    pub revision: u64,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub layers: Vec<RenderLayerSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelRect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn union(self, other: Self) -> Self {
        let left = self.x.min(other.x);
        let top = self.y.min(other.y);
        let right = self
            .x
            .saturating_add(self.width)
            .max(other.x.saturating_add(other.width));
        let bottom = self
            .y
            .saturating_add(self.height)
            .max(other.y.saturating_add(other.height));
        Self::new(left, top, right - left, bottom - top)
    }
}

pub fn build_render_frame(document: &Document) -> RenderFrameSnapshot {
    let layers = document
        .layers
        .iter()
        .filter_map(build_layer_snapshot)
        .collect();

    RenderFrameSnapshot {
        revision: document.revision,
        canvas_width: document.canvas_width,
        canvas_height: document.canvas_height,
        layers,
    }
}

pub fn flatten_frame_bgra(frame: &RenderFrameSnapshot) -> FlattenedFrame {
    if let Some(flattened) = flatten_single_normal_layer(frame) {
        return flattened;
    }

    let mut pixels_bgra = vec![0u8; frame.canvas_width as usize * frame.canvas_height as usize * 4];

    for layer in &frame.layers {
        let width = layer.width.min(frame.canvas_width) as usize;
        let height = layer.height.min(frame.canvas_height) as usize;
        for row in 0..height {
            for col in 0..width {
                let src = (row * layer.width as usize + col) * 4;
                let dst = (row * frame.canvas_width as usize + col) * 4;
                if src + 3 >= layer.pixels_bgra.len() || dst + 3 >= pixels_bgra.len() {
                    continue;
                }

                let src_alpha =
                    ((layer.pixels_bgra[src + 3] as f32 / 255.0) * layer.opacity).clamp(0.0, 1.0);
                if src_alpha <= 0.0 {
                    continue;
                }
                let dst_alpha = pixels_bgra[dst + 3] as f32 / 255.0;
                let out_alpha = src_alpha + dst_alpha * (1.0 - src_alpha);
                if out_alpha <= 0.0 {
                    continue;
                }

                for channel in 0..3 {
                    let src_value = layer.pixels_bgra[src + channel] as f32 / 255.0;
                    let dst_value = pixels_bgra[dst + channel] as f32 / 255.0;
                    let blended = blend_channel(src_value, dst_value, layer.blend_mode);
                    let out_value = (blended * src_alpha
                        + dst_value * dst_alpha * (1.0 - src_alpha))
                        / out_alpha;
                    pixels_bgra[dst + channel] = (out_value * 255.0).round() as u8;
                }
                pixels_bgra[dst + 3] = (out_alpha * 255.0).round() as u8;
            }
        }
    }

    FlattenedFrame {
        width: frame.canvas_width,
        height: frame.canvas_height,
        pixels_bgra,
    }
}

fn flatten_single_normal_layer(frame: &RenderFrameSnapshot) -> Option<FlattenedFrame> {
    let [layer] = frame.layers.as_slice() else {
        return None;
    };
    if layer.opacity != 1.0 || layer.blend_mode != BlendMode::Normal {
        return None;
    }

    if layer.width == frame.canvas_width && layer.height == frame.canvas_height {
        return Some(FlattenedFrame {
            width: frame.canvas_width,
            height: frame.canvas_height,
            pixels_bgra: layer.pixels_bgra.clone(),
        });
    }

    let mut pixels_bgra = vec![0u8; frame.canvas_width as usize * frame.canvas_height as usize * 4];
    let copy_width = layer.width.min(frame.canvas_width) as usize;
    let copy_height = layer.height.min(frame.canvas_height) as usize;
    for row in 0..copy_height {
        let src_start = row * layer.width as usize * 4;
        let src_end = src_start + copy_width * 4;
        let dst_start = row * frame.canvas_width as usize * 4;
        let dst_end = dst_start + copy_width * 4;
        pixels_bgra[dst_start..dst_end].copy_from_slice(&layer.pixels_bgra[src_start..src_end]);
    }

    Some(FlattenedFrame {
        width: frame.canvas_width,
        height: frame.canvas_height,
        pixels_bgra,
    })
}

pub fn flatten_document_region_bgra(document: &Document, rect: PixelRect, pixels_bgra: &mut [u8]) {
    let canvas_width = document.canvas_width as usize;
    let canvas_height = document.canvas_height as usize;
    if pixels_bgra.len() < canvas_width.saturating_mul(canvas_height).saturating_mul(4) {
        return;
    }

    let end_x = rect.x.saturating_add(rect.width).min(document.canvas_width) as usize;
    let end_y = rect
        .y
        .saturating_add(rect.height)
        .min(document.canvas_height) as usize;

    for row in rect.y as usize..end_y {
        for col in rect.x as usize..end_x {
            let dst = (row * canvas_width + col) * 4;
            pixels_bgra[dst..dst + 4].fill(0);

            for layer in &document.layers {
                if !layer.visible || layer.opacity < 0.01 {
                    continue;
                }
                let LayerKind::Raster(raster) = &layer.kind else {
                    continue;
                };
                if row >= raster.height as usize || col >= raster.width as usize {
                    continue;
                }

                let src = (row * raster.width as usize + col) * 4;
                if src + 3 >= raster.data.len() {
                    continue;
                }

                let mask_alpha = layer
                    .mask
                    .as_ref()
                    .and_then(|mask| {
                        if mask.enabled
                            && mask.width == raster.width
                            && mask.height == raster.height
                        {
                            mask.data.get(row * raster.width as usize + col).copied()
                        } else {
                            None
                        }
                    })
                    .unwrap_or(255);

                let src_alpha = ((raster.data[src + 3] as f32 / 255.0)
                    * (mask_alpha as f32 / 255.0)
                    * layer.opacity)
                    .clamp(0.0, 1.0);
                if src_alpha <= 0.0 {
                    continue;
                }

                let dst_alpha = pixels_bgra[dst + 3] as f32 / 255.0;
                let out_alpha = src_alpha + dst_alpha * (1.0 - src_alpha);
                if out_alpha <= 0.0 {
                    continue;
                }

                let src_bgra = [raster.data[src + 2], raster.data[src + 1], raster.data[src]];
                for channel in 0..3 {
                    let src_value = src_bgra[channel] as f32 / 255.0;
                    let dst_value = pixels_bgra[dst + channel] as f32 / 255.0;
                    let blended = blend_channel(src_value, dst_value, layer.blend_mode);
                    let out_value = (blended * src_alpha
                        + dst_value * dst_alpha * (1.0 - src_alpha))
                        / out_alpha;
                    pixels_bgra[dst + channel] = (out_value * 255.0).round() as u8;
                }
                pixels_bgra[dst + 3] = (out_alpha * 255.0).round() as u8;
            }
        }
    }
}

fn build_layer_snapshot(layer: &Layer) -> Option<RenderLayerSnapshot> {
    if !layer.visible || layer.opacity < 0.01 {
        return None;
    }

    let LayerKind::Raster(raster) = &layer.kind else {
        return None;
    };

    let mut pixels_bgra = vec![0u8; raster.width as usize * raster.height as usize * 4];
    for row in 0..raster.height as usize {
        for col in 0..raster.width as usize {
            let src = (row * raster.width as usize + col) * 4;
            let dst = src;
            if src + 3 >= raster.data.len() {
                continue;
            }

            let mask_alpha = layer
                .mask
                .as_ref()
                .and_then(|mask| {
                    if mask.enabled && mask.width == raster.width && mask.height == raster.height {
                        mask.data.get(row * raster.width as usize + col).copied()
                    } else {
                        None
                    }
                })
                .unwrap_or(255);

            pixels_bgra[dst] = raster.data[src + 2];
            pixels_bgra[dst + 1] = raster.data[src + 1];
            pixels_bgra[dst + 2] = raster.data[src];
            pixels_bgra[dst + 3] = ((raster.data[src + 3] as u16 * mask_alpha as u16) / 255) as u8;
        }
    }

    Some(RenderLayerSnapshot {
        width: raster.width,
        height: raster.height,
        opacity: layer.opacity,
        blend_mode: layer.blend_mode,
        pixels_bgra,
    })
}

fn blend_channel(src: f32, dst: f32, mode: BlendMode) -> f32 {
    match mode {
        BlendMode::Normal => src,
        BlendMode::Multiply => src * dst,
        BlendMode::Screen => 1.0 - (1.0 - src) * (1.0 - dst),
        BlendMode::Overlay => {
            if dst <= 0.5 {
                2.0 * src * dst
            } else {
                1.0 - 2.0 * (1.0 - src) * (1.0 - dst)
            }
        }
        BlendMode::Darken => src.min(dst),
        BlendMode::Lighten => src.max(dst),
        BlendMode::ColorDodge => {
            if src >= 1.0 {
                1.0
            } else {
                (dst / (1.0 - src)).min(1.0)
            }
        }
        BlendMode::ColorBurn => {
            if src <= 0.0 {
                0.0
            } else {
                1.0 - ((1.0 - dst) / src).min(1.0)
            }
        }
        BlendMode::HardLight => {
            if src <= 0.5 {
                2.0 * src * dst
            } else {
                1.0 - 2.0 * (1.0 - src) * (1.0 - dst)
            }
        }
        BlendMode::SoftLight => {
            if src <= 0.5 {
                dst - (1.0 - 2.0 * src) * dst * (1.0 - dst)
            } else {
                let d = if dst <= 0.25 {
                    ((16.0 * dst - 12.0) * dst + 4.0) * dst
                } else {
                    dst.sqrt()
                };
                dst + (2.0 * src - 1.0) * (d - dst)
            }
        }
        BlendMode::Difference => (dst - src).abs(),
        BlendMode::Exclusion => dst + src - 2.0 * dst * src,
        BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity => src,
    }
    .clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Layer, Mask};
    use std::time::{Duration, Instant};

    #[test]
    fn render_frame_skips_hidden_and_non_raster_layers() {
        let mut doc = Document::new(2, 2);
        let mut hidden = Layer::new_raster("Hidden", 2, 2, vec![255; 16]);
        hidden.visible = false;
        doc.add_layer(hidden);
        doc.add_layer(Layer::new_text("Text"));

        let frame = build_render_frame(&doc);

        assert!(frame.layers.is_empty());
    }

    #[test]
    fn render_frame_outputs_bgra_with_enabled_mask_alpha() {
        let mut doc = Document::new(1, 1);
        let mut layer = Layer::new_raster("Layer", 1, 1, vec![10, 20, 30, 200]);
        let mut mask = Mask::new("Mask", 1, 1);
        mask.data[0] = 128;
        layer.mask = Some(mask);
        doc.add_layer(layer);

        let frame = build_render_frame(&doc);
        let layer = &frame.layers[0];

        assert_eq!(layer.pixels_bgra[0], 30);
        assert_eq!(layer.pixels_bgra[1], 20);
        assert_eq!(layer.pixels_bgra[2], 10);
        assert_eq!(layer.pixels_bgra[3], 100);
    }

    #[test]
    fn flatten_frame_composites_layer_opacity() {
        let mut doc = Document::new(1, 1);
        let mut bottom = Layer::new_raster("Bottom", 1, 1, vec![255, 0, 0, 255]);
        bottom.opacity = 1.0;
        let mut top = Layer::new_raster("Top", 1, 1, vec![0, 0, 255, 255]);
        top.opacity = 0.5;
        doc.add_layer(bottom);
        doc.add_layer(top);

        let frame = build_render_frame(&doc);
        let flattened = flatten_frame_bgra(&frame);

        assert_eq!(flattened.width, 1);
        assert_eq!(flattened.height, 1);
        assert!(flattened.pixels_bgra[0] > 120);
        assert_eq!(flattened.pixels_bgra[1], 0);
        assert!(flattened.pixels_bgra[2] > 120);
        assert_eq!(flattened.pixels_bgra[3], 255);
    }

    #[test]
    fn flatten_frame_applies_layer_blend_mode() {
        let mut doc = Document::new(1, 1);
        let bottom = Layer::new_raster("Bottom", 1, 1, vec![128, 128, 128, 255]);
        let mut top = Layer::new_raster("Top", 1, 1, vec![128, 128, 128, 255]);
        top.blend_mode = BlendMode::Multiply;
        doc.add_layer(bottom);
        doc.add_layer(top);

        let frame = build_render_frame(&doc);
        let flattened = flatten_frame_bgra(&frame);

        assert_eq!(flattened.pixels_bgra[0], 64);
        assert_eq!(flattened.pixels_bgra[1], 64);
        assert_eq!(flattened.pixels_bgra[2], 64);
        assert_eq!(flattened.pixels_bgra[3], 255);
    }

    #[test]
    fn flatten_frame_copies_single_normal_layer_without_blending() {
        let mut doc = Document::new(2, 1);
        doc.add_layer(Layer::new_raster(
            "Layer",
            2,
            1,
            vec![10, 20, 30, 255, 40, 50, 60, 255],
        ));

        let flattened = flatten_frame_bgra(&build_render_frame(&doc));

        assert_eq!(
            flattened.pixels_bgra,
            vec![30, 20, 10, 255, 60, 50, 40, 255]
        );
    }

    #[test]
    fn flatten_document_region_matches_full_frame() {
        let mut doc = Document::new(2, 2);
        doc.add_layer(Layer::new_raster(
            "Bottom",
            2,
            2,
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        ));
        let mut top = Layer::new_raster("Top", 2, 2, vec![255; 16]);
        top.opacity = 0.5;
        doc.add_layer(top);

        let full = flatten_frame_bgra(&build_render_frame(&doc));
        let mut region = vec![0u8; 16];
        flatten_document_region_bgra(&doc, PixelRect::new(1, 0, 1, 2), &mut region);

        for row in 0..2 {
            let idx = (row * 2 + 1) * 4;
            assert_eq!(&region[idx..idx + 4], &full.pixels_bgra[idx..idx + 4]);
        }
    }

    #[test]
    #[ignore = "manual performance probe"]
    fn profile_flatten_costs() {
        for (width, height) in [(1920, 1080), (3840, 2160)] {
            let doc = opaque_test_document(width, height);
            let full = timed(5, || {
                let frame = build_render_frame(&doc);
                let flattened = flatten_frame_bgra(&frame);
                std::hint::black_box(flattened);
            });

            let mut region_pixels = vec![0u8; width as usize * height as usize * 4];
            let dirty = timed(200, || {
                flatten_document_region_bgra(
                    &doc,
                    PixelRect::new(width / 2, height / 2, 64, 64),
                    &mut region_pixels,
                );
                std::hint::black_box(&region_pixels);
            });

            println!(
                "{width}x{height}: full flatten avg {:?}, 64x64 dirty avg {:?}",
                full, dirty
            );
        }
    }

    fn opaque_test_document(width: u32, height: u32) -> Document {
        let mut doc = Document::new(width, height);
        doc.add_layer(Layer::new_raster(
            "Layer",
            width,
            height,
            vec![255; width as usize * height as usize * 4],
        ));
        doc
    }

    fn timed(iterations: u32, mut f: impl FnMut()) -> Duration {
        let start = Instant::now();
        for _ in 0..iterations {
            f();
        }
        start.elapsed() / iterations
    }
}
