#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Png,
    Jpeg,
    WebP,
}

impl ExportFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Png => "PNG",
            Self::Jpeg => "JPEG",
            Self::WebP => "WebP",
        }
    }

    pub fn extensions() -> &'static [&'static str] {
        &["png", "jpg", "webp"]
    }
}

#[derive(Debug, Clone)]
pub struct ExportParams {
    pub format: ExportFormat,
    pub quality: u8,
    pub strip_metadata: bool,
}

impl Default for ExportParams {
    fn default() -> Self {
        Self {
            format: ExportFormat::Png,
            quality: 90,
            strip_metadata: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ResizeKernel {
    Nearest,
    Linear,
    Cubic,
    Lanczos3,
}

impl ResizeKernel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Nearest => "Nearest",
            Self::Linear => "Linear",
            Self::Cubic => "Cubic",
            Self::Lanczos3 => "Lanczos3",
        }
    }

    pub fn all() -> &'static [ResizeKernel] {
        &[Self::Nearest, Self::Linear, Self::Cubic, Self::Lanczos3]
    }
}

impl Default for ResizeKernel {
    fn default() -> Self {
        Self::Lanczos3
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResizeMode {
    Fit,
    FillCrop,
    Stretch,
}

impl ResizeMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fit => "Fit inside",
            Self::FillCrop => "Fill crop",
            Self::Stretch => "Stretch",
        }
    }

    pub fn all() -> &'static [ResizeMode] {
        &[Self::Fit, Self::FillCrop, Self::Stretch]
    }
}

impl Default for ResizeMode {
    fn default() -> Self {
        Self::FillCrop
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CropRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl CropRect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn aspect_ratio(&self) -> f64 {
        if self.height == 0.0 {
            1.0
        } else {
            self.width / self.height
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ResizeTarget {
    pub width: u32,
    pub height: u32,
    pub mode: ResizeMode,
}

impl ResizeTarget {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            mode: ResizeMode::FillCrop,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PresetTarget {
    Original,
    Custom(u32, u32),
}

impl PresetTarget {
    pub fn presets() -> &'static [(u32, u32, &'static str)] {
        &[
            (1920, 1080, "1920 x 1080"),
            (1280, 720, "1280 x 720"),
            (1080, 1080, "1080 x 1080"),
            (3840, 2160, "3840 x 2160"),
            (800, 600, "800 x 600"),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Rotation {
    None,
    Clockwise90,
    Clockwise180,
    Clockwise270,
}

impl Rotation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "0 deg",
            Self::Clockwise90 => "90 deg",
            Self::Clockwise180 => "180 deg",
            Self::Clockwise270 => "270 deg",
        }
    }

    pub fn all() -> &'static [Rotation] {
        &[
            Self::None,
            Self::Clockwise90,
            Self::Clockwise180,
            Self::Clockwise270,
        ]
    }
}

impl Default for Rotation {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditPipeline {
    pub rotation: Rotation,
    pub crop: Option<CropRect>,
    pub resize: Option<ResizeTarget>,
    pub kernel: ResizeKernel,
    pub preset: PresetTarget,
}

impl Default for EditPipeline {
    fn default() -> Self {
        Self {
            rotation: Rotation::None,
            crop: None,
            resize: None,
            kernel: ResizeKernel::Lanczos3,
            preset: PresetTarget::Custom(1920, 1080),
        }
    }
}
