#![allow(dead_code)]

use adw::prelude::*;

use crate::document::Document;
use crate::image::pipeline::{
    CropRect, EditPipeline, ExportFormat, ExportParams, PresetTarget, ResizeKernel, ResizeMode,
    ResizeTarget, Rotation,
};

use std::cell::RefCell;
use std::rc::Rc;

pub struct QuickEditPanel {
    widget: gtk4::Box,
    document: Rc<RefCell<Document>>,
    pipeline: Rc<RefCell<EditPipeline>>,
    export_params: Rc<RefCell<ExportParams>>,
    on_export: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    export_button: gtk4::Button,
    filename_label: gtk4::Label,
    format_dropdown: gtk4::DropDown,
    quality_adj: gtk4::Adjustment,
}

#[derive(Clone, PartialEq)]
struct OutputPreviewSignature {
    canvas_width: u32,
    canvas_height: u32,
    file_path: Option<String>,
    crop: Option<CropRect>,
    resize: Option<ResizeTarget>,
    rotation: Rotation,
    format: ExportFormat,
}

impl QuickEditPanel {
    pub fn new(
        document: Rc<RefCell<Document>>,
        pipeline: Rc<RefCell<EditPipeline>>,
        export_params: Rc<RefCell<ExportParams>>,
    ) -> Self {
        let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

        let crop_group = Self::build_crop_section(&document, &pipeline);
        main_box.append(&crop_group);

        let resize_group = Self::build_resize_section(&pipeline);
        main_box.append(&resize_group);

        let rotate_group = Self::build_rotation_section(&pipeline);
        main_box.append(&rotate_group);

        let export_inner = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        let filename_label = gtk4::Label::new(Some("image.png"));
        filename_label.add_css_class("dim-label");
        filename_label.set_margin_top(4);
        filename_label.set_margin_bottom(4);
        filename_label.set_margin_start(12);
        filename_label.set_margin_end(12);
        filename_label.set_halign(gtk4::Align::Center);
        filename_label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);

        let (format_dropdown, quality_adj, export_button) =
            Self::build_export_section(&export_params, &export_inner, &filename_label);

        export_inner.append(&filename_label);

        let export_prefs = adw::PreferencesGroup::new();
        export_prefs.set_title("Export");
        export_prefs.add(&export_inner);
        main_box.append(&export_prefs);

        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_child(Some(&main_box));
        scrolled.set_vexpand(true);

        let outer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        outer.append(&scrolled);

        let panel = Self {
            widget: outer,
            document,
            pipeline,
            export_params,
            on_export: Rc::new(RefCell::new(None)),
            export_button,
            filename_label,
            format_dropdown,
            quality_adj,
        };

        panel.update_filename();
        panel.connect_live_refresh();

