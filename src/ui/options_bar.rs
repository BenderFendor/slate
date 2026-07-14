#![allow(dead_code)]

use crate::document::Document;
use crate::image::pipeline::{CropRect, EditPipeline};
use crate::tools::tool::ToolKind;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

struct PresetDef {
    label: &'static str,
    aspect: Option<(f64, f64)>,
}

const PRESETS: &[PresetDef] = &[
    PresetDef {
        label: "Free",
        aspect: None,
    },
    PresetDef {
        label: "1:1",
        aspect: Some((1.0, 1.0)),
    },
    PresetDef {
        label: "4:3",
        aspect: Some((4.0, 3.0)),
    },
    PresetDef {
        label: "16:9",
        aspect: Some((16.0, 9.0)),
    },
];

pub struct OptionsBar {
    widget: gtk4::Box,
    stack: gtk4::Stack,
    pub brush_size: Rc<RefCell<f64>>,
    pub brush_hardness: Rc<RefCell<f64>>,
    pub brush_opacity: Rc<RefCell<f64>>,
    pub brush_flow: Rc<RefCell<f64>>,
    pub brush_color: Rc<RefCell<[f32; 4]>>,
}

impl OptionsBar {
    pub fn new(
        active_tool: Rc<RefCell<ToolKind>>,
        pipeline: Rc<RefCell<EditPipeline>>,
        document: Rc<RefCell<Document>>,
    ) -> Self {
        let widget = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        widget.add_css_class("options-bar");
        widget.set_margin_start(4);
        widget.set_margin_end(4);
        widget.set_margin_top(2);
        widget.set_margin_bottom(2);

        let stack = gtk4::Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
        stack.set_transition_duration(120);
        stack.set_hexpand(true);
        widget.append(&stack);

        let brush_size = Rc::new(RefCell::new(10.0));
        let brush_hardness = Rc::new(RefCell::new(0.8));
        let brush_opacity = Rc::new(RefCell::new(1.0));
        let brush_flow = Rc::new(RefCell::new(1.0));
        let brush_color = Rc::new(RefCell::new([0.0, 0.0, 0.0, 1.0]));

        // ---- Move page ----
        let move_page = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        move_page.set_margin_start(8);
        move_page.set_margin_end(8);
        let move_label = gtk4::Label::new(Some("Move Tool"));
        move_label.set_halign(gtk4::Align::Start);
        move_label.set_valign(gtk4::Align::Center);
        move_label.set_margin_top(4);
        move_label.set_margin_bottom(4);
        move_page.append(&move_label);
        stack.add_named(&move_page, Some("move"));

        // ---- Brush page ----
        let brush_page = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        brush_page.set_margin_start(8);
        brush_page.set_margin_end(8);
        brush_page.set_margin_top(2);
        brush_page.set_margin_bottom(2);

        let size_adj = gtk4::Adjustment::new(10.0, 1.0, 500.0, 1.0, 10.0, 0.0);
        let size_spin = gtk4::SpinButton::new(Some(&size_adj), 1.0, 0);
        size_spin.set_valign(gtk4::Align::Center);
        {
            let bs = brush_size.clone();
            size_adj.connect_value_changed(move |adj| {
                *bs.borrow_mut() = adj.value();
            });
        }
        let size_lbl = gtk4::Label::new(Some("Size:"));
        size_lbl.set_valign(gtk4::Align::Center);
        brush_page.append(&size_lbl);
        brush_page.append(&size_spin);

        let hard_adj = gtk4::Adjustment::new(0.8, 0.0, 1.0, 0.01, 0.1, 0.0);
        let hard_scale = gtk4::Scale::new(gtk4::Orientation::Horizontal, Some(&hard_adj));
        hard_scale.set_width_request(80);
        hard_scale.set_valign(gtk4::Align::Center);
        hard_scale.set_digits(2);
        {
            let bh = brush_hardness.clone();
            hard_adj.connect_value_changed(move |adj| {
                *bh.borrow_mut() = adj.value();
            });
        }
        let hard_lbl = gtk4::Label::new(Some("Hard:"));
        hard_lbl.set_valign(gtk4::Align::Center);
        brush_page.append(&hard_lbl);
        brush_page.append(&hard_scale);

        let opac_adj = gtk4::Adjustment::new(1.0, 0.0, 1.0, 0.01, 0.1, 0.0);
        let opac_scale = gtk4::Scale::new(gtk4::Orientation::Horizontal, Some(&opac_adj));
        opac_scale.set_width_request(80);
        opac_scale.set_valign(gtk4::Align::Center);
        opac_scale.set_digits(2);
        {
            let bo = brush_opacity.clone();
            opac_adj.connect_value_changed(move |adj| {
                *bo.borrow_mut() = adj.value();
            });
        }
        let opac_lbl = gtk4::Label::new(Some("Opac:"));
        opac_lbl.set_valign(gtk4::Align::Center);
        brush_page.append(&opac_lbl);
        brush_page.append(&opac_scale);

        let flow_adj = gtk4::Adjustment::new(1.0, 0.0, 1.0, 0.01, 0.1, 0.0);
        let flow_scale = gtk4::Scale::new(gtk4::Orientation::Horizontal, Some(&flow_adj));
        flow_scale.set_width_request(80);
        flow_scale.set_valign(gtk4::Align::Center);
        flow_scale.set_digits(2);
        {
            let bf = brush_flow.clone();
            flow_adj.connect_value_changed(move |adj| {
                *bf.borrow_mut() = adj.value();
            });
        }
        let flow_lbl = gtk4::Label::new(Some("Flow:"));
        flow_lbl.set_valign(gtk4::Align::Center);
        brush_page.append(&flow_lbl);
        brush_page.append(&flow_scale);

        stack.add_named(&brush_page, Some("brush"));

        // ---- Crop page ----
        let crop_page = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        crop_page.set_margin_start(4);
        crop_page.set_margin_end(4);
        crop_page.set_margin_top(2);
        crop_page.set_margin_bottom(2);

        let saved_state: Rc<RefCell<Option<EditPipeline>>> = Rc::new(RefCell::new(None));

        for preset in PRESETS {
            let btn = gtk4::Button::with_label(preset.label);
            btn.add_css_class("flat");
            btn.set_valign(gtk4::Align::Center);
            btn.set_has_frame(false);

            let pip = pipeline.clone();
            let doc = document.clone();
            let saved = saved_state.clone();
            let aspect = preset.aspect;

            btn.connect_clicked(move |_| {
                if saved.borrow().is_none() {
                    *saved.borrow_mut() = Some(pip.borrow().clone());
                }

                let d = doc.borrow();
                let img_w = d.canvas_width as f64;
                let img_h = d.canvas_height as f64;
                drop(d);

                let mut p = pip.borrow_mut();

                if let Some((aw, ah)) = aspect {
                    let target_aspect = aw / ah;
                    let img_aspect = img_w / img_h;
                    let (cw, ch) = if target_aspect > img_aspect {
                        (img_w, img_w / target_aspect)
                    } else {
                        (img_h * target_aspect, img_h)
                    };
                    let cx = (img_w - cw) / 2.0;
                    let cy = (img_h - ch) / 2.0;
                    p.crop = Some(CropRect::new(cx, cy, cw, ch));
                } else {
                    p.crop = None;
                }

                p.resize = None;
            });

            crop_page.append(&btn);
        }

        let custom_entry = gtk4::Entry::new();
        custom_entry.set_placeholder_text(Some("W:H"));
        custom_entry.set_width_request(64);
        custom_entry.set_valign(gtk4::Align::Center);
        {
            let pip = pipeline.clone();
            let doc = document.clone();
            let saved = saved_state.clone();
            custom_entry.connect_activate(move |entry| {
                let text = entry.text();
                let text = text.trim();
                if let Some((w_str, h_str)) = text.split_once(':') {
                    if let (Ok(w), Ok(h)) =
                        (w_str.trim().parse::<f64>(), h_str.trim().parse::<f64>())
                    {
                        if saved.borrow().is_none() {
                            *saved.borrow_mut() = Some(pip.borrow().clone());
                        }
                        let d = doc.borrow();
                        let img_w = d.canvas_width as f64;
                        let img_h = d.canvas_height as f64;
                        drop(d);
                        if w <= 0.0 || h <= 0.0 {
                            return;
                        }
                        let aspect = w / h;
                        let img_aspect = img_w / img_h;
                        let (cw, ch) = if aspect > img_aspect {
                            (img_w, img_w / aspect)
                        } else {
                            (img_h * aspect, img_h)
                        };
                        let cx = (img_w - cw) / 2.0;
                        let cy = (img_h - ch) / 2.0;
                        let mut p = pip.borrow_mut();
                        p.crop = Some(CropRect::new(cx, cy, cw, ch));
                        p.resize = None;
                        entry.set_text("");
                    }
                }
            });
        }
        crop_page.append(&custom_entry);

        crop_page.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));

        let apply_btn = gtk4::Button::with_label("Apply");
        apply_btn.add_css_class("suggested-action");
        apply_btn.set_valign(gtk4::Align::Center);
        apply_btn.set_has_frame(false);
        {
            let saved = saved_state.clone();
            apply_btn.connect_clicked(move |_| {
                saved.borrow_mut().take();
            });
        }
        crop_page.append(&apply_btn);

        let cancel_btn = gtk4::Button::with_label("Cancel");
        cancel_btn.set_valign(gtk4::Align::Center);
        cancel_btn.set_has_frame(false);
        {
            let pip = pipeline.clone();
            let saved = saved_state.clone();
            cancel_btn.connect_clicked(move |_| {
                if let Some(state) = saved.borrow_mut().take() {
                    *pip.borrow_mut() = state;
                }
            });
        }
        crop_page.append(&cancel_btn);

        stack.add_named(&crop_page, Some("crop"));

        // ---- Initial state ----
        {
            let current = *active_tool.borrow();
            Self::switch_stack_page(&stack, current);
        }

        // ---- Tool change watch ----
        {
            let active = active_tool.clone();
            let stack_watch = stack.clone();
            use std::cell::Cell;
            let last: Rc<Cell<Option<ToolKind>>> = Rc::new(Cell::new(Some(*active.borrow())));

            glib::timeout_add_local(std::time::Duration::from_millis(80), move || {
                let current = *active.borrow();
                if last.get() != Some(current) {
                    last.set(Some(current));
                    Self::switch_stack_page(&stack_watch, current);
                }
                glib::ControlFlow::Continue
            });
        }

        Self {
            widget,
            stack,
            brush_size,
            brush_hardness,
            brush_opacity,
            brush_flow,
            brush_color,
        }
    }

    fn switch_stack_page(stack: &gtk4::Stack, tool: ToolKind) {
        let name = match tool {
            ToolKind::Brush
            | ToolKind::Eraser
            | ToolKind::Clone
            | ToolKind::Heal
            | ToolKind::Blur
            | ToolKind::Sharpen
            | ToolKind::Smudge => "brush",
            ToolKind::Crop => "crop",
            _ => "move",
        };
        stack.set_visible_child_name(name);
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }

    pub fn set_tool(&self, tool: ToolKind) {
        Self::switch_stack_page(&self.stack, tool);
    }
}
