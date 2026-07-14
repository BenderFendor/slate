#![allow(dead_code)]

use crate::tools::tool::ToolKind;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

const TOOL_GROUPS: &[(ToolKind, &str, &str)] = &[
    (ToolKind::Move, "Move (V)", "slate-move-symbolic"),
    (ToolKind::Lasso, "Lasso (L)", "slate-lasso-symbolic"),
    (ToolKind::Crop, "Crop (C)", "slate-crop-symbolic"),
    (ToolKind::Brush, "Brush (B)", "slate-brush-symbolic"),
    (ToolKind::Eraser, "Eraser (E)", "slate-eraser-symbolic"),
    (ToolKind::ColorPicker, "Color Picker (I)", "slate-pipette-symbolic"),
    (ToolKind::Zoom, "Zoom (Z)", "slate-zoom-symbolic"),
];

pub struct Toolbar {
    widget: gtk4::Box,
    pub active_tool: Rc<RefCell<ToolKind>>,
    buttons: Vec<gtk4::ToggleButton>,
    on_tool_change: Rc<RefCell<Option<Box<dyn Fn(ToolKind)>>>>,
}

impl Toolbar {
    pub fn new(active_tool: Rc<RefCell<ToolKind>>) -> Self {
        let box_widget = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        box_widget.set_width_request(48);
        box_widget.set_margin_start(4);
        box_widget.set_margin_end(4);
        box_widget.set_margin_top(4);
        box_widget.add_css_class("toolbar");
        box_widget.add_css_class("navigation-sidebar");

        let on_tool_change: Rc<RefCell<Option<Box<dyn Fn(ToolKind)>>>> =
            Rc::new(RefCell::new(None));

        let mut buttons: Vec<gtk4::ToggleButton> = Vec::with_capacity(TOOL_GROUPS.len());

        for (i, &(kind, tooltip, icon_name)) in TOOL_GROUPS.iter().enumerate() {
            if i == 2 || i == 4 {
                let sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
                sep.set_margin_start(8);
                sep.set_margin_end(8);
                sep.set_margin_top(4);
                sep.set_margin_bottom(4);
                box_widget.append(&sep);
            }
            let btn = gtk4::ToggleButton::builder()
                .tooltip_text(tooltip)
                .css_classes(vec!["flat".to_string()])
                .width_request(44)
                .height_request(44)
                .has_frame(false)
                .icon_name(icon_name)
                .build();

            if kind == *active_tool.borrow() {
                btn.set_active(true);
            }

            box_widget.append(&btn);
            buttons.push(btn);
        }

        for (i, btn) in buttons.iter().enumerate() {
            let all = buttons.clone();
            let kind = TOOL_GROUPS[i].0;
            let active = active_tool.clone();
            let notify = on_tool_change.clone();
            let this_btn = btn.clone();

            btn.connect_clicked(move |_| {
                for other in all.iter() {
                    other.set_active(false);
                }
                this_btn.set_active(true);
                *active.borrow_mut() = kind;
                if let Some(ref cb) = *notify.borrow() {
                    cb(kind);
                }
            });
        }

        Toolbar {
            widget: box_widget,
            active_tool,
            buttons,
            on_tool_change,
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }

    pub fn activate_tool(&self, kind: ToolKind) {
        *self.active_tool.borrow_mut() = kind;
        for (i, btn) in self.buttons.iter().enumerate() {
            btn.set_active(TOOL_GROUPS[i].0 == kind);
        }
        if let Some(ref cb) = *self.on_tool_change.borrow() {
            cb(kind);
        }
    }

    pub fn set_on_tool_change<F: Fn(ToolKind) + 'static>(&self, f: F) {
        *self.on_tool_change.borrow_mut() = Some(Box::new(f));
    }
}
