use crate::document::{Document, LayerKind, UndoStack};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub const PROJECT_EXTENSION: &str = "slate";
pub const PROJECT_FORMAT: &str = "com.slate.editor.project";
pub const PROJECT_VERSION: u32 = 1;
const MAX_PROJECT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_DIMENSION: u32 = 32_768;
const MAX_LAYERS: usize = 1_024;

#[derive(Debug, Serialize, Deserialize)]
struct ProjectFile {
    format: String,
    version: u32,
    document: Document,
}

#[derive(Debug)]
pub enum ProjectError {
    Io(io::Error),
    Json(serde_json::Error),
    UnsupportedVersion(u32),
    Invalid(String),
}

impl fmt::Display for ProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Json(error) => write!(formatter, "Invalid project JSON: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "Project version {version} is newer than this Slate build")
            }
            Self::Invalid(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ProjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::UnsupportedVersion(_) | Self::Invalid(_) => None,
        }
    }
}

impl From<io::Error> for ProjectError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ProjectError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub fn is_project_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(PROJECT_EXTENSION))
}

pub fn encode_project(document: &Document) -> Result<Vec<u8>, ProjectError> {
    validate_document(document)?;
    let project = ProjectFile {
        format: PROJECT_FORMAT.to_string(),
        version: PROJECT_VERSION,
        document: document.clone(),
    };
    Ok(serde_json::to_vec_pretty(&project)?)
}

pub fn decode_project(bytes: &[u8]) -> Result<Document, ProjectError> {
    let project: ProjectFile = serde_json::from_slice(bytes)?;
    if project.format != PROJECT_FORMAT {
        return Err(ProjectError::Invalid(
            "The selected file is not a Slate project".to_string(),
        ));
    }
    if project.version > PROJECT_VERSION {
        return Err(ProjectError::UnsupportedVersion(project.version));
    }
    if project.version == 0 {
        return Err(ProjectError::Invalid(
            "Project version 0 is not supported".to_string(),
        ));
    }

    validate_document(&project.document)?;
    let mut document = project.document;
    document.undo_stack = UndoStack::new();
    document.has_unsaved_changes = false;
    Ok(document)
}

pub fn load_project(path: &Path) -> Result<Document, ProjectError> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_PROJECT_BYTES {
        return Err(ProjectError::Invalid(format!(
            "Project is larger than the {} GiB safety limit",
            MAX_PROJECT_BYTES / 1024 / 1024 / 1024
        )));
    }
    decode_project(&fs::read(path)?)
}

pub fn save_project(path: &Path, document: &Document) -> Result<(), ProjectError> {
    let bytes = encode_project(document)?;
    let temporary_path = temporary_path_for(path);

    let mut file = File::create(&temporary_path)?;
    if let Err(error) = file.write_all(&bytes).and_then(|_| file.sync_all()) {
        let _ = fs::remove_file(&temporary_path);
        return Err(ProjectError::Io(error));
    }

    if let Err(error) = fs::rename(&temporary_path, path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(ProjectError::Io(error));
    }
    Ok(())
}

fn temporary_path_for(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project.slate");
    path.with_file_name(format!(".{file_name}.tmp"))
}

fn checked_pixel_len(width: u32, height: u32, channels: usize) -> Result<usize, ProjectError> {
    if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(ProjectError::Invalid(format!(
            "Invalid pixel dimensions {width}x{height}"
        )));
    }
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(channels))
        .ok_or_else(|| ProjectError::Invalid("Pixel dimensions overflow memory limits".to_string()))
}

fn validate_document(document: &Document) -> Result<(), ProjectError> {
    checked_pixel_len(document.canvas_width, document.canvas_height, 1)?;
    if document.layers.len() > MAX_LAYERS {
        return Err(ProjectError::Invalid(format!(
            "Project contains more than {MAX_LAYERS} layers"
        )));
    }

    let mut layer_ids = HashSet::with_capacity(document.layers.len());
    for layer in &document.layers {
        if !layer_ids.insert(layer.id) {
            return Err(ProjectError::Invalid(
                "Project contains duplicate layer identifiers".to_string(),
            ));
        }
        if !layer.opacity.is_finite() || !(0.0..=1.0).contains(&layer.opacity) {
            return Err(ProjectError::Invalid(format!(
                "Layer '{}' has an invalid opacity",
                layer.name
            )));
        }

        if let LayerKind::Raster(raster) = &layer.kind {
            let expected = checked_pixel_len(raster.width, raster.height, 4)?;
            if raster.data.len() != expected {
                return Err(ProjectError::Invalid(format!(
                    "Layer '{}' has {} bytes but {} were expected",
                    layer.name,
                    raster.data.len(),
                    expected
                )));
            }
        }

        if let Some(mask) = &layer.mask {
            let expected = checked_pixel_len(mask.width, mask.height, 1)?;
            if mask.data.len() != expected {
                return Err(ProjectError::Invalid(format!(
                    "Mask '{}' has {} bytes but {} were expected",
                    mask.name,
                    mask.data.len(),
                    expected
                )));
            }
        }
    }

    if let Some(active_layer_id) = document.active_layer_id {
        if !layer_ids.contains(&active_layer_id) {
            return Err(ProjectError::Invalid(
                "The active layer identifier does not exist in the layer stack".to_string(),
            ));
        }
    }

    if let Some(selection) = &document.selection {
        let expected = checked_pixel_len(selection.width, selection.height, 1)?;
        if selection.data.len() != expected {
            return Err(ProjectError::Invalid(
                "The document selection has invalid pixel data".to_string(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Layer;

    fn sample_document() -> Document {
        let mut document = Document::new(2, 2);
        document.add_layer(Layer::new_raster(
            "Pixels",
            2,
            2,
            vec![255; 2 * 2 * 4],
        ));
        document
    }

    #[test]
    fn project_round_trip_preserves_layers_and_resets_runtime_state() {
        let document = sample_document();
        let encoded = encode_project(&document).expect("encode project");
        let decoded = decode_project(&encoded).expect("decode project");

        assert_eq!(decoded.canvas_width, 2);
        assert_eq!(decoded.canvas_height, 2);
        assert_eq!(decoded.layers.len(), 1);
        assert_eq!(decoded.layers[0].name, "Pixels");
        assert!(!decoded.has_unsaved_changes);
        assert!(!decoded.undo_stack.can_undo());
    }

    #[test]
    fn malformed_raster_payload_is_rejected() {
        let mut document = sample_document();
        let LayerKind::Raster(raster) = &mut document.layers[0].kind else {
            panic!("expected raster layer");
        };
        raster.data.pop();

        let error = encode_project(&document).expect_err("invalid payload should fail");
        assert!(error.to_string().contains("bytes"));
    }

    #[test]
    fn future_project_versions_are_rejected() {
        let document = sample_document();
        let mut value: serde_json::Value =
            serde_json::from_slice(&encode_project(&document).unwrap()).unwrap();
        value["version"] = serde_json::Value::from(PROJECT_VERSION + 1);

        let error = decode_project(&serde_json::to_vec(&value).unwrap())
            .expect_err("future version should fail");
        assert!(matches!(error, ProjectError::UnsupportedVersion(_)));
    }

    #[test]
    fn project_path_detection_is_case_insensitive() {
        assert!(is_project_path(Path::new("poster.slate")));
        assert!(is_project_path(Path::new("poster.SLATE")));
        assert!(!is_project_path(Path::new("poster.png")));
    }
}
