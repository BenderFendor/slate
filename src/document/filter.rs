#![allow(dead_code)]

use serde::{Deserialize, Serialize};

pub type FilterId = usize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterNode {
    pub id: FilterId,
    pub name: String,
    pub kind: FilterKind,
    pub enabled: bool,
    pub opacity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterKind {
    Blur(BlurParams),
    BrightnessContrast(BrightnessContrastParams),
    Levels(LevelsParams),
    Curves(CurvesParams),
    HueSaturation(HueSaturationParams),
    ColorBalance(ColorBalanceParams),
    Sharpen(SharpenParams),
    Invert,
    Grayscale,
    Threshold(ThresholdParams),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlurParams {
    pub radius: f32,
    pub kind: BlurKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlurKind {
    Gaussian,
    Box,
    Median,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrightnessContrastParams {
    pub brightness: f32,
    pub contrast: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelsParams {
    pub shadows: f32,
    pub midtones: f32,
    pub highlights: f32,
    pub output_min: f32,
    pub output_max: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurvesParams {
    pub points: Vec<(f32, f32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HueSaturationParams {
    pub hue: f32,
    pub saturation: f32,
    pub lightness: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorBalanceParams {
    pub shadows: [f32; 3],
    pub midtones: [f32; 3],
    pub highlights: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharpenParams {
    pub radius: f32,
    pub amount: f32,
    pub threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdParams {
    pub level: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterStack {
    pub filters: Vec<FilterNode>,
}

impl FilterStack {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    pub fn add(&mut self, filter: FilterNode) {
        self.filters.push(filter);
    }

    pub fn remove(&mut self, id: FilterId) -> Option<FilterNode> {
        if let Some(idx) = self.filters.iter().position(|f| f.id == id) {
            Some(self.filters.remove(idx))
        } else {
            None
        }
    }

    pub fn enabled_filters(&self) -> impl Iterator<Item = &FilterNode> {
        self.filters.iter().filter(|f| f.enabled)
    }
}

impl Default for FilterStack {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) static NEXT_FILTER_ID: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(1);

pub fn next_filter_id() -> FilterId {
    NEXT_FILTER_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl FilterNode {
    pub fn new_blur(radius: f32) -> Self {
        Self {
            id: next_filter_id(),
            name: "Blur".to_string(),
            kind: FilterKind::Blur(BlurParams {
                radius,
                kind: BlurKind::Gaussian,
            }),
            enabled: true,
            opacity: 1.0,
        }
    }

    pub fn new_brightness_contrast(brightness: f32, contrast: f32) -> Self {
        Self {
            id: next_filter_id(),
            name: "Brightness/Contrast".to_string(),
            kind: FilterKind::BrightnessContrast(BrightnessContrastParams {
                brightness,
                contrast,
            }),
            enabled: true,
            opacity: 1.0,
        }
    }

    pub fn new_invert() -> Self {
        Self {
            id: next_filter_id(),
            name: "Invert".to_string(),
            kind: FilterKind::Invert,
            enabled: true,
            opacity: 1.0,
        }
    }

    pub fn new_grayscale() -> Self {
        Self {
            id: next_filter_id(),
            name: "Grayscale".to_string(),
            kind: FilterKind::Grayscale,
            enabled: true,
            opacity: 1.0,
        }
    }
}
