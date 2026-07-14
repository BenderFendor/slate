#![allow(dead_code)]

use adw::prelude::*;
use gtk4::gio;

use crate::document::{
    AddLayerCommand, AddLayerMaskCommand, ApplyLayerMaskCommand, Command, Document, Layer,
    LayerKind, Mask, RemoveLayerCommand, RemoveLayerMaskCommand, ReplaceDocumentCommand,
};
use crate::image::pipeline::{EditPipeline, ExportParams};
use crate::tile::snapshot::{build_render_frame, flatten_frame_bgra};
use crate::tools::tool::ToolKind;
use crate::ui::canvas::CanvasWidget;
use crate::ui::command_palette::CommandPalette;
use crate::ui::options_bar::OptionsBar;
use crate::ui::panels::RightPanels;
use crate::ui::quick_edit::QuickEditPanel;
use crate::ui::toolbar::Toolbar;

use std::cell::RefCell;
use std::rc::Rc;

struct ShortcutSpec {
    action: Option<&'static str>,
    command: &'static str,
    display: &'static str,
    accels: &'static [&'static str],
}

const SHORTCUTS: &[ShortcutSpec] = &[
    ShortcutSpec {
        action: Some("win.new"),
        command: "New",
        display: "Ctrl+N",
        accels: &["<Control>N"],
    },
    ShortcutSpec {
        action: Some("win.open"),
        command: "Open",
        display: "Ctrl+O",
        accels: &["<Control>O"],
    },
    ShortcutSpec {
        action: Some("win.save"),
        command: "Save",
        display: "Ctrl+S",
        accels: &["<Control>S"],
    },
    ShortcutSpec {
        action: Some("win.save-as"),
        command: "Save As",
        display: "Ctrl+Shift+S",
        accels: &["<Control><Shift>S"],
    },
    ShortcutSpec {
        action: Some("win.export"),
        command: "Export",
        display: "Ctrl+Shift+E",
        accels: &["<Control><Shift>E"],
    },
    ShortcutSpec {
        action: Some("win.undo"),
        command: "Undo",
        display: "Ctrl+Z",
        accels: &["<Control>Z"],
    },
    ShortcutSpec {
        action: Some("win.redo"),
        command: "Redo",
        display: "Ctrl+Shift+Z / Ctrl+Y",
        accels: &["<Control><Shift>Z", "<Control>Y"],
    },
    ShortcutSpec {
        action: Some("win.cut"),
        command: "Cut Layer",
        display: "Ctrl+X",
        accels: &["<Control>X"],
    },
    ShortcutSpec {
        action: Some("win.copy"),
        command: "Copy Layer",
        display: "Ctrl+C",
        accels: &["<Control>C"],
    },
    ShortcutSpec {
        action: Some("win.paste"),
        command: "Paste Layer",
        display: "Ctrl+V",
        accels: &["<Control>V"],
    },
    ShortcutSpec {
        action: Some("win.new-layer"),
        command: "New Layer",
        display: "Ctrl+Shift+N",
        accels: &["<Control><Shift>N"],
    },
    ShortcutSpec {
        action: Some("win.duplicate-layer"),
        command: "Duplicate Layer",
        display: "Ctrl+J",
        accels: &["<Control>J"],
    },
    ShortcutSpec {
        action: Some("win.delete-layer"),
        command: "Delete Layer",
        display: "Delete",
        accels: &["Delete"],
    },
    ShortcutSpec {
        action: Some("win.toggle-mask-edit"),
        command: "Toggle Mask Edit",
        display: "Ctrl+M",
        accels: &["<Control>M"],
    },
    ShortcutSpec {
        action: Some("win.tool-lasso"),
        command: "Lasso Tool",
        display: "L",
        accels: &["L"],
    },
    ShortcutSpec {
        action: Some("win.tool-move"),
        command: "Move Tool",
        display: "V",
        accels: &["V"],
    },
    ShortcutSpec {
        action: Some("win.crop"),
        command: "Crop Tool",
        display: "C",
        accels: &["C"],
    },
    ShortcutSpec {
        action: Some("win.tool-brush"),
        command: "Brush Tool",
        display: "B",
        accels: &["B"],
    },
    ShortcutSpec {
        action: Some("win.tool-eraser"),
        command: "Eraser Tool",
        display: "E",
        accels: &["E"],
    },
    ShortcutSpec {
        action: Some("win.tool-zoom"),
        command: "Zoom Tool",
        display: "Z",
        accels: &["Z"],
    },
    ShortcutSpec {
        action: Some("win.zoom-in"),
        command: "Zoom In",
        display: "Ctrl++",
        accels: &["<Control>plus", "<Control>equal"],
    },
    ShortcutSpec {
        action: Some("win.zoom-out"),
        command: "Zoom Out",
        display: "Ctrl+-",
        accels: &["<Control>minus"],
    },
    ShortcutSpec {
        action: Some("win.fit-to-screen"),
        command: "Fit to Screen",
        display: "Ctrl+0",
        accels: &["<Control>0"],
    },
    ShortcutSpec {
        action: Some("win.keyboard-shortcuts"),
        command: "Keyboard Shortcuts",
        display: "Ctrl+Shift+K",
        accels: &["<Control><Shift>K"],
    },
    ShortcutSpec {
        action: Some("win.quit"),
        command: "Quit",
        display: "Ctrl+Q",
        accels: &["<Control>Q"],
    },
    ShortcutSpec {
        action: None,
        command: "Temporary Pan",
        display: "Space",
        accels: &[],
    },
    ShortcutSpec {
        action: None,
        command: "Brush Size",
        display: "[ / ]",
        accels: &[],
    },
    ShortcutSpec {
        action: None,
        command: "Brush Hardness",
        display: "Shift+[ / Shift+]",
        accels: &[],
    },
];

const VISIBLE_WINDOW_ACTIONS: &[&str] = &[
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
    "crop",
    "rotate",
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
    "workspace-quick",
    "workspace-full",
    "workspace-canvas-only",
    "tool-move",
    "tool-brush",
    "tool-eraser",
    "tool-zoom",
    "zoom-in",
    "zoom-out",
    "fit-to-screen",
    "keyboard-shortcuts",
    "about",
];

#[allow(dead_code)]
pub struct MainWindow {
    pub window: adw::ApplicationWindow,
    pub document: Rc<RefCell<Document>>,
    pub canvas: CanvasWidget,
    pub pipeline: Rc<RefCell<EditPipeline>>,
    pub export_params: Rc<RefCell<ExportParams>>,
    pub active_tool: Rc<RefCell<ToolKind>>,
    zoom: Rc<RefCell<f64>>,
    panels: RightPanels,
    workspace_stack: adw::ViewStack,
    status_zoom: gtk4::Label,
    status_dims: gtk4::Label,
    status_output: gtk4::Label,
}

