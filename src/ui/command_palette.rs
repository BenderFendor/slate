#![allow(dead_code)]

use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct CommandPalette {
    window: gtk4::Window,
    entry: gtk4::Entry,
    listbox: gtk4::ListBox,
    commands: Rc<RefCell<Vec<PaletteCommand>>>,
}

#[derive(Clone)]
pub struct PaletteCommand {
    pub name: String,
    pub description: String,
    pub shortcut: Option<String>,
    pub action: Rc<dyn Fn()>,
}

impl PaletteCommand {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        shortcut: Option<&str>,
        action: impl Fn() + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            shortcut: shortcut.map(|s| s.to_string()),
            action: Rc::new(action),
        }
    }
}

impl CommandPalette {
    pub fn new(parent: &gtk4::Window) -> Self {
        let window = gtk4::Window::new();
        window.set_modal(true);
        window.set_transient_for(Some(parent));
        window.set_default_size(500, 300);
        window.set_resizable(false);
        window.set_deletable(false);
        window.set_title(Some("Command Palette"));

        let entry = gtk4::Entry::new();
        entry.set_placeholder_text(Some("Type a command..."));
        entry.set_margin_top(12);
        entry.set_margin_start(12);
        entry.set_margin_end(12);
        entry.set_margin_bottom(6);

        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_hexpand(true);

        let listbox = gtk4::ListBox::new();
        listbox.set_margin_start(6);
        listbox.set_margin_end(6);
        listbox.set_margin_bottom(6);

        scrolled.set_child(Some(&listbox));

        let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        vbox.append(&entry);
        vbox.append(&scrolled);

        window.set_child(Some(&vbox));

        let commands = Rc::new(RefCell::new(Self::default_commands(parent)));

        let palette = Self {
            window,
            entry,
            listbox,
            commands,
        };

        palette.connect_signals();
        palette.refresh_list("");

        palette
    }

    fn default_commands(parent: &gtk4::Window) -> Vec<PaletteCommand> {
        let action = |name: &'static str| {
            let parent = parent.downgrade();
            move || {
                if let Some(parent) = parent.upgrade() {
                    if let Err(error) = parent.activate_action(name, None) {
                        log::warn!(
                            "Command palette action '{}' was not handled: {}",
                            name,
                            error
                        );
                    }
                }
            }
        };

        vec![
            PaletteCommand::new(
                "Open Image",
                "Open an image file from disk",
                Some("Ctrl+O"),
                action("open"),
            ),
            PaletteCommand::new(
                "Export",
                "Export the current image",
                Some("Ctrl+Shift+E"),
                action("export"),
            ),
            PaletteCommand::new(
                "Crop Tool",
                "Switch to the crop tool",
                Some("C"),
                action("crop"),
            ),
            PaletteCommand::new(
                "Brush Tool",
                "Switch to the brush tool",
                Some("B"),
                action("tool-brush"),
            ),
            PaletteCommand::new(
                "Move Tool",
                "Switch to the move tool",
                Some("V"),
                action("tool-move"),
            ),
            PaletteCommand::new(
                "Eraser Tool",
                "Switch to the eraser tool",
                Some("E"),
                action("tool-eraser"),
            ),
            PaletteCommand::new(
                "Zoom Tool",
                "Switch to the zoom tool",
                Some("Z"),
                action("tool-zoom"),
            ),
            PaletteCommand::new(
                "Undo",
                "Undo the last action",
                Some("Ctrl+Z"),
                action("undo"),
            ),
            PaletteCommand::new(
                "Redo",
                "Redo the last undone action",
                Some("Ctrl+Shift+Z"),
                action("redo"),
            ),
            PaletteCommand::new(
                "Canvas Only Mode",
                "Toggle canvas-only view, hiding side panels",
                None,
                action("workspace-canvas-only"),
            ),
            PaletteCommand::new(
                "Keyboard Shortcuts",
                "Show the configured keyboard shortcuts",
                Some("Ctrl+Shift+K"),
                action("keyboard-shortcuts"),
            ),
        ]
    }

    fn connect_signals(&self) {
        let listbox = self.listbox.clone();
        let commands = self.commands.clone();
        let window = self.window.clone();

        self.entry.connect_changed(move |entry| {
            let text = entry.text();
            Self::refresh_list_static(&listbox, &commands, &window, &text);
        });

        let window = self.window.clone();
        let key_ctrl = gtk4::EventControllerKey::new();
        key_ctrl.connect_key_pressed(move |_ctrl, keyval, _keycode, _modifiers| {
            if keyval.name() == Some(gtk4::glib::GString::from("Escape")) {
                window.set_visible(false);
                return gtk4::glib::Propagation::Stop;
            }
            gtk4::glib::Propagation::Proceed
        });
        self.window.add_controller(key_ctrl);

        self.window.connect_close_request(|w| {
            w.set_visible(false);
            gtk4::glib::Propagation::Stop
        });
    }

    fn refresh_list(&self, filter: &str) {
        Self::refresh_list_static(&self.listbox, &self.commands, &self.window, filter);
    }

    fn refresh_list_static(
        listbox: &gtk4::ListBox,
        commands: &RefCell<Vec<PaletteCommand>>,
        window: &gtk4::Window,
        filter: &str,
    ) {
        while let Some(child) = listbox.first_child() {
            listbox.remove(&child);
        }

        let filter_lower = filter.to_lowercase();
        let cmds = commands.borrow();

        for cmd in cmds.iter() {
            if !filter_lower.is_empty()
                && !cmd.name.to_lowercase().contains(&filter_lower)
                && !cmd.description.to_lowercase().contains(&filter_lower)
            {
                continue;
            }

            let action = cmd.action.clone();
            let w = window.clone();

            let row = gtk4::ListBoxRow::new();
            let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            hbox.set_margin_top(6);
            hbox.set_margin_bottom(6);
            hbox.set_margin_start(12);
            hbox.set_margin_end(12);

            let name_label = gtk4::Label::new(Some(&cmd.name));
            name_label.set_halign(gtk4::Align::Start);
            name_label.set_hexpand(true);
            hbox.append(&name_label);

            if let Some(ref shortcut) = cmd.shortcut {
                let shortcut_label = gtk4::Label::new(Some(shortcut));
                shortcut_label.add_css_class("dim-label");
                hbox.append(&shortcut_label);
            }

            row.set_child(Some(&hbox));
            row.connect_activate(move |_| {
                action();
                w.set_visible(false);
            });

            listbox.append(&row);
        }
    }

    pub fn show(&self) {
        self.entry.set_text("");
        self.window.present();
        self.entry.grab_focus();
    }

    pub fn widget(&self) -> &gtk4::Window {
        &self.window
    }
}
