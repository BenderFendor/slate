#![allow(dead_code)]

use crate::document::{Document, Layer, LayerId, LayerKind, Mask};

pub trait Command: Send {
    fn name(&self) -> &'static str;
    fn apply(&mut self, doc: &mut Document);
    fn undo(&mut self, doc: &mut Document);
}

pub struct UndoStack {
    undone: Vec<Box<dyn Command>>,
    done: Vec<Box<dyn Command>>,
    max_depth: usize,
}

impl UndoStack {
    pub fn new() -> Self {
        Self {
            undone: Vec::new(),
            done: Vec::new(),
            max_depth: 64,
        }
    }

    pub fn execute(&mut self, cmd: Box<dyn Command>, doc: &mut Document) {
        let mut cmd = cmd;
        cmd.apply(doc);
        self.done.push(cmd);
        if self.done.len() > self.max_depth {
            self.done.remove(0);
        }
        self.undone.clear();
    }

    pub fn undo(&mut self, doc: &mut Document) {
        if let Some(mut cmd) = self.done.pop() {
            cmd.undo(doc);
            self.undone.push(cmd);
        }
    }

    pub fn redo(&mut self, doc: &mut Document) {
        if let Some(mut cmd) = self.undone.pop() {
            cmd.apply(doc);
            self.done.push(cmd);
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.done.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.undone.is_empty()
    }

    pub fn undo_name(&self) -> Option<&str> {
        self.done.last().map(|c| c.name())
    }

    pub fn redo_name(&self) -> Option<&str> {
        self.undone.last().map(|c| c.name())
    }

    pub fn history_names(&self) -> Vec<&'static str> {
        self.done.iter().map(|command| command.name()).collect()
    }
}

impl std::fmt::Debug for UndoStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UndoStack")
            .field("undone_len", &self.undone.len())
            .field("done_len", &self.done.len())
            .field("max_depth", &self.max_depth)
            .finish()
    }
}

impl Clone for UndoStack {
    fn clone(&self) -> Self {
        Self {
            undone: Vec::new(),
            done: Vec::new(),
            max_depth: self.max_depth,
        }
    }
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AddLayerCommand {
    layer: Option<Layer>,
    index: usize,
}

impl AddLayerCommand {
    pub fn new(layer: Layer, index: usize) -> Self {
        Self {
            layer: Some(layer),
            index,
        }
    }
}

impl Command for AddLayerCommand {
    fn name(&self) -> &'static str {
        "Add Layer"
    }

    fn apply(&mut self, doc: &mut Document) {
        if let Some(layer) = self.layer.take() {
            let idx = self.index.min(doc.layers.len());
            let id = layer.id;
            doc.layers.insert(idx, layer);
            doc.active_layer_id = Some(id);
            doc.revision += 1;
            doc.has_unsaved_changes = true;
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if self.index < doc.layers.len() {
            let removed_id = doc.layers[self.index].id;
            self.layer = Some(doc.layers.remove(self.index));
            if doc.active_layer_id == Some(removed_id) {
                doc.active_layer_id = doc.layers.last().map(|layer| layer.id);
            }
            doc.revision += 1;
            doc.has_unsaved_changes = true;
        }
    }
}

pub struct RemoveLayerCommand {
    layer: Option<Layer>,
    index: usize,
}

impl RemoveLayerCommand {
    pub fn new(index: usize) -> Self {
        Self { layer: None, index }
    }
}

impl Command for RemoveLayerCommand {
    fn name(&self) -> &'static str {
        "Remove Layer"
    }

    fn apply(&mut self, doc: &mut Document) {
        if self.index < doc.layers.len() {
            let removed_id = doc.layers[self.index].id;
            self.layer = Some(doc.layers.remove(self.index));
            if doc.active_layer_id == Some(removed_id) {
                doc.active_layer_id = doc.layers.last().map(|layer| layer.id);
            }
            doc.revision += 1;
            doc.has_unsaved_changes = true;
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(layer) = self.layer.take() {
            let id = layer.id;
            doc.layers.insert(self.index, layer);
            doc.active_layer_id = Some(id);
            doc.revision += 1;
            doc.has_unsaved_changes = true;
        }
    }
}

pub struct ModifyLayerCommand {
    layer_id: LayerId,
    old_state: Option<Layer>,
    new_state: Layer,
}

pub struct ReplaceDocumentCommand {
    name: &'static str,
    old_state: Option<Document>,
    new_state: Document,
}

impl ReplaceDocumentCommand {
    pub fn new(name: &'static str, new_state: Document) -> Self {
        Self {
            name,
            old_state: None,
            new_state,
        }
    }
}

impl Command for ReplaceDocumentCommand {
    fn name(&self) -> &'static str {
        self.name
    }