impl MainWindow {
    pub fn new(app: &adw::Application) -> Self {
        let document = Rc::new(RefCell::new(Document::new(1920, 1080)));
        let pipeline = Rc::new(RefCell::new(EditPipeline::default()));
        let export_params = Rc::new(RefCell::new(ExportParams::default()));
        let active_tool = Rc::new(RefCell::new(ToolKind::Move));
        let zoom = Rc::new(RefCell::new(1.0));

        let window = adw::ApplicationWindow::new(app);
        window.set_default_size(1400, 900);
        window.set_title(Some("Slate"));

        let toolbar = Rc::new(Toolbar::new(active_tool.clone()));
        let toolbar_widget = toolbar.widget().clone();

        let options_bar = OptionsBar::new(active_tool.clone(), pipeline.clone(), document.clone());
        let opts_widget = options_bar.widget().clone();
        let brush_size = options_bar.brush_size.clone();
        let brush_hardness = options_bar.brush_hardness.clone();
        let brush_opacity = options_bar.brush_opacity.clone();
        let brush_flow = options_bar.brush_flow.clone();
        let brush_color = options_bar.brush_color.clone();

        toolbar.set_on_tool_change(move |kind| {
            options_bar.set_tool(kind);
        });

        let canvas = CanvasWidget::new(
            document.clone(),
            active_tool.clone(),
            zoom.clone(),
            pipeline.clone(),
            brush_size,
            brush_hardness,
            brush_opacity,
            brush_flow,
            brush_color.clone(),
        );

        let panels = RightPanels::new(document.clone(), brush_color.clone());
        let panels_widget = panels.widget().clone();

        let quick_panel =
            QuickEditPanel::new(document.clone(), pipeline.clone(), export_params.clone());

        let workspace_stack = adw::ViewStack::new();
        let full_page = workspace_stack.add_titled(
            &panels_widget,
            Some("full"),
            "Full Edit",
        );
        full_page.set_icon_name(Some("format-justify-fill-symbolic"));

        let quick_page = workspace_stack.add_titled(
            quick_panel.widget(),
            Some("quick"),
            "Quick Edit",
        );
        quick_page.set_icon_name(Some("image-filter-vintage-symbolic"));
        workspace_stack.set_visible_child_name("full");

        Self::install_window_actions(
            &window,
            app,
            &document,
            &pipeline,
            &export_params,
            canvas.widget(),
            &toolbar,
            &zoom,
        );

        let (editor_box, right_panel) = Self::build_editor_layout(
            &toolbar_widget,
            &opts_widget,
            canvas.root(),
            &workspace_stack,
        );

        let header =
            Self::create_header(&document, &pipeline, &export_params, &zoom, canvas.widget(), &workspace_stack);

        let (status, status_zoom, status_dims, status_output) =
            Self::create_status_bar(&zoom, &document, &pipeline);

        let canvas_only = Rc::new(RefCell::new(false));
        let menubar = Self::create_menubar(&window);

        // Workspace actions keep direct widget references because they change chrome visibility.
        {
            let ws = workspace_stack.clone();
            let co = canvas_only.clone();
            let toolbar = toolbar_widget.clone();
            let options = opts_widget.clone();
            let rp = right_panel.clone();
            let hdr = header.clone();
            let st = status.clone();
            let mb = menubar.clone();
            let action = gio::SimpleAction::new("workspace-quick", None);
            action.connect_activate(move |_, _| {
                *co.borrow_mut() = false;
                Self::show_chrome(&toolbar, &options, &rp, &hdr, &st, &mb);
                ws.set_visible_child_name("quick");
            });
            window.add_action(&action);
        }
        {
            let ws = workspace_stack.clone();
            let co = canvas_only.clone();
            let toolbar = toolbar_widget.clone();
            let options = opts_widget.clone();
            let rp = right_panel.clone();
            let hdr = header.clone();
            let st = status.clone();
            let mb = menubar.clone();
            let action = gio::SimpleAction::new("workspace-full", None);
            action.connect_activate(move |_, _| {
                *co.borrow_mut() = false;
                Self::show_chrome(&toolbar, &options, &rp, &hdr, &st, &mb);
                ws.set_visible_child_name("full");
            });
            window.add_action(&action);
        }
        {
            let co = canvas_only.clone();
            let toolbar = toolbar_widget.clone();
            let options = opts_widget.clone();
            let rp = right_panel.clone();
            let hdr = header.clone();
            let st = status.clone();
            let mb = menubar.clone();
            let action = gio::SimpleAction::new("workspace-canvas-only", None);
            action.connect_activate(move |_, _| {
                *co.borrow_mut() = true;
                Self::hide_chrome(&toolbar, &options, &rp, &hdr, &st, &mb);
            });
            window.add_action(&action);
        }

        Self::audit_visible_actions(&window);

        let main_vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        main_vbox.append(&menubar);
        main_vbox.append(&header);
        main_vbox.append(&editor_box);
        main_vbox.append(&status);

        window.set_content(Some(&main_vbox));

        let cmd_palette = CommandPalette::new(window.upcast_ref());
        let cmd_widget = cmd_palette.widget().clone();

        let window_weak = window.downgrade();
        let cmd_weak = cmd_widget.downgrade();
        let tb_obj = toolbar.clone();
        let key_ctrl = gtk4::EventControllerKey::new();
        {
            let ws = workspace_stack.clone();
            let co = canvas_only.clone();
            let toolbar = toolbar_widget.clone();
            let options = opts_widget.clone();
            let rp = right_panel.clone();
            let hdr = header.clone();
            let st = status.clone();
            let mb = menubar.clone();
            let tb = tb_obj.clone();
            key_ctrl.connect_key_pressed(move |_ctrl, key, _code, mods| {
                let ctrl = mods.contains(gtk4::gdk::ModifierType::CONTROL_MASK);
                if ctrl && key == gtk4::gdk::Key::p {
                    if let (Some(_win), Some(cmd_win)) = (window_weak.upgrade(), cmd_weak.upgrade())
                    {
                        cmd_win.present();
                        cmd_win.grab_focus();
                    }
                    return gtk4::glib::Propagation::Stop;
                }
                if ctrl && key == gtk4::gdk::Key::_1 {
                    *co.borrow_mut() = false;
                    Self::show_chrome(&toolbar, &options, &rp, &hdr, &st, &mb);
                    ws.set_visible_child_name("quick");
                    return gtk4::glib::Propagation::Stop;
                }
                if ctrl && key == gtk4::gdk::Key::_2 {
                    *co.borrow_mut() = false;
                    Self::show_chrome(&toolbar, &options, &rp, &hdr, &st, &mb);
                    ws.set_visible_child_name("full");
                    return gtk4::glib::Propagation::Stop;
                }
                if ctrl && key == gtk4::gdk::Key::_3 {
                    Self::hide_chrome(&toolbar, &options, &rp, &hdr, &st, &mb);
                    *co.borrow_mut() = true;
                    return gtk4::glib::Propagation::Stop;
                }

                if !mods.is_empty() {
                    return gtk4::glib::Propagation::Proceed;
                }

                let kind = match key {
                    gtk4::gdk::Key::v => Some(ToolKind::Move),
                    gtk4::gdk::Key::c => Some(ToolKind::Crop),
                    gtk4::gdk::Key::b => Some(ToolKind::Brush),
                    gtk4::gdk::Key::e => Some(ToolKind::Eraser),
                    gtk4::gdk::Key::h => Some(ToolKind::Move),
                    gtk4::gdk::Key::z => Some(ToolKind::Zoom),
                    gtk4::gdk::Key::x => {
                        log::info!("Swap foreground/background colors");
                        return gtk4::glib::Propagation::Stop;
                    }
                    gtk4::gdk::Key::d => {
                        log::info!("Default colors (D)");
                        return gtk4::glib::Propagation::Stop;
                    }
                    _ => None,
                };

                if let Some(kind) = kind {
                    tb.activate_tool(kind);
                    return gtk4::glib::Propagation::Stop;
                }

                gtk4::glib::Propagation::Proceed
            });
        }
        window.add_controller(key_ctrl);

        // Tab to toggle panels (Canvas-only mode)
        {
            let co = canvas_only.clone();
            let toolbar = toolbar_widget.clone();
            let options = opts_widget.clone();
            let rp = right_panel.clone();
            let hdr = header.clone();
            let st = status.clone();
            let mb = menubar.clone();

            let key_tab = gtk4::EventControllerKey::new();
            key_tab.connect_key_pressed(move |_ctrl, key, _code, _mods| {
                if key == gtk4::gdk::Key::Tab {
                    let mut canvas_only = co.borrow_mut();
                    *canvas_only = !*canvas_only;
                    if *canvas_only {
                        Self::hide_chrome(&toolbar, &options, &rp, &hdr, &st, &mb);
                    } else {
                        Self::show_chrome(&toolbar, &options, &rp, &hdr, &st, &mb);
                    }
                    return gtk4::glib::Propagation::Stop;
                }
                gtk4::glib::Propagation::Proceed
            });
            window.add_controller(key_tab);
        }

        // Drop-to-open support
        let drop_target =
            gtk4::DropTarget::new(gio::File::static_type(), gtk4::gdk::DragAction::COPY);
        {
            let doc = document.clone();
            let pip = pipeline.clone();
            let zoom = zoom.clone();
            let cw = canvas.widget().clone();
            drop_target.connect_drop(move |_target, value, _x, _y| {
                if let Ok(file) = value.get::<gio::File>() {
                    if let Some(path) = file.path() {
                        Self::open_image(&doc, &pip, &path);
                        Self::queue_fit_to_screen(&doc, &zoom, &cw);
                        return true;
                    }
                }
                false
            });
        }
        window.add_controller(drop_target);

        {
            let doc = document.clone();
            let pip = pipeline.clone();
            let exp = export_params.clone();
            quick_panel.set_on_export(move || {
                Self::run_export(&doc, &pip, &exp);
            });
        }

        let zoom_clone = zoom.clone();
        let doc_for_status = document.clone();
        let pip_for_status = pipeline.clone();
        let tool_clone = active_tool.clone();
        let main_window = Self {
            window,
            document,
            canvas,
            pipeline,
            export_params,
            active_tool,
            zoom,
            panels,
            workspace_stack,
            status_zoom: status_zoom.clone(),
            status_dims: status_dims.clone(),
            status_output: status_output.clone(),
        };

        {
            let zoom_label = main_window.status_zoom.clone();
            let dims_label = main_window.status_dims.clone();
            let output_label = main_window.status_output.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                let z = *zoom_clone.borrow();
                zoom_label.set_text(&format!("{:.0}%", z * 100.0));

                let (cw, ch, active_id) = {
                    let d = doc_for_status.borrow();
                    (d.canvas_width, d.canvas_height, d.active_layer_id)
                };
                dims_label.set_text(&format!("{} x {}", cw, ch));

                let p = pip_for_status.borrow();
                let t = *tool_clone.borrow();
                let output_text = if let Some(crop) = &p.crop {
                    let resize_str = p
                        .resize
                        .as_ref()
                        .map(|r| format!("{}x{}", r.width, r.height))
                        .unwrap_or_else(|| "Original".to_string());
                    format!(
                        "Crop: {:.0}x{:.0} -> Output: {}",
                        crop.width, crop.height, resize_str
                    )
                } else if t == ToolKind::Brush || t == ToolKind::Eraser {
                    "Editing: Layer pixels".to_string()
                } else {
                    let layer_text = active_id
                        .and_then(|id| doc_for_status.borrow().layer(id).map(|l| l.name.clone()))
                        .unwrap_or_else(|| "--".to_string());
                    layer_text
                };
                drop(p);
                output_label.set_text(&output_text);

                glib::ControlFlow::Continue
            });
        }

        main_window
    }

    fn create_menubar(_window: &adw::ApplicationWindow) -> gtk4::PopoverMenuBar {
        let menu = gio::Menu::new();

        let file_menu = gio::Menu::new();
        file_menu.append(Some("New..."), Some("win.new"));
        file_menu.append(Some("Open..."), Some("win.open"));
        file_menu.append(Some("Save"), Some("win.save"));
        file_menu.append(Some("Save As..."), Some("win.save-as"));
        file_menu.append(Some("Export..."), Some("win.export"));
        file_menu.append(Some("Exit"), Some("win.quit"));
        menu.append_submenu(Some("File"), &file_menu);

        let edit_menu = gio::Menu::new();
        edit_menu.append(Some("Undo"), Some("win.undo"));
        edit_menu.append(Some("Redo"), Some("win.redo"));
        edit_menu.append(Some("Cut"), Some("win.cut"));
        edit_menu.append(Some("Copy"), Some("win.copy"));
        edit_menu.append(Some("Paste"), Some("win.paste"));
        menu.append_submenu(Some("Edit"), &edit_menu);

        let image_menu = gio::Menu::new();
        image_menu.append(Some("Image Size..."), Some("win.image-size"));
        image_menu.append(Some("Canvas Size..."), Some("win.canvas-size"));
        image_menu.append(Some("Upscale Image..."), Some("win.upscale"));
        image_menu.append(Some("Crop"), Some("win.crop"));
        image_menu.append(Some("Rotate Clockwise"), Some("win.rotate"));
        image_menu.append(Some("Flip Horizontal"), Some("win.flip-h"));
        image_menu.append(Some("Flip Vertical"), Some("win.flip-v"));
        menu.append_submenu(Some("Image"), &image_menu);

        let layer_menu = gio::Menu::new();
        layer_menu.append(Some("New Layer"), Some("win.new-layer"));
        layer_menu.append(Some("Duplicate Layer"), Some("win.duplicate-layer"));
        layer_menu.append(Some("Delete Layer"), Some("win.delete-layer"));
        layer_menu.append(Some("Add Layer Mask"), Some("win.add-mask"));
        layer_menu.append(Some("Remove Layer Mask"), Some("win.remove-mask"));
        layer_menu.append(Some("Apply Layer Mask"), Some("win.apply-mask"));
        layer_menu.append(Some("Toggle Edit Mask"), Some("win.toggle-mask-edit"));
        layer_menu.append(
            Some("Enable / Disable Mask"),
            Some("win.toggle-mask-enabled"),
        );
        layer_menu.append(Some("Merge Down"), Some("win.merge-down"));
        menu.append_submenu(Some("Layer"), &layer_menu);

        let filter_menu = gio::Menu::new();
        filter_menu.append(Some("Blur"), Some("win.blur"));
        filter_menu.append(Some("Sharpen"), Some("win.sharpen"));
        filter_menu.append(Some("Noise"), Some("win.noise"));
        filter_menu.append(Some("Invert"), Some("win.invert"));
        filter_menu.append(Some("Grayscale"), Some("win.grayscale"));
        menu.append_submenu(Some("Filter"), &filter_menu);

        let view_menu = gio::Menu::new();
        let workspace_section = gio::Menu::new();
        workspace_section.append(Some("Quick Edit Workspace"), Some("win.workspace-quick"));
        workspace_section.append(Some("Full Edit Workspace"), Some("win.workspace-full"));
        view_menu.append_section(None, &workspace_section);
        let zoom_section = gio::Menu::new();
        zoom_section.append(Some("Zoom In"), Some("win.zoom-in"));
        zoom_section.append(Some("Zoom Out"), Some("win.zoom-out"));
        zoom_section.append(Some("Fit to Screen"), Some("win.fit-to-screen"));
        view_menu.append_section(None, &zoom_section);
        menu.append_submenu(Some("View"), &view_menu);

        let help_menu = gio::Menu::new();
        help_menu.append(Some("Keyboard Shortcuts"), Some("win.keyboard-shortcuts"));
        help_menu.append(Some("About"), Some("win.about"));
        menu.append_submenu(Some("Help"), &help_menu);

        let menubar = gtk4::PopoverMenuBar::from_model(Some(&menu));
        menubar.add_css_class("menubar");
        menubar
    }

    fn install_window_actions(
        window: &adw::ApplicationWindow,
        app: &adw::Application,
        document: &Rc<RefCell<Document>>,
        pipeline: &Rc<RefCell<EditPipeline>>,
        export_params: &Rc<RefCell<ExportParams>>,
        canvas: &gtk4::DrawingArea,
        toolbar: &Rc<Toolbar>,
        zoom: &Rc<RefCell<f64>>,
    ) {
        for shortcut in SHORTCUTS {
            if let Some(action) = shortcut.action {
                app.set_accels_for_action(action, shortcut.accels);
            }
        }

        let layer_clipboard: Rc<RefCell<Option<Layer>>> = Rc::new(RefCell::new(None));

        {
            let doc = document.clone();
            let pip = pipeline.clone();
            let exp = export_params.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("new", None);
            action.connect_activate(move |_, _| {
                let mut d = doc.borrow_mut();
                *d = Document::new(1920, 1080);
                *pip.borrow_mut() = EditPipeline::default();
                *exp.borrow_mut() = ExportParams::default();
                cw.queue_draw();
            });
            window.add_action(&action);
        }
        {
            let doc = document.clone();
            let pip = pipeline.clone();
            let exp = export_params.clone();
            let action = gio::SimpleAction::new("save-as", None);
            action.connect_activate(move |_, _| {
                Self::run_export(&doc, &pip, &exp);
            });
            window.add_action(&action);
        }
        {
            let doc = document.clone();
            let pip = pipeline.clone();
            let zoom = zoom.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("open", None);
            action.connect_activate(move |_, _| {
                Self::open_image_dialog(&doc, &pip, &zoom, &cw);
            });
            window.add_action(&action);
        }
        {
            let doc = document.clone();
            let pip = pipeline.clone();
            let exp = export_params.clone();
            let action = gio::SimpleAction::new("save", None);
            action.connect_activate(move |_, _| {
                Self::run_export(&doc, &pip, &exp);
            });
            window.add_action(&action);
        }
        {
            let doc = document.clone();
            let pip = pipeline.clone();
            let exp = export_params.clone();
            let action = gio::SimpleAction::new("export", None);
            action.connect_activate(move |_, _| {
                Self::run_export(&doc, &pip, &exp);
            });
            window.add_action(&action);
        }
        {
            let doc = document.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("new-layer", None);
            action.connect_activate(move |_, _| {
                let (w, h) = {
                    let d = doc.borrow();
                    (d.canvas_width, d.canvas_height)
                };
                let layer = Layer::new_raster("New Layer", w, h, vec![0u8; (w * h * 4) as usize]);
                let index = doc.borrow().layers.len();
                Self::execute_document_command(&doc, Box::new(AddLayerCommand::new(layer, index)));
                cw.queue_draw();
            });
            window.add_action(&action);
        }
        {
            let doc = document.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("delete-layer", None);
            action.connect_activate(move |_, _| {
                let index = {
                    let d = doc.borrow();
                    d.active_layer_id
                        .and_then(|id| d.layers.iter().position(|layer| layer.id == id))
                };
                if let Some(index) = index {
                    Self::execute_document_command(&doc, Box::new(RemoveLayerCommand::new(index)));
                    cw.queue_draw();
                }
            });
            window.add_action(&action);
        }
        {
            let doc = document.clone();
            let clipboard = layer_clipboard.clone();
            let action = gio::SimpleAction::new("copy", None);
            action.connect_activate(move |_, _| {
                *clipboard.borrow_mut() = {
                    let d = doc.borrow();
                    d.active_layer_id.and_then(|id| d.layer(id).cloned())
                };
            });
            window.add_action(&action);
        }
        {
            let doc = document.clone();
            let clipboard = layer_clipboard.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("cut", None);
            action.connect_activate(move |_, _| {
                let (copied, index) = {
                    let d = doc.borrow();
                    let copied = d.active_layer_id.and_then(|id| d.layer(id).cloned());
                    let index = d
                        .active_layer_id
                        .and_then(|id| d.layers.iter().position(|layer| layer.id == id));
                    (copied, index)
                };
                if let Some(layer) = copied {
                    *clipboard.borrow_mut() = Some(layer);
                }
                if let Some(index) = index {
                    Self::execute_document_command(&doc, Box::new(RemoveLayerCommand::new(index)));
                    cw.queue_draw();
                }
            });
            window.add_action(&action);
        }
        {
            let doc = document.clone();
            let clipboard = layer_clipboard.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("paste", None);
            action.connect_activate(move |_, _| {
                let layer = clipboard.borrow().clone();
                if let Some(mut layer) = layer {
                    layer.id = crate::document::LayerId::new();
                    layer.name = format!("{} paste", layer.name);
                    let index = doc.borrow().layers.len();
                    Self::execute_document_command(
                        &doc,
                        Box::new(AddLayerCommand::new(layer, index)),
                    );
                    cw.queue_draw();
                }
            });
            window.add_action(&action);
        }
        {
            let doc = document.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("duplicate-layer", None);
            action.connect_activate(move |_, _| {
                let duplicate = {
                    let d = doc.borrow();
                    d.active_layer_id.and_then(|id| d.layer(id).cloned())
                };
                if let Some(mut layer) = duplicate {
                    layer.id = crate::document::LayerId::new();
                    layer.name = format!("{} copy", layer.name);
                    let index = doc.borrow().layers.len();
                    Self::execute_document_command(
                        &doc,
                        Box::new(AddLayerCommand::new(layer, index)),
                    );
                    cw.queue_draw();
                }
            });
            window.add_action(&action);
        }
        {
            let doc = document.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("add-mask", None);
            action.connect_activate(move |_, _| {
                let command = {
                    let d = doc.borrow();
                    d.active_layer_id.and_then(|id| {
                        d.layer(id).and_then(|layer| {
                            if layer.mask.is_none() {
                                Some(Box::new(AddLayerMaskCommand::new(id, {
                                    let (width, height) = match &layer.kind {
                                        LayerKind::Raster(raster) => (raster.width, raster.height),
                                        _ => (d.canvas_width, d.canvas_height),
                                    };
                                    Mask::new("Layer Mask", width, height)
                                })) as Box<dyn Command>)
                            } else {
                                None
                            }
                        })
                    })
                };
                if let Some(command) = command {
                    Self::execute_document_command(&doc, command);
                    cw.queue_draw();
                }
            });
            window.add_action(&action);
        }
        {
            let doc = document.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("remove-mask", None);
            action.connect_activate(move |_, _| {
                let layer_id = {
                    let d = doc.borrow();
                    d.active_layer_id
                        .filter(|id| d.layer(*id).and_then(|layer| layer.mask.as_ref()).is_some())
                };
                if let Some(layer_id) = layer_id {
                    Self::execute_document_command(
                        &doc,
                        Box::new(RemoveLayerMaskCommand::new(layer_id)),
                    );
                    cw.queue_draw();
                }
            });
            window.add_action(&action);
        }
        {
            let doc = document.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("apply-mask", None);
            action.connect_activate(move |_, _| {
                let layer_id = {
                    let d = doc.borrow();
                    d.active_layer_id
                        .filter(|id| d.layer(*id).and_then(|layer| layer.mask.as_ref()).is_some())
                };
                if let Some(layer_id) = layer_id {
                    Self::execute_document_command(
                        &doc,
                        Box::new(ApplyLayerMaskCommand::new(layer_id)),
                    );
                    cw.queue_draw();
                }
            });
            window.add_action(&action);
        }
        for (name, update) in [
            ("toggle-mask-edit", Self::toggle_mask_edit as fn(&mut Mask)),
            (
                "toggle-mask-enabled",
                Self::toggle_mask_enabled as fn(&mut Mask),
            ),
            ("toggle-mask-view", Self::toggle_mask_view as fn(&mut Mask)),
        ] {
            let doc = document.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new(name, None);
            action.connect_activate(move |_, _| {
                let mut d = doc.borrow_mut();
                let active_id = d.active_layer_id;
                if let Some(layer) = active_id.and_then(|id| d.layer_mut(id)) {
                    if let Some(mask) = layer.mask.as_mut() {
                        update(mask);
                        d.revision += 1;
                        d.has_unsaved_changes = true;
                        cw.queue_draw();
                    }
                }
            });
            window.add_action(&action);
        }
        {
            let doc = document.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("undo", None);
            action.connect_activate(move |_, _| {
                let mut d = doc.borrow_mut();
                let mut undo_stack = std::mem::take(&mut d.undo_stack);
                undo_stack.undo(&mut d);
                d.undo_stack = undo_stack;
                cw.queue_draw();
            });
            window.add_action(&action);
        }
        {
            let doc = document.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("redo", None);
            action.connect_activate(move |_, _| {
                let mut d = doc.borrow_mut();
                let mut undo_stack = std::mem::take(&mut d.undo_stack);
                undo_stack.redo(&mut d);
                d.undo_stack = undo_stack;
                cw.queue_draw();
            });
            window.add_action(&action);
        }
        {
            let tb = toolbar.clone();
            let action = gio::SimpleAction::new("crop", None);
            action.connect_activate(move |_, _| {
                tb.activate_tool(ToolKind::Crop);
            });
            window.add_action(&action);
        }
        for (name, tool) in [
            ("tool-move", ToolKind::Move),
            ("tool-lasso", ToolKind::Lasso),
            ("tool-brush", ToolKind::Brush),
            ("tool-eraser", ToolKind::Eraser),
            ("tool-zoom", ToolKind::Zoom),
        ] {
            let tb = toolbar.clone();
            let action = gio::SimpleAction::new(name, None);
            action.connect_activate(move |_, _| {
                tb.activate_tool(tool);
            });
            window.add_action(&action);
        }
        {
            let zoom = zoom.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("zoom-in", None);
            action.connect_activate(move |_, _| {
                *zoom.borrow_mut() = (*zoom.borrow() * 1.25).clamp(0.01, 64.0);
                cw.queue_draw();
            });
            window.add_action(&action);
        }
        {
            let zoom = zoom.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("zoom-out", None);
            action.connect_activate(move |_, _| {
                *zoom.borrow_mut() = (*zoom.borrow() / 1.25).clamp(0.01, 64.0);
                cw.queue_draw();
            });
            window.add_action(&action);
        }
        {
            let zoom = zoom.clone();
            let cw = canvas.clone();
            let doc = document.clone();
            let action = gio::SimpleAction::new("fit-to-screen", None);
            action.connect_activate(move |_, _| {
                #[allow(deprecated)]
                let allocation = cw.allocation();
                let (doc_w, doc_h) = {
                    let d = doc.borrow();
                    (d.canvas_width as f64, d.canvas_height as f64)
                };
                let view_w = allocation.width() as f64 * 0.9;
                let view_h = allocation.height() as f64 * 0.9;
                if doc_w > 0.0 && doc_h > 0.0 && view_w > 0.0 && view_h > 0.0 {
                    *zoom.borrow_mut() = (view_w / doc_w).min(view_h / doc_h).clamp(0.01, 64.0);
                    cw.queue_draw();
                }
            });
            window.add_action(&action);
        }
        {
            let win = window.clone();
            let action = gio::SimpleAction::new("keyboard-shortcuts", None);
            action.connect_activate(move |_, _| {
                Self::show_keyboard_shortcuts(&win);
            });
            window.add_action(&action);
        }
        {
            let win = window.clone();
            let action = gio::SimpleAction::new("quit", None);
            action.connect_activate(move |_, _| {
                win.close();
            });
            window.add_action(&action);
        }

        {
            let win = window.clone();
            let doc = document.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("image-size", None);
            action.connect_activate(move |_, _| {
                Self::show_image_size_dialog(&win, &doc, &cw);
            });
            window.add_action(&action);
        }
        {
            let win = window.clone();
            let doc = document.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("canvas-size", None);
            action.connect_activate(move |_, _| {
                Self::show_canvas_size_dialog(&win, &doc, &cw);
            });
            window.add_action(&action);
        }
        {
            let win = window.clone();
            let doc = document.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("upscale", None);
            action.connect_activate(move |_, _| {
                Self::show_upscale_dialog(&win, &doc, &cw);
            });
            window.add_action(&action);
        }
        {
            let doc = document.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("rotate", None);
            action.connect_activate(move |_, _| {
                let mut next = doc.borrow().clone();
                Self::rotate_document_clockwise(&mut next);
                Self::execute_document_command(
                    &doc,
                    Box::new(ReplaceDocumentCommand::new("Rotate Image", next)),
                );
                cw.queue_draw();
            });
            window.add_action(&action);
        }
        {
            let doc = document.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new("merge-down", None);
            action.connect_activate(move |_, _| {
                if let Some(next) = Self::merged_down_document(&doc.borrow()) {
                    Self::execute_document_command(
                        &doc,
                        Box::new(ReplaceDocumentCommand::new("Merge Down", next)),
                    );
                    cw.queue_draw();
                }
            });
            window.add_action(&action);
        }

        for (name, func) in [
            ("flip-h", Self::flip_document_horizontal as fn(&mut Document)),
            ("flip-v", Self::flip_document_vertical as fn(&mut Document)),
        ] {
            let doc = document.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new(name, None);
            action.connect_activate(move |_, _| {
                let mut d = doc.borrow_mut();
                func(&mut d);
                cw.queue_draw();
            });
            window.add_action(&action);
        }

        for (name, filter) in [
            ("blur", Self::filter_blur as fn(&mut [u8], u32, u32)),
            ("sharpen", Self::filter_sharpen as fn(&mut [u8], u32, u32)),
            ("noise", Self::filter_noise as fn(&mut [u8], u32, u32)),
            ("invert", Self::filter_invert as fn(&mut [u8], u32, u32)),
            ("grayscale", Self::filter_grayscale as fn(&mut [u8], u32, u32)),
        ] {
            let doc = document.clone();
            let cw = canvas.clone();
            let action = gio::SimpleAction::new(name, None);
            action.connect_activate(move |_, _| {
                let mut next = doc.borrow().clone();
                if Self::apply_active_raster_filter(&mut next, filter) {
                    Self::execute_document_command(
                        &doc,
                        Box::new(ReplaceDocumentCommand::new("Apply Filter", next)),
                    );
                    cw.queue_draw();
                }
            });
            window.add_action(&action);
        }
        {
            let win = window.clone();
            let action = gio::SimpleAction::new("about", None);
            action.connect_activate(move |_, _| {
                let about = gtk4::AboutDialog::builder()
                    .program_name("Slate")
                    .version(env!("CARGO_PKG_VERSION"))
                    .comments("Native GTK4 image editor")
                    .modal(true)
                    .transient_for(&win)
                    .build();
                about.present();
            });
            window.add_action(&action);
        }
    }

    fn audit_visible_actions(window: &adw::ApplicationWindow) {
        for action_name in VISIBLE_WINDOW_ACTIONS {
            if window.lookup_action(action_name).is_none() {
                log::warn!("Visible window action '{}' is not registered", action_name);
            }
        }
    }

    fn show_image_size_dialog(
        parent: &adw::ApplicationWindow,
        document: &Rc<RefCell<Document>>,
        canvas: &gtk4::DrawingArea,
    ) {
        Self::show_size_dialog(parent, document, canvas, "Image Size", "Resize Image", true);
    }

    fn show_upscale_dialog(
        parent: &adw::ApplicationWindow,
        document: &Rc<RefCell<Document>>,
        canvas: &gtk4::DrawingArea,
    ) {
        let dialog = gtk4::Window::new();
        dialog.set_title(Some("Upscale Image"));
        dialog.set_transient_for(Some(parent));
        dialog.set_modal(true);
        dialog.set_default_size(320, -1);

        let scale_adj = gtk4::Adjustment::new(2.0, 1.1, 10.0, 0.1, 1.0, 0.0);
        let scale_spin = gtk4::SpinButton::new(Some(&scale_adj), 0.1, 1);
        scale_spin.set_valign(gtk4::Align::Center);

        let scale_row = adw::ActionRow::new();
        scale_row.set_title("Scale Factor");
        scale_row.set_subtitle("High-quality Lanczos3 upscaling");
        scale_row.add_suffix(&scale_spin);

        let group = adw::PreferencesGroup::new();
        group.set_margin_top(12);
        group.set_margin_bottom(12);
        group.set_margin_start(12);
        group.set_margin_end(12);
        group.add(&scale_row);

        let cancel_btn = gtk4::Button::with_label("Cancel");
        let upscale_btn = gtk4::Button::with_label("Upscale");
        upscale_btn.add_css_class("suggested-action");

        let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        buttons.set_halign(gtk4::Align::End);
        buttons.set_margin_start(12);
        buttons.set_margin_end(12);
        buttons.set_margin_bottom(12);
        buttons.append(&cancel_btn);
        buttons.append(&upscale_btn);

        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        content.append(&group);
        content.append(&buttons);
        dialog.set_child(Some(&content));

        {
            let dlg = dialog.clone();
            cancel_btn.connect_clicked(move |_| dlg.close());
        }
        {
            let doc = document.clone();
            let cw = canvas.clone();
            let dlg = dialog.clone();
            upscale_btn.connect_clicked(move |_| {
                let factor = scale_spin.value();
                let mut next = doc.borrow().clone();
                if let Err(e) = crate::image::upscale::upscale_document(&mut next, factor) {
                    eprintln!("Upscale failed: {}", e);
                } else {
                    Self::execute_document_command(
                        &doc,
                        Box::new(ReplaceDocumentCommand::new("Upscale Image", next)),
                    );
                    cw.queue_draw();
                }
                dlg.close();
            });
        }

        dialog.present();
    }

    fn show_canvas_size_dialog(
        parent: &adw::ApplicationWindow,
        document: &Rc<RefCell<Document>>,
        canvas: &gtk4::DrawingArea,
    ) {
        Self::show_size_dialog(
            parent,
            document,
            canvas,
            "Canvas Size",
            "Resize Canvas",
            false,
        );
    }

    fn show_size_dialog(
        parent: &adw::ApplicationWindow,
        document: &Rc<RefCell<Document>>,
        canvas: &gtk4::DrawingArea,
        title: &'static str,
        command_name: &'static str,
        scale_pixels: bool,
    ) {
        let (current_w, current_h) = {
            let doc = document.borrow();
            (doc.canvas_width, doc.canvas_height)
        };

        let dialog = gtk4::Window::new();
        dialog.set_title(Some(title));
        dialog.set_transient_for(Some(parent));
        dialog.set_modal(true);
        dialog.set_default_size(320, -1);

        let width_adj = gtk4::Adjustment::new(current_w as f64, 1.0, 32768.0, 1.0, 10.0, 0.0);
        let height_adj = gtk4::Adjustment::new(current_h as f64, 1.0, 32768.0, 1.0, 10.0, 0.0);
        let width_spin = gtk4::SpinButton::new(Some(&width_adj), 1.0, 0);
        let height_spin = gtk4::SpinButton::new(Some(&height_adj), 1.0, 0);

        let width_row = adw::ActionRow::new();
        width_row.set_title("Width");
        width_row.add_suffix(&width_spin);
        width_spin.set_valign(gtk4::Align::Center);

        let height_row = adw::ActionRow::new();
        height_row.set_title("Height");
        height_row.add_suffix(&height_spin);
        height_spin.set_valign(gtk4::Align::Center);

        let group = adw::PreferencesGroup::new();
        group.set_margin_top(12);
        group.set_margin_bottom(12);
        group.set_margin_start(12);
        group.set_margin_end(12);
        group.add(&width_row);
        group.add(&height_row);

        let cancel_btn = gtk4::Button::with_label("Cancel");
        let apply_btn = gtk4::Button::with_label("Apply");
        apply_btn.add_css_class("suggested-action");

        let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        buttons.set_halign(gtk4::Align::End);
        buttons.set_margin_start(12);
        buttons.set_margin_end(12);
        buttons.set_margin_bottom(12);
        buttons.append(&cancel_btn);
        buttons.append(&apply_btn);

        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        content.append(&group);
        content.append(&buttons);
        dialog.set_child(Some(&content));

        {
            let dlg = dialog.clone();
            cancel_btn.connect_clicked(move |_| dlg.close());
        }
        {
            let doc = document.clone();
            let cw = canvas.clone();
            let dlg = dialog.clone();
            apply_btn.connect_clicked(move |_| {
                let width = width_spin.value() as u32;
                let height = height_spin.value() as u32;
                let mut next = doc.borrow().clone();
                if scale_pixels {
                    Self::resize_document_pixels(&mut next, width, height);
                } else {
                    Self::resize_document_canvas(&mut next, width, height);
                }
                Self::execute_document_command(
                    &doc,
                    Box::new(ReplaceDocumentCommand::new(command_name, next)),
                );
                cw.queue_draw();
                dlg.close();
            });
        }

        dialog.present();
    }

    fn resize_document_pixels(document: &mut Document, width: u32, height: u32) {
        let old_w = document.canvas_width.max(1);
        let old_h = document.canvas_height.max(1);
        for layer in &mut document.layers {
            if let LayerKind::Raster(raster) = &mut layer.kind {
                raster.data = Self::resize_rgba_nearest(
                    &raster.data,
                    raster.width,
                    raster.height,
                    width,
                    height,
                );
                raster.width = width;
                raster.height = height;
            }
            if let Some(mask) = layer.mask.as_mut() {
                mask.data =
                    Self::resize_mask_nearest(&mask.data, mask.width, mask.height, width, height);
                mask.width = width;
                mask.height = height;
            }
        }
        document.canvas_width = width.max(1);
        document.canvas_height = height.max(1);
        if old_w != 0 && old_h != 0 {
            document.revision += 1;
        }
    }

    fn resize_document_canvas(document: &mut Document, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        for layer in &mut document.layers {
            if let LayerKind::Raster(raster) = &mut layer.kind {
                raster.data = Self::fit_rgba_to_canvas(
                    &raster.data,
                    raster.width,
                    raster.height,
                    width,
                    height,
                );
                raster.width = width;
                raster.height = height;
            }
            if let Some(mask) = layer.mask.as_mut() {
                mask.data =
                    Self::fit_mask_to_canvas(&mask.data, mask.width, mask.height, width, height);
                mask.width = width;
                mask.height = height;
            }
        }
        document.canvas_width = width;
        document.canvas_height = height;
        document.revision += 1;
    }

    fn rotate_document_clockwise(document: &mut Document) {
        for layer in &mut document.layers {
            if let LayerKind::Raster(raster) = &mut layer.kind {
                raster.data =
                    Self::rotate_rgba_clockwise(&raster.data, raster.width, raster.height);
                std::mem::swap(&mut raster.width, &mut raster.height);
            }
            if let Some(mask) = layer.mask.as_mut() {
                mask.data = Self::rotate_mask_clockwise(&mask.data, mask.width, mask.height);
                std::mem::swap(&mut mask.width, &mut mask.height);
            }
        }
        std::mem::swap(&mut document.canvas_width, &mut document.canvas_height);
        document.revision += 1;
    }

    fn flip_document_horizontal(document: &mut Document) {
        for layer in &mut document.layers {
            if let LayerKind::Raster(raster) = &mut layer.kind {
                let mut flipped = raster.data.clone();
                for y in 0..raster.height {
                    for x in 0..raster.width {
                        let src_idx = ((y * raster.width + x) * 4) as usize;
                        let dst_idx = ((y * raster.width + (raster.width - 1 - x)) * 4) as usize;
                        flipped[dst_idx..dst_idx + 4]
                            .copy_from_slice(&raster.data[src_idx..src_idx + 4]);
                    }
                }
                raster.data = flipped;
            }
            if let Some(mask) = layer.mask.as_mut() {
                let mut flipped = mask.data.clone();
                for y in 0..mask.height {
                    for x in 0..mask.width {
                        let src_idx = (y * mask.width + x) as usize;
                        let dst_idx = (y * mask.width + (mask.width - 1 - x)) as usize;
                        flipped[dst_idx] = mask.data[src_idx];
                    }
                }
                mask.data = flipped;
            }
        }
        document.revision += 1;
    }

    fn flip_document_vertical(document: &mut Document) {
        for layer in &mut document.layers {
            if let LayerKind::Raster(raster) = &mut layer.kind {
                let mut flipped = raster.data.clone();
                for y in 0..raster.height {
                    let src_y = y;
                    let dst_y = raster.height - 1 - y;
                    let src_idx = (src_y * raster.width * 4) as usize;
                    let dst_idx = (dst_y * raster.width * 4) as usize;
                    let len = (raster.width * 4) as usize;
                    flipped[dst_idx..dst_idx + len].copy_from_slice(&raster.data[src_idx..src_idx + len]);
                }
                raster.data = flipped;
            }
            if let Some(mask) = layer.mask.as_mut() {
                let mut flipped = mask.data.clone();
                for y in 0..mask.height {
                    let src_y = y;
                    let dst_y = mask.height - 1 - y;
                    let src_idx = (src_y * mask.width) as usize;
                    let dst_idx = (dst_y * mask.width) as usize;
                    let len = mask.width as usize;
                    flipped[dst_idx..dst_idx + len].copy_from_slice(&mask.data[src_idx..src_idx + len]);
                }
                mask.data = flipped;
            }
        }
        document.revision += 1;
    }

    fn merged_down_document(document: &Document) -> Option<Document> {
        let active_id = document.active_layer_id?;
        let top_index = document
            .layers
            .iter()
            .position(|layer| layer.id == active_id)?;
        if top_index == 0 {
            return None;
        }
        let bottom_index = top_index - 1;
        let mut temp = Document::new(document.canvas_width, document.canvas_height);
        temp.layers = vec![
            document.layers[bottom_index].clone(),
            document.layers[top_index].clone(),
        ];
        let flattened = flatten_frame_bgra(&build_render_frame(&temp));
        let mut rgba = Vec::with_capacity(flattened.pixels_bgra.len());
        for px in flattened.pixels_bgra.chunks_exact(4) {
            rgba.push(px[2]);
            rgba.push(px[1]);
            rgba.push(px[0]);
            rgba.push(px[3]);
        }

        let mut merged = Layer::new_raster("Merged Layer", flattened.width, flattened.height, rgba);
        merged.name = format!(
            "{} + {}",
            document.layers[bottom_index].name, document.layers[top_index].name
        );
        let merged_id = merged.id;

        let mut next = document.clone();
        next.layers[bottom_index] = merged;
        next.layers.remove(top_index);
        next.active_layer_id = Some(merged_id);
        next.revision += 1;
        next.has_unsaved_changes = true;
        Some(next)
    }

    fn apply_active_raster_filter(
        document: &mut Document,
        filter: fn(&mut [u8], u32, u32),
    ) -> bool {
        let Some(active_id) = document.active_layer_id else {
            return false;
        };
        let Some(layer) = document
            .layers
            .iter_mut()
            .find(|layer| layer.id == active_id)
        else {
            return false;
        };
        let LayerKind::Raster(raster) = &mut layer.kind else {
            return false;
        };
        filter(&mut raster.data, raster.width, raster.height);
        document.revision += 1;
        document.has_unsaved_changes = true;
        true
    }

    fn resize_rgba_nearest(data: &[u8], old_w: u32, old_h: u32, new_w: u32, new_h: u32) -> Vec<u8> {
        let new_w = new_w.max(1);
        let new_h = new_h.max(1);
        let old_w = old_w.max(1);
        let old_h = old_h.max(1);
        let mut out = vec![0u8; new_w as usize * new_h as usize * 4];
        for y in 0..new_h {
            for x in 0..new_w {
                let src_x = (x as u64 * old_w as u64 / new_w as u64) as u32;
                let src_y = (y as u64 * old_h as u64 / new_h as u64) as u32;
                let src = ((src_y.min(old_h - 1) * old_w + src_x.min(old_w - 1)) * 4) as usize;
                let dst = ((y * new_w + x) * 4) as usize;
                if src + 3 < data.len() {
                    out[dst..dst + 4].copy_from_slice(&data[src..src + 4]);
                }
            }
        }
        out
    }

    fn resize_mask_nearest(data: &[u8], old_w: u32, old_h: u32, new_w: u32, new_h: u32) -> Vec<u8> {
        let new_w = new_w.max(1);
        let new_h = new_h.max(1);
        let old_w = old_w.max(1);
        let old_h = old_h.max(1);
        let mut out = vec![255u8; new_w as usize * new_h as usize];
        for y in 0..new_h {
            for x in 0..new_w {
                let src_x = (x as u64 * old_w as u64 / new_w as u64) as u32;
                let src_y = (y as u64 * old_h as u64 / new_h as u64) as u32;
                let src = (src_y.min(old_h - 1) * old_w + src_x.min(old_w - 1)) as usize;
                let dst = (y * new_w + x) as usize;
                if src < data.len() {
                    out[dst] = data[src];
                }
            }
        }
        out
    }

    fn fit_rgba_to_canvas(data: &[u8], old_w: u32, old_h: u32, new_w: u32, new_h: u32) -> Vec<u8> {
        let mut out = vec![0u8; new_w as usize * new_h as usize * 4];
        let copy_w = old_w.min(new_w);
        let copy_h = old_h.min(new_h);
        for y in 0..copy_h {
            let src = (y * old_w * 4) as usize;
            let dst = (y * new_w * 4) as usize;
            let len = (copy_w * 4) as usize;
            if src + len <= data.len() && dst + len <= out.len() {
                out[dst..dst + len].copy_from_slice(&data[src..src + len]);
            }
        }
        out
    }

    fn fit_mask_to_canvas(data: &[u8], old_w: u32, old_h: u32, new_w: u32, new_h: u32) -> Vec<u8> {
        let mut out = vec![255u8; new_w as usize * new_h as usize];
        let copy_w = old_w.min(new_w);
        let copy_h = old_h.min(new_h);
        for y in 0..copy_h {
            let src = (y * old_w) as usize;
            let dst = (y * new_w) as usize;
            let len = copy_w as usize;
            if src + len <= data.len() && dst + len <= out.len() {
                out[dst..dst + len].copy_from_slice(&data[src..src + len]);
            }
        }
        out
    }

    fn rotate_rgba_clockwise(data: &[u8], width: u32, height: u32) -> Vec<u8> {
        let mut out = vec![0u8; width as usize * height as usize * 4];
        for y in 0..height {
            for x in 0..width {
                let src = ((y * width + x) * 4) as usize;
                let dst_x = height - 1 - y;
                let dst_y = x;
                let dst = ((dst_y * height + dst_x) * 4) as usize;
                if src + 3 < data.len() && dst + 3 < out.len() {
                    out[dst..dst + 4].copy_from_slice(&data[src..src + 4]);
                }
            }
        }
        out
    }

    fn rotate_mask_clockwise(data: &[u8], width: u32, height: u32) -> Vec<u8> {
        let mut out = vec![255u8; width as usize * height as usize];
        for y in 0..height {
            for x in 0..width {
                let src = (y * width + x) as usize;
                let dst_x = height - 1 - y;
                let dst_y = x;
                let dst = (dst_y * height + dst_x) as usize;
                if src < data.len() && dst < out.len() {
                    out[dst] = data[src];
                }
            }
        }
        out
    }

    fn filter_blur(data: &mut [u8], width: u32, height: u32) {
        if let Ok(img) = libvips::VipsImage::new_from_memory(
            data,
            width as i32,
            height as i32,
            4,
            libvips::ops::BandFormat::Uchar,
        ) {
            if let Ok(blurred) = libvips::ops::gaussblur(&img, 2.0) {
                let bytes = blurred.image_write_to_memory();
                data.copy_from_slice(&bytes);
            }
        }
    }

    fn filter_sharpen(data: &mut [u8], width: u32, height: u32) {
        if let Ok(img) = libvips::VipsImage::new_from_memory(
            data,
            width as i32,
            height as i32,
            4,
            libvips::ops::BandFormat::Uchar,
        ) {
            if let Ok(sharpened) = libvips::ops::sharpen(&img) {
                let bytes = sharpened.image_write_to_memory();
                data.copy_from_slice(&bytes);
            }
        }
    }

    fn filter_noise(data: &mut [u8], width: u32, height: u32) {
        for y in 0..height {
            for x in 0..width {
                let n = (((x as u64 * 73_856_093) ^ (y as u64 * 19_349_663)) % 31) as i16 - 15;
                let idx = ((y * width + x) * 4) as usize;
                for channel in 0..3 {
                    data[idx + channel] = (data[idx + channel] as i16 + n).clamp(0, 255) as u8;
                }
            }
        }
    }

    fn filter_invert(data: &mut [u8], _width: u32, _height: u32) {
        for i in 0..data.len() {
            if i % 4 == 3 {
                continue; // Skip alpha
            }
            data[i] = 255 - data[i];
        }
    }

    fn filter_grayscale(data: &mut [u8], _width: u32, _height: u32) {
        for i in (0..data.len()).step_by(4) {
            let b = data[i] as u32;
            let g = data[i + 1] as u32;
            let r = data[i + 2] as u32;
            let gray = ((r * 30 + g * 59 + b * 11) / 100) as u8;
            data[i] = gray;
            data[i + 1] = gray;
            data[i + 2] = gray;
        }
    }

    fn toggle_mask_edit(mask: &mut Mask) {
        mask.editing = !mask.editing;
    }

    fn toggle_mask_enabled(mask: &mut Mask) {
        mask.enabled = !mask.enabled;
    }

    fn toggle_mask_view(mask: &mut Mask) {
        mask.show_on_canvas = !mask.show_on_canvas;
    }

    fn execute_document_command(document: &Rc<RefCell<Document>>, command: Box<dyn Command>) {
        let mut doc = document.borrow_mut();
        let mut undo_stack = std::mem::take(&mut doc.undo_stack);
        undo_stack.execute(command, &mut doc);
        doc.undo_stack = undo_stack;
    }

    fn show_keyboard_shortcuts(parent: &adw::ApplicationWindow) {
        let dialog = gtk4::Window::new();
        dialog.set_title(Some("Keyboard Shortcuts"));
        dialog.set_transient_for(Some(parent));
        dialog.set_modal(true);
        dialog.set_default_size(420, 520);

        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_hexpand(true);

        let list = gtk4::ListBox::new();
        list.add_css_class("boxed-list");
        list.set_margin_top(12);
        list.set_margin_bottom(12);
        list.set_margin_start(12);
        list.set_margin_end(12);

        for shortcut in SHORTCUTS {
            let row = adw::ActionRow::new();
            row.set_title(shortcut.command);
            row.set_subtitle(shortcut.display);
            list.append(&row);
        }

        scrolled.set_child(Some(&list));
        dialog.set_child(Some(&scrolled));
        dialog.present();
    }

    fn create_header(
        document: &Rc<RefCell<Document>>,
        pipeline: &Rc<RefCell<EditPipeline>>,
        export_params: &Rc<RefCell<ExportParams>>,
        zoom: &Rc<RefCell<f64>>,
        canvas: &gtk4::DrawingArea,
        stack: &adw::ViewStack,
    ) -> adw::HeaderBar {
        let header = adw::HeaderBar::new();

        let open_btn = gtk4::Button::builder()
            .icon_name("document-open-symbolic")
            .tooltip_text("Open image (Ctrl+O)")
            .build();

        let export_btn = gtk4::Button::builder()
            .icon_name("document-save-as-symbolic")
            .tooltip_text("Export (Ctrl+Shift+E)")
            .css_classes(vec!["suggested-action".to_string()])
            .build();

        let undo_btn = gtk4::Button::builder()
            .icon_name("edit-undo-symbolic")
            .tooltip_text("Undo (Ctrl+Z)")
            .build();

        let redo_btn = gtk4::Button::builder()
            .icon_name("edit-redo-symbolic")
            .tooltip_text("Redo (Ctrl+Shift+Z)")
            .build();

        let switcher = adw::ViewSwitcher::new();
        switcher.set_stack(Some(stack));
        switcher.set_policy(adw::ViewSwitcherPolicy::Wide);
        header.set_title_widget(Some(&switcher));

        header.pack_start(&open_btn);
        header.pack_start(&undo_btn);
        header.pack_start(&redo_btn);
        header.pack_end(&export_btn);

        let doc_open = document.clone();
        let pip_open = pipeline.clone();
        let zoom_open = zoom.clone();
        let c = canvas.clone();
        open_btn.connect_clicked(move |_| {
            Self::open_image_dialog(&doc_open, &pip_open, &zoom_open, &c);
        });

        {
            let doc = document.clone();
            let cw = canvas.clone();
            undo_btn.connect_clicked(move |_| {
                let mut d = doc.borrow_mut();
                let mut undo_stack = std::mem::take(&mut d.undo_stack);
                undo_stack.undo(&mut d);
                d.undo_stack = undo_stack;
                cw.queue_draw();
            });
        }

        {
            let doc = document.clone();
            let cw = canvas.clone();
            redo_btn.connect_clicked(move |_| {
                let mut d = doc.borrow_mut();
                let mut undo_stack = std::mem::take(&mut d.undo_stack);
                undo_stack.redo(&mut d);
                d.undo_stack = undo_stack;
                cw.queue_draw();
            });
        }

        let pip_export = pipeline.clone();
        let exp_export = export_params.clone();
        let doc_export = document.clone();
        export_btn.connect_clicked(move |_| {
            Self::run_export(&doc_export, &pip_export, &exp_export);
        });

        header
    }

    fn build_editor_layout(
        toolbar: &gtk4::Box,
        options_bar: &gtk4::Box,
        canvas: &gtk4::Overlay,
        panel_stack: &adw::ViewStack,
    ) -> (gtk4::Box, gtk4::Box) {
        let toolbar_frame = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        toolbar_frame.set_width_request(48);
        toolbar_frame.add_css_class("tool-panel");
        toolbar_frame.append(toolbar);

        let right_panel = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        right_panel.set_width_request(280);
        right_panel.append(panel_stack);

        options_bar.set_height_request(36);
        options_bar.add_css_class("options-bar");

        let toolbar_sep = gtk4::Separator::new(gtk4::Orientation::Vertical);
        let panel_sep = gtk4::Separator::new(gtk4::Orientation::Vertical);

        let center_vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        center_vbox.append(options_bar);
        let opts_sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        center_vbox.append(&opts_sep);
        canvas.set_hexpand(true);
        canvas.set_vexpand(true);
        center_vbox.append(canvas);

        let work_hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        work_hbox.append(&toolbar_frame);
        work_hbox.append(&toolbar_sep);
        work_hbox.append(&center_vbox);
        work_hbox.append(&panel_sep);
        work_hbox.append(&right_panel);

        let editor_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        editor_box.append(&work_hbox);

        let right_panel_clone = right_panel.clone();

        (editor_box, right_panel_clone)
    }

    fn hide_chrome(
        toolbar: &gtk4::Box,
        options: &gtk4::Box,
        right_panel: &gtk4::Box,
        header: &adw::HeaderBar,
        status: &gtk4::Box,
        menubar: &gtk4::PopoverMenuBar,
    ) {
        toolbar.set_visible(false);
        options.set_visible(false);
        right_panel.set_visible(false);
        header.set_visible(false);
        status.set_visible(false);
        menubar.set_visible(false);
    }

    fn show_chrome(
        toolbar: &gtk4::Box,
        options: &gtk4::Box,
        right_panel: &gtk4::Box,
        header: &adw::HeaderBar,
        status: &gtk4::Box,
        menubar: &gtk4::PopoverMenuBar,
    ) {
        toolbar.set_visible(true);
        options.set_visible(true);
        right_panel.set_visible(true);
        header.set_visible(true);
        status.set_visible(true);
        menubar.set_visible(true);
    }

    fn create_status_bar(
        _zoom: &Rc<RefCell<f64>>,
        _document: &Rc<RefCell<Document>>,
        _pipeline: &Rc<RefCell<EditPipeline>>,
    ) -> (gtk4::Box, gtk4::Label, gtk4::Label, gtk4::Label) {
        let status = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        status.set_margin_start(12);
        status.set_margin_end(12);
        status.set_margin_top(4);
        status.set_margin_bottom(4);
        status.add_css_class("status-bar");

        let zoom_label = gtk4::Label::new(Some("100%"));
        zoom_label.add_css_class("dim-label");

        let dims_label = gtk4::Label::new(Some("--"));
        dims_label.add_css_class("dim-label");

        let output_label = gtk4::Label::new(Some("--"));
        output_label.add_css_class("dim-label");

        status.append(&zoom_label);
        status.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));
        status.append(&dims_label);
        status.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));
        status.append(&output_label);

        (status, zoom_label, dims_label, output_label)
    }

    fn open_image(
        document: &Rc<RefCell<Document>>,
        pipeline: &Rc<RefCell<EditPipeline>>,
        path: &std::path::Path,
    ) {
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                log::error!("Failed to read file: {}", e);
                return;
            }
        };

        let img = match libvips::VipsImage::new_from_buffer(&data, "") {
            Ok(img) => img,
            Err(e) => {
                log::error!("Failed to load image: {}", e);
                return;
            }
        };

        let w = img.get_width() as u32;
        let h = img.get_height() as u32;

        let has_alpha = img.image_hasalpha();

        let img = if !has_alpha {
            match libvips::ops::addalpha(&img) {
                Ok(with_alpha) => with_alpha,
                Err(_) => img,
            }
        } else {
            img
        };

        let raw = img.image_write_to_memory();

        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for row in 0..h as usize {
            for col in 0..w as usize {
                let si = (row * w as usize + col) * 4;
                let di = (row * w as usize + col) * 4;
                if si + 3 < raw.len() && di + 3 < rgba.len() {
                    rgba[di] = raw[si];
                    rgba[di + 1] = raw[si + 1];
                    rgba[di + 2] = raw[si + 2];
                    rgba[di + 3] = raw[si + 3];
                }
            }
        }

        let mut doc = document.borrow_mut();
        let layer = Layer::new_raster(
            path.file_stem().and_then(|s| s.to_str()).unwrap_or("Layer"),
            w,
            h,
            rgba,
        );
        doc.layers.clear();
        doc.add_layer(layer);
        doc.canvas_width = w;
        doc.canvas_height = h;
        doc.file_path = Some(path.to_string_lossy().to_string());
        doc.undo_stack = Default::default();
        doc.has_unsaved_changes = false;
        *pipeline.borrow_mut() = EditPipeline::default();

        log::info!("Opened image: {} ({}x{})", path.display(), w, h);
    }

    pub fn open_path(&self, path: &std::path::Path) {
        Self::open_image(&self.document, &self.pipeline, path);
        Self::queue_fit_to_screen(&self.document, &self.zoom, self.canvas.widget());
    }

    fn open_image_dialog(
        document: &Rc<RefCell<Document>>,
        pipeline: &Rc<RefCell<EditPipeline>>,
        zoom: &Rc<RefCell<f64>>,
        canvas: &gtk4::DrawingArea,
    ) {
        let file_filter = gtk4::FileFilter::new();
        file_filter.add_mime_type("image/jpeg");
        file_filter.add_mime_type("image/png");
        file_filter.add_mime_type("image/webp");
        file_filter.add_mime_type("image/tiff");
        file_filter.set_name(Some("Images"));

        let dialog = gtk4::FileDialog::new();
        dialog.set_default_filter(Some(&file_filter));

        let doc = document.clone();
        let pip = pipeline.clone();
        let zoom = zoom.clone();
        let cw = canvas.clone();
        dialog.open(
            None::<&gtk4::Window>,
            gio::Cancellable::NONE,
            move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        Self::open_image(&doc, &pip, &path);
                        Self::queue_fit_to_screen(&doc, &zoom, &cw);
                    }
                }
            },
        );
    }

    fn queue_fit_to_screen(
        document: &Rc<RefCell<Document>>,
        zoom: &Rc<RefCell<f64>>,
        canvas: &gtk4::DrawingArea,
    ) {
        let doc = document.clone();
        let zoom = zoom.clone();
        let cw = canvas.clone();
        glib::idle_add_local_once(move || {
            let document = doc.borrow();
            if document.canvas_width == 0 || document.canvas_height == 0 {
                cw.queue_draw();
                return;
            }

            let viewport_width = cw.width().max(1) as f64;
            let viewport_height = cw.height().max(1) as f64;
            let scale_x = viewport_width / document.canvas_width as f64;
            let scale_y = viewport_height / document.canvas_height as f64;
            let scale = scale_x.min(scale_y).min(1.0).max(0.01);
            drop(document);

            *zoom.borrow_mut() = scale;
            cw.queue_draw();
        });
    }

    fn run_export(
        document: &Rc<RefCell<Document>>,
        pipeline: &Rc<RefCell<EditPipeline>>,
        export_params: &Rc<RefCell<ExportParams>>,
    ) {
        let doc = document.borrow();
        let frame = build_render_frame(&doc);
        drop(doc);

        let flattened = flatten_frame_bgra(&frame);
        if flattened.pixels_bgra.is_empty() {
            log::warn!("No raster layer to export");
            return;
        }

        let mut rgba = Vec::with_capacity(flattened.pixels_bgra.len());
        for px in flattened.pixels_bgra.chunks_exact(4) {
            rgba.push(px[2]);
            rgba.push(px[1]);
            rgba.push(px[0]);
            rgba.push(px[3]);
        }

        let pip = pipeline.borrow();
        let params = export_params.borrow();
        let export_format = params.format;
        let result = crate::image::execute_pipeline_rgba(
            &rgba,
            flattened.width,
            flattened.height,
            &pip,
            &params,
        );
        drop(pip);
        drop(params);

        match result {
            crate::image::PipelineResult::Success(buf, w, h) => {
                let file_filter = gtk4::FileFilter::new();
                file_filter.add_mime_type(Self::export_mime_type(export_format));
                file_filter.set_name(Some(Self::export_filter_name(export_format)));

                let dialog = gtk4::FileDialog::new();

                let current_path = document.borrow().file_path.clone();
                let default_name = current_path
                    .as_ref()
                    .and_then(|p| std::path::Path::new(p).file_stem())
                    .and_then(|s| s.to_str())
                    .map(|s| format!("{}_exported.{}", s, Self::export_extension(export_format)))
                    .unwrap_or_else(|| format!("export.{}", Self::export_extension(export_format)));

                let default_file = gio::File::for_path(&default_name);
                dialog.set_initial_file(Some(&default_file));

                let buf_clone = buf.clone();
                dialog.save(
                    None::<&gtk4::Window>,
                    gio::Cancellable::NONE,
                    move |result| {
                        if let Ok(file) = result {
                            if let Some(path) = file.path() {
                                if let Err(e) = std::fs::write(&path, &buf_clone) {
                                    log::error!("Failed to write export: {}", e);
                                } else {
                                    log::info!("Exported to {} ({}x{})", path.display(), w, h);
                                }
                            }
                        }
                    },
                );
            }
            crate::image::PipelineResult::Error(e) => {
                log::error!("Export failed: {}", e);
            }
        }
    }

    fn export_extension(format: crate::image::pipeline::ExportFormat) -> &'static str {
        match format {
            crate::image::pipeline::ExportFormat::Png => "png",
            crate::image::pipeline::ExportFormat::Jpeg => "jpg",
            crate::image::pipeline::ExportFormat::WebP => "webp",
        }
    }

    fn export_mime_type(format: crate::image::pipeline::ExportFormat) -> &'static str {
        match format {
            crate::image::pipeline::ExportFormat::Png => "image/png",
            crate::image::pipeline::ExportFormat::Jpeg => "image/jpeg",
            crate::image::pipeline::ExportFormat::WebP => "image/webp",
        }
    }

    fn export_filter_name(format: crate::image::pipeline::ExportFormat) -> &'static str {
        match format {
            crate::image::pipeline::ExportFormat::Png => "PNG Image",
            crate::image::pipeline::ExportFormat::Jpeg => "JPEG Image",
            crate::image::pipeline::ExportFormat::WebP => "WebP Image",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn shortcut_registry_has_unique_action_entries() {
        let mut seen = HashSet::new();
        for shortcut in SHORTCUTS.iter().filter_map(|shortcut| shortcut.action) {
            assert!(
                seen.insert(shortcut),
                "duplicate shortcut action: {shortcut}"
            );
        }
    }

    #[test]
    fn visible_window_actions_are_unique() {
        let mut seen = HashSet::new();
        for action in VISIBLE_WINDOW_ACTIONS {
            assert!(seen.insert(action), "duplicate visible action: {action}");
        }
    }

    #[test]
    fn resize_rgba_nearest_preserves_corner_pixels() {
        let data = vec![10, 0, 0, 255, 20, 0, 0, 255, 30, 0, 0, 255, 40, 0, 0, 255];

        let resized = MainWindow::resize_rgba_nearest(&data, 2, 2, 4, 4);

        assert_eq!(resized[0], 10);
        assert_eq!(resized[(3 * 4 + 3) * 4], 40);
    }

    #[test]
    fn rotate_rgba_clockwise_swaps_dimensions_in_pixel_order() {
        let data = vec![
            1, 0, 0, 255, 2, 0, 0, 255, 3, 0, 0, 255, 4, 0, 0, 255, 5, 0, 0, 255, 6, 0, 0, 255,
        ];

        let rotated = MainWindow::rotate_rgba_clockwise(&data, 3, 2);
        let red_values: Vec<u8> = rotated.chunks_exact(4).map(|px| px[0]).collect();

        assert_eq!(red_values, vec![4, 1, 5, 2, 6, 3]);
    }

    #[test]
    fn merge_down_flattens_active_layer_into_layer_below() {
        let mut doc = Document::new(1, 1);
        let bottom = Layer::new_raster("Bottom", 1, 1, vec![255, 0, 0, 255]);
        let mut top = Layer::new_raster("Top", 1, 1, vec![0, 0, 255, 255]);
        top.opacity = 0.5;
        let top_id = top.id;
        doc.add_layer(bottom);
        doc.add_layer(top);
        doc.active_layer_id = Some(top_id);

        let merged = MainWindow::merged_down_document(&doc).unwrap();

        assert_eq!(merged.layers.len(), 1);
        let LayerKind::Raster(raster) = &merged.layers[0].kind else {
            panic!("expected raster layer");
        };
        assert!(raster.data[0] > 120);
        assert_eq!(raster.data[1], 0);
        assert!(raster.data[2] > 120);
        assert_eq!(raster.data[3], 255);
    }

    #[test]
    fn export_dialog_metadata_matches_selected_format() {
        use crate::image::pipeline::ExportFormat;

        assert_eq!(MainWindow::export_extension(ExportFormat::Png), "png");
        assert_eq!(
            MainWindow::export_mime_type(ExportFormat::Jpeg),
            "image/jpeg"
        );
        assert_eq!(
            MainWindow::export_filter_name(ExportFormat::WebP),
            "WebP Image"
        );
    }
}
