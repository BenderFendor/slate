#![allow(dead_code)]

use crate::document::filter::FilterStack;
use crate::document::mask::Mask;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LayerId(Uuid);

impl LayerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for LayerId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl BlendMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Multiply => "Multiply",
            Self::Screen => "Screen",
            Self::Overlay => "Overlay",
            Self::Darken => "Darken",
            Self::Lighten => "Lighten",
            Self::ColorDodge => "Color Dodge",
            Self::ColorBurn => "Color Burn",
            Self::HardLight => "Hard Light",
            Self::SoftLight => "Soft Light",
            Self::Difference => "Difference",
            Self::Exclusion => "Exclusion",
            Self::Hue => "Hue",
            Self::Saturation => "Saturation",
            Self::Color => "Color",
            Self::Luminosity => "Luminosity",
        }
    }
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub mask: Option<Mask>,
    pub kind: LayerKind,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayerKind {
    Raster(RasterLayer),
    Text(TextLayer),
    Fill(FillLayer),
    Group(Vec<LayerId>),
    Adjustment(FilterStack),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RasterLayer {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub offset_x: i32,
    pub offset_y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextLayer {
    pub text: String,
    pub font_family: String,
    pub font_size: f32,
    pub color: [f32; 4],
    pub box_rect: (f64, f64, f64, f64),
    pub alignment: TextAlign,
    pub line_spacing: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillLayer {
    pub color: [f32; 4],
}

impl Layer {
    pub fn new_raster(name: &str, width: u32, height: u32, data: Vec<u8>) -> Self {
        Self {
            id: LayerId::new(),
            name: name.to_string(),
            visible: true,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            mask: None,
            kind: LayerKind::Raster(RasterLayer {
                width,
                height,
                data,
                offset_x: 0,
                offset_y: 0,
            }),
            locked: false,
        }
    }

    #[allow(dead_code)]
    pub fn new_text(name: &str) -> Self {
        Self {
            id: LayerId::new(),
            name: name.to_string(),
            visible: true,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            mask: None,
            kind: LayerKind::Text(TextLayer {
                text: String::new(),
                font_family: "Sans".to_string(),
                font_size: 24.0,
                color: [0.0, 0.0, 0.0, 1.0],
                box_rect: (100.0, 100.0, 400.0, 200.0),
                alignment: TextAlign::Left,
                line_spacing: 1.2,
            }),
            locked: false,
        }
    }

    #[allow(dead_code)]
    pub fn new_group(name: &str) -> Self {
        Self {
            id: LayerId::new(),
            name: name.to_string(),
            visible: true,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            mask: None,
            kind: LayerKind::Group(Vec::new()),
            locked: false,
        }
    }
}