    fn apply(&mut self, doc: &mut Document) {
        self.old_state = Some(std::mem::replace(doc, self.new_state.clone()));
        doc.revision += 1;
        doc.has_unsaved_changes = true;
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(old_state) = self.old_state.take() {
            self.new_state = std::mem::replace(doc, old_state);
            doc.revision += 1;
            doc.has_unsaved_changes = true;
        }
    }
}

pub struct AddLayerMaskCommand {
    layer_id: LayerId,
    new_mask: Option<Mask>,
    old_mask: Option<Mask>,
}

impl AddLayerMaskCommand {
    pub fn new(layer_id: LayerId, mask: Mask) -> Self {
        Self {
            layer_id,
            new_mask: Some(mask),
            old_mask: None,
        }
    }
}

impl Command for AddLayerMaskCommand {
    fn name(&self) -> &'static str {
        "Add Layer Mask"
    }

    fn apply(&mut self, doc: &mut Document) {
        if let (Some(mask), Some(layer)) = (self.new_mask.take(), doc.layer_mut(self.layer_id)) {
            self.old_mask = layer.mask.replace(mask);
            doc.revision += 1;
            doc.has_unsaved_changes = true;
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(layer) = doc.layer_mut(self.layer_id) {
            self.new_mask = layer.mask.take();
            layer.mask = self.old_mask.take();
            doc.revision += 1;
            doc.has_unsaved_changes = true;
        }
    }
}

pub struct RemoveLayerMaskCommand {
    layer_id: LayerId,
    mask: Option<Mask>,
}

pub struct ApplyLayerMaskCommand {
    layer_id: LayerId,
    old_layer: Option<Layer>,
}

impl ApplyLayerMaskCommand {
    pub fn new(layer_id: LayerId) -> Self {
        Self {
            layer_id,
            old_layer: None,
        }
    }
}

impl Command for ApplyLayerMaskCommand {
    fn name(&self) -> &'static str {
        "Apply Layer Mask"
    }

    fn apply(&mut self, doc: &mut Document) {
        if let Some(layer) = doc.layer_mut(self.layer_id) {
            let old_layer = layer.clone();
            let Some(mask) = layer.mask.take() else {
                return;
            };
            let LayerKind::Raster(raster) = &mut layer.kind else {
                layer.mask = Some(mask);
                return;
            };
            if mask.width != raster.width || mask.height != raster.height {
                layer.mask = Some(mask);
                return;
            }

            self.old_layer = Some(old_layer);
            for row in 0..raster.height as usize {
                for col in 0..raster.width as usize {
                    let pixel = (row * raster.width as usize + col) * 4 + 3;
                    let mask_idx = row * raster.width as usize + col;
                    if pixel < raster.data.len() && mask_idx < mask.data.len() {
                        raster.data[pixel] =
                            ((raster.data[pixel] as u16 * mask.data[mask_idx] as u16) / 255) as u8;
                    }
                }
            }
            doc.revision += 1;
            doc.has_unsaved_changes = true;
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(old_layer) = self.old_layer.take() {
            if let Some(idx) = doc
                .layers
                .iter()
                .position(|layer| layer.id == self.layer_id)
            {
                doc.layers[idx] = old_layer;
                doc.revision += 1;
                doc.has_unsaved_changes = true;
            }
        }
    }
}

impl RemoveLayerMaskCommand {
    pub fn new(layer_id: LayerId) -> Self {
        Self {
            layer_id,
            mask: None,
        }
    }
}

impl Command for RemoveLayerMaskCommand {
    fn name(&self) -> &'static str {
        "Remove Layer Mask"
    }

    fn apply(&mut self, doc: &mut Document) {
        if let Some(layer) = doc.layer_mut(self.layer_id) {
            self.mask = layer.mask.take();
            doc.revision += 1;
            doc.has_unsaved_changes = true;
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let (Some(mask), Some(layer)) = (self.mask.take(), doc.layer_mut(self.layer_id)) {
            layer.mask = Some(mask);
            doc.revision += 1;
            doc.has_unsaved_changes = true;
        }
    }
}