        panel
    }

    fn connect_live_refresh(&self) {
        let document = self.document.clone();
        let pipeline = self.pipeline.clone();
        let export_params = self.export_params.clone();
        let filename_label = self.filename_label.clone();
        let mut last_signature = Self::output_preview_signature(
            &document.borrow(),
            &pipeline.borrow(),
            &export_params.borrow(),
        );

        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            let next_signature = Self::output_preview_signature(
                &document.borrow(),
                &pipeline.borrow(),
                &export_params.borrow(),
            );
            if next_signature != last_signature {
                last_signature = next_signature;
                let name = Self::output_filename(
                    &document.borrow(),
                    &pipeline.borrow(),
                    &export_params.borrow(),
                );
                filename_label.set_text(&name);
            }
            glib::ControlFlow::Continue
        });
    }

    fn build_crop_section(
        document: &Rc<RefCell<Document>>,
        pipeline: &Rc<RefCell<EditPipeline>>,
    ) -> adw::PreferencesGroup {
        let group = adw::PreferencesGroup::new();
        group.set_title("Crop");

        let flow = gtk4::FlowBox::new();
        flow.set_max_children_per_line(4);
        flow.set_selection_mode(gtk4::SelectionMode::None);
        flow.set_homogeneous(true);
        flow.set_margin_top(8);
        flow.set_margin_bottom(8);
        flow.set_margin_start(8);
        flow.set_margin_end(8);

        for (label, aspect) in [
            ("Free", None),
            ("1:1", Some((1.0, 1.0))),
            ("4:3", Some((4.0, 3.0))),
            ("16:9", Some((16.0, 9.0))),
        ] {
            let btn = gtk4::ToggleButton::with_label(label);
            btn.add_css_class("flat");
            let doc = document.clone();
            let pip = pipeline.clone();
            btn.connect_clicked(move |_| {
                let mut p = pip.borrow_mut();
                if let Some((w, h)) = aspect {
                    p.crop = Self::crop_for_aspect(&doc.borrow(), w / h);
                } else {
                    p.crop = None;
                }
            });
            flow.append(&btn);
        }

        group.add(&flow);

        group.add(&flow);

        let ratio_row = adw::ActionRow::new();
        ratio_row.set_title("Custom Aspect Ratio (W:H)");
        
        let ratio_entry = gtk4::Entry::new();
        ratio_entry.set_valign(gtk4::Align::Center);
        ratio_row.add_suffix(&ratio_entry);

        {
            let doc = document.clone();
            let pip = pipeline.clone();
            ratio_entry.connect_activate(move |entry| {
                let text = entry.text();
                let text = text.trim();
                let Some((w_str, h_str)) = text.split_once(':') else {
                    return;
                };
                let (Ok(w), Ok(h)) = (w_str.trim().parse::<f64>(), h_str.trim().parse::<f64>())
                else {
                    return;
                };
                if w <= 0.0 || h <= 0.0 {
                    return;
                }

                pip.borrow_mut().crop = Self::crop_for_aspect(&doc.borrow(), w / h);
                entry.set_text("");
            });
        }

        group.add(&ratio_row);
        group
    }

    fn build_resize_section(pipeline: &Rc<RefCell<EditPipeline>>) -> adw::PreferencesGroup {
        let group = adw::PreferencesGroup::new();
        group.set_title("Resize");

        let flow = gtk4::FlowBox::new();
        flow.set_max_children_per_line(3);
        flow.set_selection_mode(gtk4::SelectionMode::None);
        flow.set_homogeneous(true);
        flow.set_margin_top(8);
        flow.set_margin_bottom(8);
        flow.set_margin_start(8);
        flow.set_margin_end(8);

        for (w, h, label) in PresetTarget::presets() {
            let btn = gtk4::ToggleButton::with_label(*label);
            btn.add_css_class("flat");
            let pip = pipeline.clone();
            let w = *w;
            let h = *h;
            btn.connect_clicked(move |_| {
                let mut p = pip.borrow_mut();
                p.resize = Some(ResizeTarget::new(w, h));
                p.preset = PresetTarget::Custom(w, h);
            });
            flow.append(&btn);
        }

        {
            let btn = gtk4::ToggleButton::with_label("Original");
            btn.add_css_class("flat");
            let pip = pipeline.clone();
            btn.connect_clicked(move |_| {
                let mut p = pip.borrow_mut();
                p.resize = None;
                p.preset = PresetTarget::Original;
            });
            flow.append(&btn);
        }

        group.add(&flow);

        let custom_resize_row = adw::ActionRow::new();
        custom_resize_row.set_title("Custom Size");
        
        let custom_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        custom_box.set_valign(gtk4::Align::Center);

        let w_entry = gtk4::Entry::new();
        w_entry.set_placeholder_text(Some("W"));
        w_entry.set_width_chars(5);
        let x_label = gtk4::Label::new(Some("×"));
        let h_entry = gtk4::Entry::new();
        h_entry.set_placeholder_text(Some("H"));
        h_entry.set_width_chars(5);

        custom_box.append(&w_entry);
        custom_box.append(&x_label);
        custom_box.append(&h_entry);
        custom_resize_row.add_suffix(&custom_box);

        let pip = pipeline.clone();
        let w_e = w_entry.clone();
        let h_e = h_entry.clone();
        let parse_custom = move || {
            let w: u32 = w_e.text().parse().unwrap_or(0);
            let h: u32 = h_e.text().parse().unwrap_or(0);
            if w > 0 && h > 0 {
                let mut p = pip.borrow_mut();
                p.resize = Some(ResizeTarget::new(w, h));
                p.preset = PresetTarget::Custom(w, h);
            }
        };

        w_entry.connect_activate(glib::clone!(#[strong] parse_custom, move |_| parse_custom()));
        h_entry.connect_activate(glib::clone!(#[strong] parse_custom, move |_| parse_custom()));

        group.add(&custom_resize_row);

        let mode_dropdown = {
            let model = gtk4::StringList::new(&["Fill crop", "Fit inside", "Stretch"]);
            let dd = gtk4::DropDown::builder()
                .model(&model)
                .valign(gtk4::Align::Center)
                .build();
            dd.set_selected(0);
            let pip = pipeline.clone();
            dd.connect_selected_notify(move |dd| {
                let idx = dd.selected();
                let mut p = pip.borrow_mut();
                if let Some(rt) = &mut p.resize {
                    rt.mode = match idx {
                        0 => ResizeMode::FillCrop,
                        1 => ResizeMode::Fit,
                        _ => ResizeMode::Stretch,
                    };
                }
            });
            dd
        };

        let mode_row = adw::ActionRow::new();
        mode_row.set_title("Mode");
        mode_row.add_suffix(&mode_dropdown);
        group.add(&mode_row);

        let kernel_dropdown = {
            let strings: Vec<&str> = ResizeKernel::all().iter().map(|k| k.as_str()).collect();
            let model = gtk4::StringList::new(&strings);
            let dd = gtk4::DropDown::builder()
                .model(&model)
                .valign(gtk4::Align::Center)
                .build();
            let kernels = ResizeKernel::all().to_vec();
            dd.set_selected(
                kernels
                    .iter()
                    .position(|k| *k == ResizeKernel::Lanczos3)
                    .unwrap_or(0) as u32,
            );
            let pip = pipeline.clone();
            dd.connect_selected_notify(move |dd| {
                let idx = dd.selected() as usize;
                let all = ResizeKernel::all();
                if let Some(k) = all.get(idx) {
                    pip.borrow_mut().kernel = *k;
                }
            });
            dd
        };

        let kernel_row = adw::ActionRow::new();
        kernel_row.set_title("Kernel");
        kernel_row.add_suffix(&kernel_dropdown);
        group.add(&kernel_row);

        group
    }

    fn crop_for_aspect(document: &Document, aspect: f64) -> Option<CropRect> {
        if aspect <= 0.0 || document.canvas_width == 0 || document.canvas_height == 0 {
            return None;
        }

        let canvas_w = document.canvas_width as f64;
        let canvas_h = document.canvas_height as f64;
        let canvas_aspect = canvas_w / canvas_h;
        let (crop_w, crop_h) = if aspect > canvas_aspect {
            (canvas_w, canvas_w / aspect)
        } else {
            (canvas_h * aspect, canvas_h)
        };
        let x = (canvas_w - crop_w) / 2.0;
        let y = (canvas_h - crop_h) / 2.0;
        Some(CropRect::new(x, y, crop_w, crop_h))
    }

    fn build_rotation_section(pipeline: &Rc<RefCell<EditPipeline>>) -> adw::PreferencesGroup {
        let group = adw::PreferencesGroup::new();
        group.set_title("Rotation");

        let rotate_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        rotate_box.set_margin_top(8);
        rotate_box.set_margin_bottom(8);
        rotate_box.set_margin_start(12);
        rotate_box.set_margin_end(12);
        rotate_box.set_homogeneous(true);

        let rotations = [
            ("0", Rotation::None),
            ("90", Rotation::Clockwise90),
            ("180", Rotation::Clockwise180),
            ("270", Rotation::Clockwise270),
        ];

        let mut group_leader: Option<gtk4::ToggleButton> = None;

        for (label, angle) in &rotations {
            let btn = gtk4::ToggleButton::with_label(label);
            btn.add_css_class("flat");
            if let Some(ref leader) = group_leader {
                btn.set_group(Some(leader));
            } else {
                group_leader = Some(btn.clone());
            }
            if *angle == Rotation::None {
                btn.set_active(true);
            }
            let pip = pipeline.clone();
            let a = *angle;
            btn.connect_clicked(move |_| {
                pip.borrow_mut().rotation = a;
            });
            rotate_box.append(&btn);
        }

        group.add(&rotate_box);
        group
    }

    fn build_export_section(
        export_params: &Rc<RefCell<ExportParams>>,
        outer_box: &gtk4::Box,
        filename_label: &gtk4::Label,
    ) -> (gtk4::DropDown, gtk4::Adjustment, gtk4::Button) {
        let format_dropdown = {
            let format_names: Vec<&str> =
                [ExportFormat::Png, ExportFormat::Jpeg, ExportFormat::WebP]
                    .iter()
                    .map(|f| f.as_str())
                    .collect();
            let model = gtk4::StringList::new(&format_names);
            let dd = gtk4::DropDown::builder()
                .model(&model)
                .valign(gtk4::Align::Center)
                .build();
            dd.set_selected(0);
            let exp = export_params.clone();
            let fl = filename_label.clone();
            dd.connect_selected_notify(move |dd| {
                let idx = dd.selected();
                let mut p = exp.borrow_mut();
                p.format = match idx {
                    1 => ExportFormat::Jpeg,
                    2 => ExportFormat::WebP,
                    _ => ExportFormat::Png,
                };
                drop(p);
                let exp = exp.borrow();
                let ext = match exp.format {
                    ExportFormat::Png => "png",
                    ExportFormat::Jpeg => "jpg",
                    ExportFormat::WebP => "webp",
                };
                let current = fl.text();
                let new = Self::replace_extension(&current, ext);
                fl.set_text(&new);
            });
            dd
        };

        let fmt_row = adw::ActionRow::new();
        fmt_row.set_title("Format");
        fmt_row.add_suffix(&format_dropdown);
        outer_box.append(&fmt_row);

        let quality_adj = gtk4::Adjustment::new(90.0, 1.0, 100.0, 1.0, 10.0, 0.0);
        let quality_scale = gtk4::Scale::new(gtk4::Orientation::Horizontal, Some(&quality_adj));
        quality_scale.set_hexpand(true);
        quality_scale.set_digits(0);
        quality_scale.set_value_pos(gtk4::PositionType::Right);
        quality_scale.set_width_request(120);
        quality_scale.set_valign(gtk4::Align::Center);

        let qual_row = adw::ActionRow::new();
        qual_row.set_title("Quality");
        qual_row.add_suffix(&quality_scale);
        outer_box.append(&qual_row);

        let exp = export_params.clone();
        quality_adj.connect_value_changed(move |adj| {
            exp.borrow_mut().quality = adj.value() as u8;
        });

        let export_btn = gtk4::Button::with_label("Export");
        export_btn.add_css_class("suggested-action");
        export_btn.set_margin_top(12);
        export_btn.set_margin_start(12);
        export_btn.set_margin_end(12);
        export_btn.set_halign(gtk4::Align::Fill);
        outer_box.append(&export_btn);

        (format_dropdown, quality_adj, export_btn)
    }

    fn labeled_row(label: &str, widget: &impl IsA<gtk4::Widget>) -> gtk4::Box {
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        row.set_margin_start(12);
        row.set_margin_end(12);
        row.set_margin_top(4);
        row.set_margin_bottom(4);

        let lbl = gtk4::Label::new(Some(label));
        lbl.set_valign(gtk4::Align::Center);
        lbl.set_halign(gtk4::Align::Start);
        row.append(&lbl);

        let widget_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        widget_box.set_hexpand(true);
        widget_box.set_halign(gtk4::Align::End);
        widget_box.append(widget);
        row.append(&widget_box);

        row
    }

    pub fn set_on_export<F: Fn() + 'static>(&self, f: F) {
        let on_export = self.on_export.clone();
        *on_export.borrow_mut() = Some(Box::new(f));

        self.export_button.connect_clicked(move |_| {
            if let Some(ref cb) = *on_export.borrow() {
                cb();
            }
        });
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }

    pub fn refresh(&self) {
        self.update_filename();
    }

    fn update_filename(&self) {
        let name = Self::output_filename(
            &self.document.borrow(),
            &self.pipeline.borrow(),
            &self.export_params.borrow(),
        );
        self.filename_label.set_text(&name);
    }

    fn output_preview_signature(
        document: &Document,
        pipeline: &EditPipeline,
        export_params: &ExportParams,
    ) -> OutputPreviewSignature {
        OutputPreviewSignature {
            canvas_width: document.canvas_width,
            canvas_height: document.canvas_height,
            file_path: document.file_path.clone(),
            crop: pipeline.crop,
            resize: pipeline.resize,
            rotation: pipeline.rotation,
            format: export_params.format,
        }
    }

    fn output_filename(
        document: &Document,
        pipeline: &EditPipeline,
        export_params: &ExportParams,
    ) -> String {
        let (base_w, base_h) = match pipeline.rotation {
            Rotation::Clockwise90 | Rotation::Clockwise270 => {
                (document.canvas_height, document.canvas_width)
            }
            Rotation::None | Rotation::Clockwise180 => {
                (document.canvas_width, document.canvas_height)
            }
        };

        let (out_w, out_h) = if let Some(ref rt) = pipeline.resize {
            (rt.width, rt.height)
        } else if let Some(ref cr) = pipeline.crop {
            (cr.width as u32, cr.height as u32)
        } else {
            (base_w, base_h)
        };

        let base = document
            .file_path
            .as_ref()
            .and_then(|p| std::path::Path::new(p).file_stem().and_then(|s| s.to_str()))
            .unwrap_or("image");

        let ext = match export_params.format {
            ExportFormat::Png => "png",
            ExportFormat::Jpeg => "jpg",
            ExportFormat::WebP => "webp",
        };

        format!("{}_{}x{}.{}", base, out_w, out_h, ext)
    }

    fn replace_extension(filename: &str, new_ext: &str) -> String {
        if let Some(pos) = filename.rfind('.') {
            format!("{}.{}", &filename[..pos], new_ext)
        } else {
            format!("{}.{}", filename, new_ext)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_for_aspect_centers_a_wider_crop_without_resizing() {
        let doc = Document::new(400, 300);

        let crop = QuickEditPanel::crop_for_aspect(&doc, 16.0 / 9.0).unwrap();

        assert_eq!(crop.x, 0.0);
        assert_eq!(crop.width, 400.0);
        assert_eq!(crop.height, 225.0);
        assert_eq!(crop.y, 37.5);
    }

    #[test]
    fn output_filename_reflects_rotation_dimensions() {
        let doc = Document::new(400, 300);
        let mut pipeline = EditPipeline::default();
        pipeline.rotation = Rotation::Clockwise90;

        let filename = QuickEditPanel::output_filename(&doc, &pipeline, &ExportParams::default());

        assert_eq!(filename, "image_300x400.png");
    }
}
