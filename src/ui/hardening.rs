use crate::document::{
    is_project_path, load_project, save_project, Document, Layer, PROJECT_EXTENSION,
};
use crate::image::pipeline::EditPipeline;
use crate::ui::window::MainWindow;
use gtk4::prelude::*;
use gtk4::{gio, glib};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const EXPOSED_ACTIONS: &[&str] = &[
    "new",
    "open",
    "save",
    "save-as",
    "export",
    "quit",
    "undo",
    "redo",
    "cut",
    "copy",
    "paste",
    "image-size",
    "canvas-size",
    "upscale",
    "crop",
    "rotate",
    "flip-h",
    "flip-v",
    "new-layer",
    "duplicate-layer",
    "delete-layer",
    "add-mask",
    "remove-mask",
    "apply-mask",
    "toggle-mask-edit",
    "toggle-mask-enabled",
    "toggle-mask-view",
    "merge-down",
    "blur",
    "sharpen",
    "noise",
    "invert",
    "grayscale",
    "workspace-quick",
    "workspace-full",
    "workspace-canvas-only",
    "tool-move",
    "tool-lasso",
    "tool-brush",
    "tool-eraser",
    "tool-zoom",
    "zoom-in",
    "zoom-out",
    "fit-to-screen",
    "keyboard-shortcuts",
    "about",
];

pub fn install(editor: &MainWindow) {
    replace_new_action(editor);
    replace_open_action(editor);
    replace_save_actions(editor);
    audit_exposed_actions(&editor.window);
    install_title_watch(editor);
}

pub fn open_path(editor: &MainWindow, path: &Path) {
    if let Err(error) = load_path(
        &editor.document,
        &editor.pipeline,
        &editor.zoom,
        editor.canvas.widget(),
        path,
    ) {
        show_error(&editor.window, "Could not open file", &error);
    }
}

fn replace_new_action(editor: &MainWindow) {
    editor.window.remove_action("new");

    let document = editor.document.clone();
    let pipeline = editor.pipeline.clone();
    let zoom = editor.zoom.clone();
    let canvas = editor.canvas.widget().clone();
    let action = gio::SimpleAction::new("new", None);
    action.connect_activate(move |_, _| {
        let width = 1920;
        let height = 1080;
        let replacement_revision = document.borrow().revision.wrapping_add(1);
        let mut next = Document::new(width, height);
        next.add_layer(Layer::new_raster(
            "Background",
            width,
            height,
            vec![0; width as usize * height as usize * 4],
        ));
        next.revision = replacement_revision;
        next.file_path = None;
        next.has_unsaved_changes = true;
        *document.borrow_mut() = next;
        *pipeline.borrow_mut() = EditPipeline::default();
        *zoom.borrow_mut() = 1.0;
        canvas.queue_draw();
    });
    editor.window.add_action(&action);
}

fn replace_open_action(editor: &MainWindow) {
    editor.window.remove_action("open");

    let parent = editor.window.clone();
    let document = editor.document.clone();
    let pipeline = editor.pipeline.clone();
    let zoom = editor.zoom.clone();
    let canvas = editor.canvas.widget().clone();
    let action = gio::SimpleAction::new("open", None);
    action.connect_activate(move |_, _| {
        show_open_dialog(&parent, &document, &pipeline, &zoom, &canvas);
    });
    editor.window.add_action(&action);
}

fn replace_save_actions(editor: &MainWindow) {
    editor.window.remove_action("save");
    editor.window.remove_action("save-as");

    let parent = editor.window.clone();
    let document = editor.document.clone();
    let save = gio::SimpleAction::new("save", None);
    save.connect_activate(move |_, _| {
        let current_path = document
            .borrow()
            .file_path
            .as_deref()
            .map(PathBuf::from)
            .filter(|path| is_project_path(path));

        if let Some(path) = current_path {
            if let Err(error) = save_to_path(&document, &path) {
                show_error(&parent, "Could not save project", &error);
            }
        } else {
            show_save_dialog(&parent, &document);
        }
    });
    editor.window.add_action(&save);

    let parent = editor.window.clone();
    let document = editor.document.clone();
    let save_as = gio::SimpleAction::new("save-as", None);
    save_as.connect_activate(move |_, _| show_save_dialog(&parent, &document));
    editor.window.add_action(&save_as);
}

fn show_open_dialog(
    parent: &adw::ApplicationWindow,
    document: &Rc<RefCell<Document>>,
    pipeline: &Rc<RefCell<EditPipeline>>,
    zoom: &Rc<RefCell<f64>>,
    canvas: &gtk4::DrawingArea,
) {
    let filter = gtk4::FileFilter::new();
    filter.set_name(Some("Slate projects and images"));
    filter.add_pattern(&format!("*.{PROJECT_EXTENSION}"));
    for mime_type in [
        "image/jpeg",
        "image/png",
        "image/webp",
        "image/tiff",
        "image/gif",
        "image/bmp",
    ] {
        filter.add_mime_type(mime_type);
    }

    let dialog = gtk4::FileDialog::new();
    dialog.set_title("Open Project or Image");
    dialog.set_default_filter(Some(&filter));

    let parent_clone = parent.clone();
    let document = document.clone();
    let pipeline = pipeline.clone();
    let zoom = zoom.clone();
    let canvas = canvas.clone();
    dialog.open(Some(parent), gio::Cancellable::NONE, move |result| {
        let Ok(file) = result else {
            return;
        };
        let Some(path) = file.path() else {
            show_error(
                &parent_clone,
                "Could not open file",
                "Slate can only open local files.",
            );
            return;
        };
        if let Err(error) = load_path(&document, &pipeline, &zoom, &canvas, &path) {
            show_error(&parent_clone, "Could not open file", &error);
        }
    });
}

