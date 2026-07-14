#![allow(dead_code)]

use crate::image::pipeline::{
    CropRect, EditPipeline, ExportFormat, ExportParams, ResizeKernel, ResizeMode, ResizeTarget,
    Rotation,
};

pub enum PipelineResult {
    Success(Vec<u8>, u32, u32),
    Error(String),
}

pub fn execute_pipeline(
    input_data: &[u8],
    pipeline: &EditPipeline,
    params: &ExportParams,
) -> PipelineResult {
    let img = match libvips::VipsImage::new_from_buffer(input_data, "") {
        Ok(img) => img,
        Err(e) => return PipelineResult::Error(format!("Failed to load image: {}", e)),
    };

    execute_pipeline_image(img, pipeline, params)
}

pub fn execute_pipeline_rgba(
    input_rgba: &[u8],
    width: u32,
    height: u32,
    pipeline: &EditPipeline,
    params: &ExportParams,
) -> PipelineResult {
    let img = match libvips::VipsImage::new_from_memory(
        input_rgba,
        width as i32,
        height as i32,
        4,
        libvips::ops::BandFormat::Uchar,
    ) {
        Ok(img) => img,
        Err(e) => return PipelineResult::Error(format!("Failed to load raw image: {}", e)),
    };

    execute_pipeline_image(img, pipeline, params)
}

fn execute_pipeline_image(
    img: libvips::VipsImage,
    pipeline: &EditPipeline,
    params: &ExportParams,
) -> PipelineResult {
    let mut current = img;

    if pipeline.rotation != Rotation::None {
        current = match apply_rotation(current, pipeline.rotation) {
            Ok(img) => img,
            Err(e) => return PipelineResult::Error(format!("Rotation failed: {}", e)),
        };
    }

    if let Some(crop) = &pipeline.crop {
        current = match apply_crop(current, crop) {
            Ok(img) => img,
            Err(e) => return PipelineResult::Error(format!("Crop failed: {}", e)),
        };
    }

    if let Some(resize) = &pipeline.resize {
        current = match apply_resize(current, resize, pipeline.kernel) {
            Ok(img) => img,
            Err(e) => return PipelineResult::Error(format!("Resize failed: {}", e)),
        };
    }

    let format_str = match params.format {
        ExportFormat::Png => ".png",
        ExportFormat::Jpeg => ".jpg",
        ExportFormat::WebP => ".webp",
    };

    let save_opts = match params.format {
        ExportFormat::Jpeg | ExportFormat::WebP => {
            let q = params.quality.min(100).max(1);
            format!("[Q={}]", q)
        }
        ExportFormat::Png => String::new(),
    };

    let filename = format!("slate_export{}{}", format_str, save_opts);

    match current.image_write_to_buffer(&filename) {
        Ok(buf) => {
            let w = current.get_width() as u32;
            let h = current.get_height() as u32;
            PipelineResult::Success(buf, w, h)
        }
        Err(e) => PipelineResult::Error(format!("Export failed: {}", e)),
    }
}

fn apply_rotation(
    img: libvips::VipsImage,
    rotation: Rotation,
) -> Result<libvips::VipsImage, String> {
    use libvips::ops;
    let angle = match rotation {
        Rotation::None => return Ok(img),
        Rotation::Clockwise90 => ops::Angle::D90,
        Rotation::Clockwise180 => ops::Angle::D180,
        Rotation::Clockwise270 => ops::Angle::D270,
    };
    ops::rot(&img, angle).map_err(|e| e.to_string())
}

fn apply_crop(img: libvips::VipsImage, crop: &CropRect) -> Result<libvips::VipsImage, String> {
    use libvips::ops;
    let x = crop.x.max(0.0).min((img.get_width() - 1) as f64) as i32;
    let y = crop.y.max(0.0).min((img.get_height() - 1) as f64) as i32;
    let w = (crop.width as i32).min(img.get_width() - x);
    let h = (crop.height as i32).min(img.get_height() - y);
    ops::extract_area(&img, x, y, w, h).map_err(|e| e.to_string())
}

fn apply_resize(
    img: libvips::VipsImage,
    target: &ResizeTarget,
    kernel: ResizeKernel,
) -> Result<libvips::VipsImage, String> {
    use libvips::ops;

    let img_w = img.get_width() as f64;
    let img_h = img.get_height() as f64;
    let tgt_w = target.width as f64;
    let tgt_h = target.height as f64;

    let vips_kernel = match kernel {
        ResizeKernel::Nearest => ops::Kernel::Nearest,
        ResizeKernel::Linear => ops::Kernel::Linear,
        ResizeKernel::Cubic => ops::Kernel::Cubic,
        ResizeKernel::Lanczos3 => ops::Kernel::Lanczos3,
    };

    match target.mode {
        ResizeMode::Stretch => {
            let scale_x = tgt_w / img_w;
            let _ = vips_kernel;
            let resized = ops::resize(&img, scale_x).map_err(|e| e.to_string())?;
            let rw = resized.get_width() as f64;
            if (rw - tgt_w).abs() > 1.0 {
                ops::extract_area(&resized, 0, 0, tgt_w as i32, tgt_h as i32)
                    .map_err(|e| e.to_string())
            } else {
                Ok(resized)
            }
        }
        ResizeMode::Fit => {
            let scale = (tgt_w / img_w).min(tgt_h / img_h);
            ops::resize(&img, scale).map_err(|e| e.to_string())
        }
        ResizeMode::FillCrop => {
            let scale = (tgt_w / img_w).max(tgt_h / img_h);
            let resized = ops::resize(&img, scale).map_err(|e| e.to_string())?;
            let rw = resized.get_width() as f64;
            let rh = resized.get_height() as f64;
            let cx = (rw - tgt_w) / 2.0;
            let cy = (rh - tgt_h) / 2.0;
            ops::extract_area(&resized, cx as i32, cy as i32, tgt_w as i32, tgt_h as i32)
                .map_err(|e| e.to_string())
        }
    }
}

pub fn generate_preview(data: &[u8], max_size: u32) -> PipelineResult {
    let img = match libvips::VipsImage::new_from_buffer(data, "") {
        Ok(img) => img,
        Err(e) => return PipelineResult::Error(format!("Failed to load: {}", e)),
    };

    let w = img.get_width();
    let h = img.get_height();
    let scale = if w > h {
        max_size as f64 / w as f64
    } else {
        max_size as f64 / h as f64
    };

    let scale = scale.min(1.0);

    let resized = match libvips::ops::resize(&img, scale) {
        Ok(img) => img,
        Err(e) => return PipelineResult::Error(e.to_string()),
    };

    match resized.image_write_to_buffer(".png") {
        Ok(buf) => {
            let w = resized.get_width() as u32;
            let h = resized.get_height() as u32;
            PipelineResult::Success(buf, w, h)
        }
        Err(e) => PipelineResult::Error(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_pipeline_rgba_encodes_png() {
        let pixels = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];
        let pipeline = EditPipeline::default();
        let params = ExportParams {
            format: ExportFormat::Png,
            quality: 90,
            strip_metadata: true,
        };

        match execute_pipeline_rgba(&pixels, 2, 2, &pipeline, &params) {
            PipelineResult::Success(buf, width, height) => {
                assert_eq!(width, 2);
                assert_eq!(height, 2);
                assert!(buf.starts_with(b"\x89PNG"));
            }
            PipelineResult::Error(error) => panic!("unexpected export error: {error}"),
        }
    }
}
