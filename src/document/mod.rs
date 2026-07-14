mod command;
mod filter;
mod layer;
mod mask;

pub use command::*;
pub use layer::*;
pub use mask::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Document {
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub layers: Vec<Layer>,
    pub active_layer_id: Option<LayerId>,
    pub color_config: ColorConfig,
    #[serde(skip)]
    #[allow(dead_code)]
    pub undo_stack: UndoStack,
    pub revision: u64,
    pub file_path: Option<String>,
    pub has_unsaved_changes: bool,
    pub selection: Option<Mask>,
}

impl Clone for Document {
    fn clone(&self) -> Self {
        Self {
            canvas_width: self.canvas_width,
            canvas_height: self.canvas_height,
            layers: self.layers.clone(),
            active_layer_id: self.active_layer_id,
            color_config: self.color_config.clone(),
            undo_stack: UndoStack::new(),
            revision: self.revision,
            file_path: self.file_path.clone(),
            has_unsaved_changes: self.has_unsaved_changes,
            selection: self.selection.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    pub working_space: ColorSpace,
    pub embedded_profile: Option<Vec<u8>>,
    pub bit_depth: BitDepth,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorSpace {
    Srgb,
    AdobeRgb,
    ProPhotoRgb,
    Linear,
    Custom(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BitDepth {
    U8,
    U16,
    Float16,
    Float32,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            working_space: ColorSpace::Srgb,
            embedded_profile: None,
            bit_depth: BitDepth::U8,
        }
    }
}

#[allow(dead_code)]
impl Document {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            canvas_width: width,
            canvas_height: height,
            layers: Vec::new(),
            active_layer_id: None,
            color_config: ColorConfig::default(),
            undo_stack: UndoStack::new(),
            revision: 0,
            file_path: None,
            has_unsaved_changes: false,
            selection: None,
        }
    }

    pub fn add_layer(&mut self, layer: Layer) {
        self.revision += 1;
        self.has_unsaved_changes = true;
        let id = layer.id;
        self.layers.push(layer);
        self.active_layer_id = Some(id);
    }

    pub fn remove_layer(&mut self, id: LayerId) -> Option<Layer> {
        if let Some(idx) = self.layers.iter().position(|l| l.id == id) {
            self.revision += 1;
            self.has_unsaved_changes = true;
            let removed = self.layers.remove(idx);
            if self.active_layer_id == Some(id) {
                self.active_layer_id = self.layers.last().map(|l| l.id);
            }
            Some(removed)
        } else {
            None
        }
    }

    pub fn layer_mut(&mut self, id: LayerId) -> Option<&mut Layer> {
        self.revision += 1;
        self.layers.iter_mut().find(|l| l.id == id)
    }

    pub fn select_layer(&mut self, id: Option<LayerId>) {
        self.active_layer_id = id;
    }

    pub fn layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn move_layer(&mut self, from: usize, to: usize) {
        if from < self.layers.len() && to < self.layers.len() {
            let layer = self.layers.remove(from);
            self.layers.insert(to, layer);
            self.revision += 1;
            self.has_unsaved_changes = true;
        }
    }

    pub fn visible_layers(&self) -> Vec<&Layer> {
        self.layers.iter().filter(|l| l.visible).collect()
    }

    pub fn mark_saved(&mut self) {
        self.has_unsaved_changes = false;
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new(1920, 1080)
    }
}
