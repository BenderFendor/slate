#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaskId(Uuid);

impl MaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MaskId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mask {
    pub id: MaskId,
    pub name: String,
    pub visible: bool,
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub kind: MaskKind,
    pub linked: bool,
    pub enabled: bool,
    pub editing: bool,
    pub show_on_canvas: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaskKind {
    Transparency,
    Filter,
    Selection,
}

impl Mask {
    pub fn new(name: &str, width: u32, height: u32) -> Self {
        Self {
            id: MaskId::new(),
            name: name.to_string(),
            visible: true,
            data: vec![255u8; (width * height) as usize],
            width,
            height,
            kind: MaskKind::Transparency,
            linked: true,
            enabled: true,
            editing: false,
            show_on_canvas: false,
        }
    }
}