impl ModifyLayerCommand {
    pub fn new(layer_id: LayerId, new_state: Layer) -> Self {
        Self {
            layer_id,
            old_state: None,
            new_state,
        }
    }
}

impl Command for ModifyLayerCommand {
    fn name(&self) -> &'static str {
        "Modify Layer"
    }

    fn apply(&mut self, doc: &mut Document) {
        if let Some(idx) = doc.layers.iter().position(|l| l.id == self.layer_id) {
            self.old_state = Some(std::mem::replace(
                &mut doc.layers[idx],
                self.new_state.clone(),
            ));
            doc.revision += 1;
            doc.has_unsaved_changes = true;
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let (Some(idx), Some(old)) = (
            doc.layers.iter().position(|l| l.id == self.layer_id),
            self.old_state.take(),
        ) {
            self.new_state = std::mem::replace(&mut doc.layers[idx], old);
            doc.revision += 1;
            doc.has_unsaved_changes = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Layer;

    #[test]
    fn mask_commands_round_trip_through_undo_redo() {
        let mut doc = Document::new(4, 4);
        let layer = Layer::new_raster("Layer", 4, 4, vec![255; 4 * 4 * 4]);
        let layer_id = layer.id;
        doc.add_layer(layer);

        let mut stack = UndoStack::new();
        stack.execute(
            Box::new(AddLayerMaskCommand::new(
                layer_id,
                Mask::new("Layer Mask", 4, 4),
            )),
            &mut doc,
        );

        assert!(doc
            .layer(layer_id)
            .and_then(|layer| layer.mask.as_ref())
            .is_some());

        stack.undo(&mut doc);
        assert!(doc
            .layer(layer_id)
            .and_then(|layer| layer.mask.as_ref())
            .is_none());

        stack.redo(&mut doc);
        assert!(doc
            .layer(layer_id)
            .and_then(|layer| layer.mask.as_ref())
            .is_some());

        stack.execute(Box::new(RemoveLayerMaskCommand::new(layer_id)), &mut doc);
        assert!(doc
            .layer(layer_id)
            .and_then(|layer| layer.mask.as_ref())
            .is_none());

        stack.undo(&mut doc);
        assert!(doc
            .layer(layer_id)
            .and_then(|layer| layer.mask.as_ref())
            .is_some());
    }

    #[test]
    fn apply_mask_multiplies_alpha_and_can_undo() {
        let mut doc = Document::new(1, 1);
        let mut layer = Layer::new_raster("Layer", 1, 1, vec![10, 20, 30, 200]);
        let layer_id = layer.id;
        let mut mask = Mask::new("Layer Mask", 1, 1);
        mask.data[0] = 128;
        layer.mask = Some(mask);
        doc.add_layer(layer);

        let mut stack = UndoStack::new();
        stack.execute(Box::new(ApplyLayerMaskCommand::new(layer_id)), &mut doc);

        let layer = doc.layer(layer_id).unwrap();
        assert!(layer.mask.is_none());
        let LayerKind::Raster(raster) = &layer.kind else {
            panic!("expected raster layer");
        };
        assert_eq!(raster.data[3], 100);

        stack.undo(&mut doc);
        let layer = doc.layer(layer_id).unwrap();
        assert!(layer.mask.is_some());
        let LayerKind::Raster(raster) = &layer.kind else {
            panic!("expected raster layer");
        };
        assert_eq!(raster.data[3], 200);
    }

    #[test]
    fn replace_document_command_round_trips_canvas_state() {
        let mut doc = Document::new(2, 2);
        let mut next = doc.clone();
        next.canvas_width = 4;
        next.canvas_height = 3;

        let mut stack = UndoStack::new();
        stack.execute(
            Box::new(ReplaceDocumentCommand::new("Resize Canvas", next)),
            &mut doc,
        );

        assert_eq!((doc.canvas_width, doc.canvas_height), (4, 3));
        stack.undo(&mut doc);
        assert_eq!((doc.canvas_width, doc.canvas_height), (2, 2));
        stack.redo(&mut doc);
        assert_eq!((doc.canvas_width, doc.canvas_height), (4, 3));
    }
}
