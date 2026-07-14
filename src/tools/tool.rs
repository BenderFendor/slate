#![allow(dead_code)]

use crate::document::{Document, LayerId};

pub type ToolId = &'static str;

#[derive(Debug, Clone)]
pub struct PointerEvent {
    pub x: f64,
    pub y: f64,
    pub pressure: f32,
    pub button: PointerButton,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl Modifiers {
    pub fn none() -> Self {
        Self {
            shift: false,
            ctrl: false,
            alt: false,
        }
    }
}

pub struct ToolContext<'a> {
    pub document: &'a mut Document,
    pub active_layer: Option<LayerId>,
    pub zoom: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

pub trait Tool: Send {
    fn id(&self) -> ToolId;
    fn name(&self) -> &'static str;
    fn icon_name(&self) -> &'static str;
    fn cursor_name(&self) -> &'static str;

    fn pointer_down(&mut self, ctx: &mut ToolContext, event: &PointerEvent);
    fn pointer_move(&mut self, ctx: &mut ToolContext, event: &PointerEvent);
    fn pointer_up(&mut self, ctx: &mut ToolContext, event: &PointerEvent);
    fn key_press(&mut self, _ctx: &mut ToolContext, _key: &str) {}
    fn cancel(&mut self) {}
    fn is_active(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Brush,
    Eraser,
    Crop,
    Move,
    Zoom,
    Text,
    ColorPicker,
    Fill,
    Clone,
    Heal,
    Blur,
    Sharpen,
    Smudge,
    Path,
    Lasso,
}

impl ToolKind {
    pub fn all() -> &'static [(ToolKind, &'static str, &'static str)] {
        &[
            (ToolKind::Brush, "Brush", "brush-tool-symbolic"),
            (ToolKind::Eraser, "Eraser", "eraser-tool-symbolic"),
            (ToolKind::Move, "Move", "move-tool-symbolic"),
            (ToolKind::Crop, "Crop", "crop-tool-symbolic"),
            (ToolKind::Lasso, "Lasso", "lasso-tool-symbolic"),
            (ToolKind::Zoom, "Zoom", "zoom-tool-symbolic"),
            (ToolKind::Text, "Text", "text-tool-symbolic"),
            (
                ToolKind::ColorPicker,
                "Color Picker",
                "color-picker-tool-symbolic",
            ),
            (ToolKind::Fill, "Fill", "fill-tool-symbolic"),
            (ToolKind::Clone, "Clone Stamp", "clone-tool-symbolic"),
            (ToolKind::Heal, "Heal", "heal-tool-symbolic"),
            (ToolKind::Blur, "Blur", "blur-tool-symbolic"),
            (ToolKind::Sharpen, "Sharpen", "sharpen-tool-symbolic"),
            (ToolKind::Smudge, "Smudge", "smudge-tool-symbolic"),
            (ToolKind::Path, "Path", "path-tool-symbolic"),
        ]
    }
}