fn load_path(
    document: &Rc<RefCell<Document>>,
    pipeline: &Rc<RefCell<EditPipeline>>,
    zoom: &Rc<RefCell<f64>>,
    canvas: &gtk4::DrawingArea,
    path: &Path,
) -> Result<(), String> {
    let replacement_revision = document.borrow().revision.wrapping_add(1);
    let mut next = if is_project_path(path) {
        load_project(path).map_err(|error| error.to_string())?
    } else {
        load_image_document(path)?
    };

    next.revision = replacement_revision;
    next.file_path = Some(path.to_string_lossy().into_owned());
    next.mark_saved();
    *document.borrow_mut() = next;
    *pipeline.borrow_mut() = EditPipeline::default();
    queue_fit_to_screen(document, zoom, canvas);
    log::info!("Opened {}", path.display());
    Ok(())
}

fn load_image_document(path: &Path) -> Result<Document, String> {
    let image = ::image::open(path)
        .map_err(|error| format!("Failed to decode {}: {error}", path.display()))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return Err("The image has invalid zero-sized dimensions.".to_string());
    }

    let name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Image");
    let mut document = Document::new(width, height);
    document.add_layer(Layer::new_raster(name, width, height, image.into_raw()));
    document.mark_saved();
    Ok(document)
}

fn show_save_dialog(parent: &adw::ApplicationWindow, document: &Rc<RefCell<Document>>) {
    let filter = gtk4::FileFilter::new();
    filter.set_name(Some("Slate Project"));
    filter.add_pattern(&format!("*.{PROJECT_EXTENSION}"));

    let dialog = gtk4::FileDialog::new();
    dialog.set_title("Save Slate Project");
    dialog.set_default_filter(Some(&filter));
    let default_name = default_project_name(&document.borrow());
    dialog.set_initial_file(Some(&gio::File::for_path(default_name)));

    let parent_clone = parent.clone();
    let document = document.clone();
    dialog.save(Some(parent), gio::Cancellable::NONE, move |result| {
        let Ok(file) = result else {
            return;
        };
        let Some(mut path) = file.path() else {
            show_error(
                &parent_clone,
                "Could not save project",
                "Slate can only save to a local file.",
            );
            return;
        };
        if !is_project_path(&path) {
            path.set_extension(PROJECT_EXTENSION);
        }
        if let Err(error) = save_to_path(&document, &path) {
            show_error(&parent_clone, "Could not save project", &error);
        }
    });
}

fn save_to_path(document: &Rc<RefCell<Document>>, path: &Path) -> Result<(), String> {
    let mut snapshot = document.borrow().clone();
    snapshot.file_path = Some(path.to_string_lossy().into_owned());
    snapshot.mark_saved();
    save_project(path, &snapshot).map_err(|error| error.to_string())?;

    let mut document = document.borrow_mut();
    document.file_path = snapshot.file_path;
    document.mark_saved();
    log::info!("Saved Slate project to {}", path.display());
    Ok(())
}

fn default_project_name(document: &Document) -> String {
    let stem = document
        .file_path
        .as_deref()
        .and_then(|path| Path::new(path).file_stem())
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("untitled");
    format!("{stem}.{PROJECT_EXTENSION}")
}

fn queue_fit_to_screen(
    document: &Rc<RefCell<Document>>,
    zoom: &Rc<RefCell<f64>>,
    canvas: &gtk4::DrawingArea,
) {
    let document = document.clone();
    let zoom = zoom.clone();
    let canvas = canvas.clone();
    glib::idle_add_local_once(move || {
        let document = document.borrow();
        let viewport_width = canvas.width().max(1) as f64;
        let viewport_height = canvas.height().max(1) as f64;
        let scale_x = viewport_width / document.canvas_width.max(1) as f64;
        let scale_y = viewport_height / document.canvas_height.max(1) as f64;
        *zoom.borrow_mut() = scale_x.min(scale_y).min(1.0).clamp(0.01, 64.0);
        drop(document);
        canvas.queue_draw();
    });
}

fn install_title_watch(editor: &MainWindow) {
    let window = editor.window.downgrade();
    let document = editor.document.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(250), move || {
        let Some(window) = window.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let document = document.borrow();
        let name = document
            .file_path
            .as_deref()
            .and_then(|path| Path::new(path).file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");
        let marker = if document.has_unsaved_changes {
            "● "
        } else {
            ""
        };
        window.set_title(Some(&format!("{marker}{name} — Slate")));
        glib::ControlFlow::Continue
    });
}

fn audit_exposed_actions(window: &adw::ApplicationWindow) {
    for action_name in EXPOSED_ACTIONS {
        if window.lookup_action(action_name).is_none() {
            log::error!("Exposed action '{action_name}' is not registered");
        }
    }
}

#[allow(deprecated)]
fn show_error(parent: &adw::ApplicationWindow, title: &str, detail: &str) {
    log::error!("{title}: {detail}");
    let dialog = gtk4::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(gtk4::MessageType::Error)
        .buttons(gtk4::ButtonsType::Close)
        .text(title)
        .secondary_text(detail)
        .build();
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn exposed_action_inventory_is_unique() {
        let mut actions = HashSet::new();
        for action in EXPOSED_ACTIONS {
            assert!(actions.insert(action), "duplicate action: {action}");
        }
    }

    #[test]
    fn default_project_name_replaces_image_extension() {
        let mut document = Document::new(1, 1);
        document.file_path = Some("/tmp/poster.png".to_string());
        assert_eq!(default_project_name(&document), "poster.slate");
    }
}
