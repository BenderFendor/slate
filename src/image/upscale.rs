use crate::document::Document;
use crate::document::LayerKind;
use libvips::ops;
use std::error::Error;

pub fn upscale_document(doc: &mut Document, scale_factor: f64) -> Result<(), Box<dyn Error>> {
    doc.canvas_width = (doc.canvas_width as f64 * scale_factor).round() as u32;
    doc.canvas_height = (doc.canvas_height as f64 * scale_factor).round() as u32;

    for layer in &mut doc.layers {
        if let LayerKind::Raster(raster) = &mut layer.kind {
            let img = libvips::VipsImage::new_from_memory(
                &raster.data,
                raster.width as i32,
                raster.height as i32,
                4,
                ops::BandFormat::Uchar,
            )?;

            let upscaled = ops::resize(&img, scale_factor)?;

            raster.width = upscaled.get_width() as u32;
            raster.height = upscaled.get_height() as u32;
            raster.data = upscaled.image_write_to_memory();
            raster.offset_x = (raster.offset_x as f64 * scale_factor).round() as i32;
            raster.offset_y = (raster.offset_y as f64 * scale_factor).round() as i32;
        }
    }

    doc.revision += 1;
    doc.has_unsaved_changes = true;
    Ok(())
}
