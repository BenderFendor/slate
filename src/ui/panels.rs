#![allow(dead_code)]

use adw::prelude::*;

use crate::document::{BlendMode, Document, LayerId, LayerKind};
use crate::tile::snapshot::{build_render_frame, flatten_frame_bgra};

use std::cell::RefCell;
use std::rc::Rc;

const ROW_HEIGHT: i32 = 44;
const THUMB_SIZE: i32 = 36;
const MASK_THUMB_SIZE: i32 = 24;

#[derive(Clone, PartialEq, Eq)]
struct LayerPanelFingerprint {
    active_layer: Option<LayerId>,
    layers: Vec<LayerUiFingerprint>,
}

#[derive(Clone, PartialEq, Eq)]
struct LayerUiFingerprint {
    id: LayerId,
    name: String,
    visible: bool,
    locked: bool,
    opacity_bits: u32,
    blend_mode: BlendMode,
    kind: &'static str,
    mask: Option<MaskUiFingerprint>,
}

#[derive(Clone, PartialEq, Eq)]
struct MaskUiFingerprint {
    visible: bool,
    linked: bool,
    enabled: bool,
    editing: bool,
    show_on_canvas: bool,
}

pub struct RightPanels {
    widget: gtk4::Box,
    document: Rc<RefCell<Document>>,
    layers_list: gtk4::ListBox,
    properties_box: gtk4::Box,
    history_list: gtk4::ListBox,
    navigator_preview: gtk4::DrawingArea,
    selected_layer: Rc<RefCell<Option<LayerId>>>,
    layer_ids: Rc<RefCell<Vec<LayerId>>>,
    blend_model: gtk4::DropDown,
    opacity_scale: gtk4::Scale,
    opacity_adj: gtk4::Adjustment,
    mask_button: gtk4::Button,
    delete_button: gtk4::Button,
    refreshing: Rc<RefCell<bool>>,
}

impl RightPanels {
    pub fn new(document: Rc<RefCell<Document>>, brush_color: Rc<RefCell<[f32; 4]>>) -> Self {
        let layer_ids: Rc<RefCell<Vec<LayerId>>> = Rc::new(RefCell::new(Vec::new()));
        let selected_layer: Rc<RefCell<Option<LayerId>>> = Rc::new(RefCell::new(None));
        let refreshing: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

        let widget = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

        let blend_strings: Vec<&str> = [
            BlendMode::Normal,
            BlendMode::Multiply,
            BlendMode::Screen,
            BlendMode::Overlay,
            BlendMode::Darken,
            BlendMode::Lighten,
            BlendMode::ColorDodge,
            BlendMode::ColorBurn,
            BlendMode::HardLight,
            BlendMode::SoftLight,
            BlendMode::Difference,
            BlendMode::Exclusion,
            BlendMode::Hue,
            BlendMode::Saturation,
            BlendMode::Color,
            BlendMode::Luminosity,
        ]
        .iter()
        .map(|m| m.as_str())
        .collect();

        let blend_model_obj = gtk4::StringList::new(&blend_strings);
        let blend_dropdown = gtk4::DropDown::builder().model(&blend_model_obj).build();
        blend_dropdown.set_tooltip_text(Some("Blend Mode"));
        blend_dropdown.set_selected(0);

        let opacity_adj = gtk4::Adjustment::new(100.0, 0.0, 100.0, 1.0, 10.0, 0.0);
        let opacity_scale = gtk4::Scale::new(gtk4::Orientation::Horizontal, Some(&opacity_adj));
        opacity_scale.set_digits(0);
        opacity_scale.set_hexpand(true);
        opacity_scale.set_size_request(60, -1);

        let layers_list = gtk4::ListBox::new();
        layers_list.set_hexpand(true);
        layers_list.set_selection_mode(gtk4::SelectionMode::Single);
        layers_list.add_css_class("navigation-sidebar");

        let history_list = gtk4::ListBox::new();
        history_list.set_hexpand(true);

        let properties_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        properties_box.set_margin_start(12);
        properties_box.set_margin_end(12);
        properties_box.set_margin_top(12);

        let scrolled_properties = gtk4::ScrolledWindow::new();
        scrolled_properties.set_hexpand(true);
        scrolled_properties.set_vexpand(true);
        scrolled_properties.set_child(Some(&properties_box));

        let mask_button = gtk4::Button::from_icon_name("view-cover-symbolic");
        mask_button.set_tooltip_text(Some("Add Mask"));
        mask_button.add_css_class("flat");

        let delete_button = gtk4::Button::from_icon_name("user-trash-symbolic");
        delete_button.set_tooltip_text(Some("Delete Layer"));
        delete_button.add_css_class("flat");
        delete_button.add_css_class("destructive-action");

        let layers_page = {
            let outer_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

            // --- Blend mode / opacity bar ---
            let top_bar = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
            top_bar.set_margin_start(6);
            top_bar.set_margin_end(6);
            top_bar.set_margin_top(4);
            top_bar.set_margin_bottom(4);

            let blend_label = gtk4::Label::new(Some("Blend:"));
            blend_label.add_css_class("dim-label");
            blend_label.set_valign(gtk4::Align::Center);
            top_bar.append(&blend_label);
            top_bar.append(&blend_dropdown);

            let opacity_label = gtk4::Label::new(Some("Opacity:"));
            opacity_label.add_css_class("dim-label");
            opacity_label.set_valign(gtk4::Align::Center);
            opacity_label.set_margin_start(6);
            top_bar.append(&opacity_label);
            top_bar.append(&opacity_scale);

            let opacity_pct = gtk4::Label::new(Some("100%"));
            opacity_pct.set_width_chars(4);
            opacity_pct.set_valign(gtk4::Align::Center);
            opacity_pct.add_css_class("dim-label");
            top_bar.append(&opacity_pct);
            {
                let lbl = opacity_pct.clone();
                opacity_adj.connect_value_changed(move |adj| {
                    lbl.set_text(&format!("{:.0}%", adj.value()));
                });
            }

            outer_box.append(&top_bar);

            // --- Separator ---
            let sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
            outer_box.append(&sep);

            // --- Layer list ---
            let scrolled_layers = gtk4::ScrolledWindow::new();
            scrolled_layers.set_hexpand(true);
            scrolled_layers.set_vexpand(true);
            scrolled_layers.set_child(Some(&layers_list));
            outer_box.append(&scrolled_layers);

            // --- Bottom action bar ---
            let button_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
            button_row.set_margin_start(4);
            button_row.set_margin_end(4);
            button_row.set_margin_top(4);
            button_row.set_margin_bottom(4);

            let add_btn = gtk4::Button::from_icon_name("list-add-symbolic");
            add_btn.set_tooltip_text(Some("Add Layer"));
            add_btn.add_css_class("flat");
            {
                add_btn.connect_clicked(move |btn| {
                    if let Some(window) = btn.root().and_downcast::<gtk4::Window>() {
                        if let Err(error) = window.activate_action("win.new-layer", None) {
                            log::warn!("Add layer action failed: {}", error);
                        }
                    }
                });
            }
            button_row.append(&add_btn);

            let mask_btn = mask_button.clone();
            mask_btn.connect_clicked(move |btn| {
                if let Some(window) = btn.root().and_downcast::<gtk4::Window>() {
                    if let Err(error) = window.activate_action("win.add-mask", None) {
                        log::warn!("Add mask action failed: {}", error);
                    }
                }
            });
            button_row.append(&mask_btn);

            let trash_btn = delete_button.clone();
            {
                trash_btn.connect_clicked(move |btn| {
                    if let Some(window) = btn.root().and_downcast::<gtk4::Window>() {
                        if let Err(error) = window.activate_action("win.delete-layer", None) {
                            log::warn!("Delete layer action failed: {}", error);
                        }
                    }
                });
            }
            button_row.append(&trash_btn);

            outer_box.append(&button_row);

            outer_box
        };

        let history_page = {
            let scrolled_history = gtk4::ScrolledWindow::new();
            scrolled_history.set_hexpand(true);
            scrolled_history.set_vexpand(true);
            scrolled_history.set_child(Some(&history_list));
            scrolled_history
        };

        let navigator_preview = gtk4::DrawingArea::new();
        let navigator_page = {
            let nav_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
            nav_box.set_margin_start(12);
            nav_box.set_margin_end(12);
            nav_box.set_margin_top(12);
            nav_box.set_margin_bottom(12);
            navigator_preview.set_content_width(220);
            navigator_preview.set_content_height(140);
            navigator_preview.set_vexpand(false);
            navigator_preview.add_css_class("layer-thumbnail");
            {
                let doc = document.clone();
                navigator_preview.set_draw_func(move |_area, cr, width, height| {
                    draw_navigator_preview(cr, width, height, &doc.borrow());
                });
            }
            nav_box.append(&navigator_preview);
            nav_box
        };

        let navigator_group = adw::PreferencesGroup::builder()
            .title("Navigator")
            .build();
        navigator_group.add(&navigator_page);

        let props_group = adw::PreferencesGroup::builder()
            .title("Properties")
            .build();
        props_group.add(&scrolled_properties);

        let layers_group = adw::PreferencesGroup::builder()
            .title("Layers")
            .build();
        layers_group.add(&layers_page);
        layers_group.set_vexpand(true);

        let history_group = adw::PreferencesGroup::builder()
            .title("History")
            .build();
        history_group.add(&history_page);

        let scrolled_panels = gtk4::ScrolledWindow::new();
        scrolled_panels.set_hscrollbar_policy(gtk4::PolicyType::Never);
        let panels_vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        panels_vbox.set_margin_start(6);
        panels_vbox.set_margin_end(6);
        panels_vbox.set_margin_top(6);
        panels_vbox.set_margin_bottom(6);
        
        panels_vbox.append(&navigator_group);
        panels_vbox.append(&props_group);
        
        let swatches_group = adw::PreferencesGroup::builder()
            .title("Swatches")
            .build();
        let swatch_grid = gtk4::FlowBox::new();
        swatch_grid.set_max_children_per_line(8);
        swatch_grid.set_selection_mode(gtk4::SelectionMode::None);
        swatch_grid.set_margin_top(8);
        swatch_grid.set_margin_bottom(8);
        swatch_grid.set_margin_start(8);
        swatch_grid.set_margin_end(8);
        
        let common_colors = [
            [0.0, 0.0, 0.0, 1.0], // Black
            [1.0, 1.0, 1.0, 1.0], // White
            [0.5, 0.5, 0.5, 1.0], // Gray
            [1.0, 0.0, 0.0, 1.0], // Red
            [0.0, 1.0, 0.0, 1.0], // Green
            [0.0, 0.0, 1.0, 1.0], // Blue
            [1.0, 1.0, 0.0, 1.0], // Yellow
            [1.0, 0.0, 1.0, 1.0], // Magenta
            [0.0, 1.0, 1.0, 1.0], // Cyan
            [1.0, 0.5, 0.0, 1.0], // Orange
            [0.5, 0.0, 1.0, 1.0], // Purple
            [0.0, 0.5, 1.0, 1.0], // Sky Blue
        ];
        
        for c in common_colors {
            let btn = gtk4::Button::new();
            btn.add_css_class("flat");
            btn.set_size_request(24, 24);
            let draw = gtk4::DrawingArea::new();
            draw.set_draw_func(move |_area, cr, w, h| {
                cr.set_source_rgba(c[0] as f64, c[1] as f64, c[2] as f64, c[3] as f64);
                cr.rectangle(0.0, 0.0, w as f64, h as f64);
                cr.fill().ok();
            });
            btn.set_child(Some(&draw));
            {
                let color = brush_color.clone();
                btn.connect_clicked(move |_| {
                    *color.borrow_mut() = c;
                });
            }
            swatch_grid.insert(&btn, -1);
        }
        let swatch_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        swatch_box.append(&swatch_grid);
        swatches_group.add(&swatch_box);
        panels_vbox.append(&swatches_group);

        panels_vbox.append(&layers_group);
        panels_vbox.append(&history_group);
        
        scrolled_panels.set_child(Some(&panels_vbox));
        widget.append(&scrolled_panels);

        let slf = Self {
            widget,
            document: document.clone(),
            layers_list,
            properties_box,
            history_list,
            navigator_preview,
            selected_layer,
            layer_ids: layer_ids.clone(),
            blend_model: blend_dropdown,
            opacity_scale,
            opacity_adj,
            mask_button,
            delete_button,
            refreshing: refreshing.clone(),
        };

        slf.connect_signals(document.clone(), layer_ids, refreshing);

        slf.refresh_layers_from_internal();

        // Start watcher
        {
            let doc = document.clone();
            let p = Rc::new(slf.clone_weak());
            let mut last_rev = doc.borrow().revision;
            let mut last_layer_fingerprint = layer_panel_fingerprint(&doc.borrow());
            let mut last_history = doc.borrow().undo_stack.history_names();
            glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                if let Some(panels) = p.upgrade() {
                    let doc_ref = doc.borrow();
                    let current_rev = doc_ref.revision;
                    if current_rev != last_rev {
                        last_rev = current_rev;
                        let current_layer_fingerprint = layer_panel_fingerprint(&doc_ref);
                        if current_layer_fingerprint != last_layer_fingerprint {
                            last_layer_fingerprint = current_layer_fingerprint;
                            drop(doc_ref);
                            panels.refresh_layers_from_internal();
                        } else {
                            let current_history = doc_ref.undo_stack.history_names();
                            drop(doc_ref);
                            if current_history != last_history {
                                last_history = current_history;
                                panels.populate_history();
                            }
                        }
                        panels.navigator_preview.queue_draw();
                    } else {
                        drop(doc_ref);
                    }
                    glib::ControlFlow::Continue
                } else {
                    glib::ControlFlow::Break
                }
            });
        }

        slf
    }

    fn clone_weak(&self) -> RightPanelsWeak {
        RightPanelsWeak {
            layers_list: self.layers_list.downgrade(),
            properties_box: self.properties_box.downgrade(),
            history_list: self.history_list.downgrade(),
            navigator_preview: self.navigator_preview.downgrade(),
            document: self.document.clone(),
            layer_ids: self.layer_ids.clone(),
            blend_model: self.blend_model.downgrade(),
            opacity_scale: self.opacity_scale.downgrade(),
            opacity_adj: self.opacity_adj.downgrade(),
            mask_button: self.mask_button.downgrade(),
            delete_button: self.delete_button.downgrade(),
            selected_layer: self.selected_layer.clone(),
            refreshing: self.refreshing.clone(),
        }
    }

    fn connect_signals(
        &self,
        document: Rc<RefCell<Document>>,
        layer_ids: Rc<RefCell<Vec<LayerId>>>,
        refreshing: Rc<RefCell<bool>>,
    ) {
        {
            let doc = document.clone();
            let layer_ids_inner = layer_ids.clone();
            let refreshing = refreshing.clone();
            let selected_layer = self.selected_layer.clone();
            let panels = self.clone_weak();
            self.layers_list.connect_row_selected(move |_list, row| {
                if *refreshing.borrow() {
                    return;
                }
                let selected = if let Some(row) = row {
                    let idx = row.index() as usize;
                    let ids = layer_ids_inner.borrow();
                    if let Some(&id) = ids.get(idx) {
                        doc.borrow_mut().select_layer(Some(id));
                        Some(id)
                    } else {
                        doc.borrow_mut().select_layer(None);
                        None
                    }
                } else {
                    doc.borrow_mut().select_layer(None);
                    None
                };

                *selected_layer.borrow_mut() = selected;
                if let Some(panels) = panels.upgrade() {
                    let doc = doc.borrow();
                    panels.refresh_properties(&doc, selected);
                }
            });
        }

        {
            let doc = document.clone();
            let blend_dd = self.blend_model.clone();
            blend_dd.connect_selected_notify(move |dd| {
                let idx = dd.selected() as usize;
                let modes = [
                    BlendMode::Normal,
                    BlendMode::Multiply,
                    BlendMode::Screen,
                    BlendMode::Overlay,
                    BlendMode::Darken,
                    BlendMode::Lighten,
                    BlendMode::ColorDodge,
                    BlendMode::ColorBurn,
                    BlendMode::HardLight,
                    BlendMode::SoftLight,
                    BlendMode::Difference,
                    BlendMode::Exclusion,
                    BlendMode::Hue,
                    BlendMode::Saturation,
                    BlendMode::Color,
                    BlendMode::Luminosity,
                ];
                if let Some(mode) = modes.get(idx) {
                    let id = doc.borrow().active_layer_id;
                    if let Some(id) = id {
                        if let Some(layer) = doc.borrow_mut().layer_mut(id) {
                            layer.blend_mode = *mode;
                        }
                    }
                }
            });
        }

        {
            let doc = document.clone();
            self.opacity_adj.connect_value_changed(move |adj| {
                let id = doc.borrow().active_layer_id;
                if let Some(id) = id {
                    if let Some(layer) = doc.borrow_mut().layer_mut(id) {
                        layer.opacity = adj.value() as f32 / 100.0;
                    }
                }
            });
        }
    }

    fn populate_history(&self) {
        while let Some(child) = self.history_list.first_child() {
            self.history_list.remove(&child);
        }

        let doc = self.document.borrow();
        let mut entries: Vec<String> = Vec::new();
        if doc.file_path.is_some() {
            entries.push("Open".to_string());
        }
        entries.extend(
            doc.undo_stack
                .history_names()
                .into_iter()
                .map(ToString::to_string),
        );
        drop(doc);

        if entries.is_empty() {
            let row = gtk4::ListBoxRow::new();
            let label = gtk4::Label::new(Some("No edits yet"));
            label.add_css_class("dim-label");
            label.set_margin_top(8);
            label.set_margin_bottom(8);
            label.set_margin_start(12);
            label.set_margin_end(12);
            label.set_halign(gtk4::Align::Start);
            row.set_child(Some(&label));
            self.history_list.append(&row);
            return;
        }

        for entry in entries {
            let row = gtk4::ListBoxRow::new();
            let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            hbox.set_margin_top(4);
            hbox.set_margin_bottom(4);
            hbox.set_margin_start(12);
            hbox.set_margin_end(12);

            let label = gtk4::Label::new(Some(&entry));
            label.set_halign(gtk4::Align::Start);
            label.set_hexpand(true);
            hbox.append(&label);

            row.set_child(Some(&hbox));
            self.history_list.append(&row);
        }
    }

    fn refresh_layers_from_internal(&self) {
        *self.refreshing.borrow_mut() = true;
        let doc = self.document.borrow();
        let sel = doc.active_layer_id;
        self.refresh_layers_impl(&doc, sel);
        drop(doc);
        self.populate_history();
        *self.refreshing.borrow_mut() = false;
    }

    fn refresh_layers_impl(&self, document: &Document, selected: Option<LayerId>) {
        while let Some(child) = self.layers_list.first_child() {
            self.layers_list.remove(&child);
        }
        let mut ids = self.layer_ids.borrow_mut();
        ids.clear();

        let has_active_layer = selected.is_some();
        self.blend_model.set_sensitive(has_active_layer);
        self.opacity_scale.set_sensitive(has_active_layer);
        self.delete_button.set_sensitive(has_active_layer);
        self.mask_button.set_sensitive(
            selected
                .and_then(|id| document.layer(id))
                .is_some_and(|layer| layer.mask.is_none()),
        );

        if document.layers.is_empty() {
            let row = gtk4::ListBoxRow::new();
            row.set_selectable(false);

            let label = gtk4::Label::new(Some("No layers"));
            label.set_margin_top(10);
            label.set_margin_bottom(10);
            label.add_css_class("dim-label");
            row.set_child(Some(&label));

            self.layers_list.append(&row);
        }

        for layer in document.layers.iter().rev() {
            let row = gtk4::ListBoxRow::new();
            row.set_size_request(-1, ROW_HEIGHT);

            let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
            hbox.set_margin_top(4);
            hbox.set_margin_bottom(4);
            hbox.set_margin_start(4);
            hbox.set_margin_end(4);
            hbox.set_valign(gtk4::Align::Center);

            // --- Visibility toggle ---
            let vis_btn = gtk4::CheckButton::new();
            vis_btn.set_active(layer.visible);
            vis_btn.set_tooltip_text(Some("Toggle visibility"));
            vis_btn.set_valign(gtk4::Align::Center);
            vis_btn.add_css_class("flat");
            {
                let doc = self.document.clone();
                let lid = layer.id;
                vis_btn.connect_toggled(move |btn| {
                    if let Some(l) = doc.borrow_mut().layer_mut(lid) {
                        l.visible = btn.is_active();
                    }
                });
            }
            hbox.append(&vis_btn);

            // --- Thumbnail DrawingArea ---
            let thumb = gtk4::DrawingArea::new();
            thumb.set_size_request(THUMB_SIZE, THUMB_SIZE);
            thumb.set_valign(gtk4::Align::Center);
            thumb.add_css_class("layer-thumbnail");

            let raster_data: Option<(u32, u32, Vec<u8>)> = match &layer.kind {
                LayerKind::Raster(r) => Some((r.width, r.height, r.data.clone())),
                _ => None,
            };
            let fill_color: Option<[f32; 4]> = match &layer.kind {
                LayerKind::Fill(f) => Some(f.color),
                _ => None,
            };
            let layer_kind_for_thumb = layer.kind.clone();

            thumb.set_draw_func(move |_area, cr, width, height| {
                draw_layer_thumbnail(
                    cr,
                    width,
                    height,
                    &layer_kind_for_thumb,
                    raster_data.as_ref(),
                    fill_color.as_ref(),
                );
            });
            hbox.append(&thumb);

            // --- Name + kind label ---
            let name_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
            name_box.set_valign(gtk4::Align::Center);
            name_box.set_hexpand(true);

            let name_label = gtk4::Label::new(Some(&layer.name));
            name_label.set_halign(gtk4::Align::Start);
            name_label.set_hexpand(true);
            name_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            name_label.add_css_class("heading");
            name_box.append(&name_label);

            let kind_str = match &layer.kind {
                LayerKind::Raster(_) => "Raster",
                LayerKind::Text(_) => "Text",
                LayerKind::Fill(_) => "Fill",
                LayerKind::Group(_) => "Group",
                LayerKind::Adjustment(_) => "Adjustment",
            };
            let kind_label = gtk4::Label::new(Some(kind_str));
            kind_label.set_halign(gtk4::Align::Start);
            kind_label.add_css_class("dim-label");
            kind_label.add_css_class("caption");
            name_box.append(&kind_label);

            // --- Double-click to rename ---
            let doc_rename = self.document.clone();
            let parent_widget = self.widget.clone();
            let lid_rename = layer.id;
            {
                let gesture = gtk4::GestureClick::new();
                gesture.set_button(1);
                gesture.connect_pressed(move |gesture, n_press, _x, _y| {
                    if n_press == 2 {
                        let lid = lid_rename;
                        let name = {
                            let d = doc_rename.borrow();
                            d.layer(lid).map(|l| l.name.clone()).unwrap_or_default()
                        };
                        let transient = parent_widget.root().and_downcast::<gtk4::Window>();
                        let entry = gtk4::Entry::new();
                        entry.set_text(&name);
                        entry.set_activates_default(true);
                        entry.set_margin_top(12);
                        entry.set_margin_bottom(12);
                        entry.set_margin_start(12);
                        entry.set_margin_end(12);

                        let dialog = gtk4::Window::new();
                        dialog.set_title(Some("Rename Layer"));
                        dialog.set_modal(true);
                        dialog.set_default_size(320, -1);
                        if let Some(ref win) = transient {
                            dialog.set_transient_for(Some(win));
                        }

                        let cancel_btn = gtk4::Button::with_label("Cancel");
                        let rename_btn = gtk4::Button::with_label("Rename");
                        rename_btn.add_css_class("suggested-action");

                        let btn_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
                        btn_box.set_halign(gtk4::Align::End);
                        btn_box.set_margin_top(12);
                        btn_box.set_margin_end(12);
                        btn_box.set_margin_bottom(12);
                        btn_box.append(&cancel_btn);
                        btn_box.append(&rename_btn);

                        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
                        content.append(&entry);
                        content.append(&btn_box);
                        dialog.set_child(Some(&content));

                        let doc2 = doc_rename.clone();
                        {
                            let dlg = dialog.clone();
                            cancel_btn.connect_clicked(move |_| {
                                dlg.close();
                            });
                        }
                        {
                            let dlg = dialog.clone();
                            let entry2 = entry.clone();
                            rename_btn.connect_clicked(move |_| {
                                let new_name = entry2.text().to_string();
                                if !new_name.is_empty() {
                                    if let Some(l) = doc2.borrow_mut().layer_mut(lid) {
                                        l.name = new_name;
                                    }
                                }
                                dlg.close();
                            });
                        }
                        entry.connect_activate({
                            let dlg = dialog.clone();
                            let entry3 = entry.clone();
                            let doc3 = doc_rename.clone();
                            move |_| {
                                let new_name = entry3.text().to_string();
                                if !new_name.is_empty() {
                                    if let Some(l) = doc3.borrow_mut().layer_mut(lid) {
                                        l.name = new_name;
                                    }
                                }
                                dlg.close();
                            }
                        });

                        dialog.present();
                        gesture.set_state(gtk4::EventSequenceState::Claimed);
                    }
                });
                name_label.add_controller(gesture);
            }

            hbox.append(&name_box);

            // --- Lock icon ---
            if layer.locked {
                let lock_icon = gtk4::Image::from_icon_name("changes-prevent-symbolic");
                lock_icon.set_pixel_size(14);
                lock_icon.set_valign(gtk4::Align::Center);
                hbox.append(&lock_icon);
            }

            // --- Mask thumbnail indicator ---
            if layer.mask.is_some() {
                let mask_thumb = gtk4::DrawingArea::new();
                mask_thumb.set_size_request(MASK_THUMB_SIZE, MASK_THUMB_SIZE);
                mask_thumb.set_valign(gtk4::Align::Center);
                mask_thumb.set_tooltip_text(Some("Layer Mask"));
                mask_thumb.set_draw_func(|_area, cr, w, h| {
                    draw_mask_thumbnail(cr, w, h);
                });
                hbox.append(&mask_thumb);
            }

            row.set_child(Some(&hbox));
            self.layers_list.append(&row);
            ids.push(layer.id);

            if Some(layer.id) == selected {
                self.layers_list.select_row(Some(&row));
            }
        }

        self.refresh_properties(document, selected);
    }

    pub fn refresh_layers(&self, document: &Document, selected_layer: Option<LayerId>) {
        self.refresh_layers_impl(document, selected_layer);
        self.refresh_properties(document, selected_layer);
    }

    fn refresh_properties(&self, document: &Document, selected: Option<LayerId>) {
        while let Some(child) = self.properties_box.first_child() {
            self.properties_box.remove(&child);
        }

        if let Some(id) = selected {
            if let Some(layer) = document.layer(id) {
                let name_entry = gtk4::Entry::new();
                name_entry.set_text(&layer.name);
                name_entry.set_width_chars(16);
                name_entry.set_max_width_chars(24);
                name_entry.set_hexpand(true);
                {
                    let doc = self.document.clone();
                    let lid = layer.id;
                    name_entry.connect_activate(move |entry| {
                        if let Some(l) = doc.borrow_mut().layer_mut(lid) {
                            l.name = entry.text().to_string();
                        }
                    });
                }
                let name_row = adw::ActionRow::new();
                name_row.set_title("Name");
                name_row.add_suffix(&name_entry);
                self.properties_box.append(&name_row);

                let kind_str = match &layer.kind {
                    LayerKind::Raster(_) => "Raster",
                    LayerKind::Text(_) => "Text",
                    LayerKind::Fill(_) => "Fill",
                    LayerKind::Group(_) => "Group",
                    LayerKind::Adjustment(_) => "Adjustment",
                };
                let kind_row = adw::ActionRow::new();
                kind_row.set_title("Type");
                kind_row.set_subtitle(kind_str);
                self.properties_box.append(&kind_row);

                let lock_btn = gtk4::CheckButton::new();
                lock_btn.set_active(layer.locked);
                {
                    let doc = self.document.clone();
                    let lid = layer.id;
                    lock_btn.connect_toggled(move |btn| {
                        if let Some(l) = doc.borrow_mut().layer_mut(lid) {
                            l.locked = btn.is_active();
                        }
                    });
                }
                let lock_row = adw::ActionRow::new();
                lock_row.set_title("Locked");
                lock_row.add_suffix(&lock_btn);
                self.properties_box.append(&lock_row);

                let opacity_adj =
                    gtk4::Adjustment::new(layer.opacity as f64 * 100.0, 0.0, 100.0, 1.0, 10.0, 0.0);
                let opacity_scale =
                    gtk4::Scale::new(gtk4::Orientation::Horizontal, Some(&opacity_adj));
                opacity_scale.set_width_request(120);
                opacity_scale.set_draw_value(true);
                {
                    let doc = self.document.clone();
                    let lid = layer.id;
                    opacity_adj.connect_value_changed(move |adj| {
                        if let Some(l) = doc.borrow_mut().layer_mut(lid) {
                            l.opacity = adj.value() as f32 / 100.0;
                        }
                    });
                }
                let opacity_row = adw::ActionRow::new();
                opacity_row.set_title("Opacity");
                opacity_row.add_suffix(&opacity_scale);
                self.properties_box.append(&opacity_row);

                if let Some(mask) = &layer.mask {
                    let mask_label = gtk4::Label::new(Some("Layer Mask"));
                    mask_label.add_css_class("heading");
                    mask_label.set_halign(gtk4::Align::Start);
                    mask_label.set_margin_top(8);
                    self.properties_box.append(&mask_label);

                    for (title, active, action_name) in [
                        ("Edit Mask", mask.editing, "win.toggle-mask-edit"),
                        ("Enable Mask", mask.enabled, "win.toggle-mask-enabled"),
                        ("Show Mask", mask.show_on_canvas, "win.toggle-mask-view"),
                    ] {
                        let toggle = gtk4::CheckButton::new();
                        toggle.set_active(active);
                        toggle.connect_toggled(move |btn| {
                            if let Some(window) = btn.root().and_downcast::<gtk4::Window>() {
                                if let Err(error) = window.activate_action(action_name, None) {
                                    log::warn!("Mask action '{}' failed: {}", action_name, error);
                                }
                            }
                        });
                        let row = adw::ActionRow::new();
                        row.set_title(title);
                        row.add_suffix(&toggle);
                        self.properties_box.append(&row);
                    }

                    let apply_btn = gtk4::Button::with_label("Apply");
                    apply_btn.add_css_class("suggested-action");
                    apply_btn.connect_clicked(|btn| {
                        if let Some(window) = btn.root().and_downcast::<gtk4::Window>() {
                            if let Err(error) = window.activate_action("win.apply-mask", None) {
                                log::warn!("Apply mask action failed: {}", error);
                            }
                        }
                    });
                    let remove_btn = gtk4::Button::with_label("Remove");
                    remove_btn.add_css_class("destructive-action");
                    remove_btn.connect_clicked(|btn| {
                        if let Some(window) = btn.root().and_downcast::<gtk4::Window>() {
                            if let Err(error) = window.activate_action("win.remove-mask", None) {
                                log::warn!("Remove mask action failed: {}", error);
                            }
                        }
                    });
                    let mask_actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
                    mask_actions.append(&apply_btn);
                    mask_actions.append(&remove_btn);
                    self.properties_box.append(&mask_actions);
                }
            }
        } else {
            let doc_label = gtk4::Label::new(Some("Document"));
            doc_label.add_css_class("heading");
            doc_label.set_halign(gtk4::Align::Start);
            doc_label.set_margin_bottom(8);
            self.properties_box.append(&doc_label);

            let size_row = adw::ActionRow::new();
            size_row.set_title("Canvas Size");
            size_row.set_subtitle(&format!(
                "{} x {}",
                document.canvas_width, document.canvas_height
            ));
            self.properties_box.append(&size_row);

            let profile_str = match document.color_config.working_space {
                crate::document::ColorSpace::Srgb => "sRGB",
                crate::document::ColorSpace::AdobeRgb => "Adobe RGB",
                crate::document::ColorSpace::ProPhotoRgb => "ProPhoto RGB",
                crate::document::ColorSpace::Linear => "Linear",
                crate::document::ColorSpace::Custom(ref s) => s.as_str(),
            };
            let profile_row = adw::ActionRow::new();
            profile_row.set_title("Color Profile");
            profile_row.set_subtitle(profile_str);
            self.properties_box.append(&profile_row);

            let depth_str = match document.color_config.bit_depth {
                crate::document::BitDepth::U8 => "8-bit",
                crate::document::BitDepth::U16 => "16-bit",
                crate::document::BitDepth::Float16 => "16-bit float",
                crate::document::BitDepth::Float32 => "32-bit float",
            };
            let depth_row = adw::ActionRow::new();
            depth_row.set_title("Bit Depth");
            depth_row.set_subtitle(depth_str);
            self.properties_box.append(&depth_row);
        }
    }

    #[allow(dead_code)]
    pub fn selected_layer(&self) -> Option<LayerId> {
        *self.selected_layer.borrow()
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }
}

fn layer_panel_fingerprint(document: &Document) -> LayerPanelFingerprint {
    LayerPanelFingerprint {
        active_layer: document.active_layer_id,
        layers: document
            .layers
            .iter()
            .map(|layer| LayerUiFingerprint {
                id: layer.id,
                name: layer.name.clone(),
                visible: layer.visible,
                locked: layer.locked,
                opacity_bits: layer.opacity.to_bits(),
                blend_mode: layer.blend_mode,
                kind: match layer.kind {
                    LayerKind::Raster(_) => "raster",
                    LayerKind::Text(_) => "text",
                    LayerKind::Fill(_) => "fill",
                    LayerKind::Group(_) => "group",
                    LayerKind::Adjustment(_) => "adjustment",
                },
                mask: layer.mask.as_ref().map(|mask| MaskUiFingerprint {
                    visible: mask.visible,
                    linked: mask.linked,
                    enabled: mask.enabled,
                    editing: mask.editing,
                    show_on_canvas: mask.show_on_canvas,
                }),
            })
            .collect(),
    }
}

struct RightPanelsWeak {
    layers_list: glib::WeakRef<gtk4::ListBox>,
    properties_box: glib::WeakRef<gtk4::Box>,
    history_list: glib::WeakRef<gtk4::ListBox>,
    navigator_preview: glib::WeakRef<gtk4::DrawingArea>,
    document: Rc<RefCell<Document>>,
    layer_ids: Rc<RefCell<Vec<LayerId>>>,
    blend_model: glib::WeakRef<gtk4::DropDown>,
    opacity_scale: glib::WeakRef<gtk4::Scale>,
    opacity_adj: glib::WeakRef<gtk4::Adjustment>,
    mask_button: glib::WeakRef<gtk4::Button>,
    delete_button: glib::WeakRef<gtk4::Button>,
    selected_layer: Rc<RefCell<Option<LayerId>>>,
    refreshing: Rc<RefCell<bool>>,
}

impl RightPanelsWeak {
    fn upgrade(&self) -> Option<RightPanels> {
        Some(RightPanels {
            widget: gtk4::Box::new(gtk4::Orientation::Vertical, 0),
            layers_list: self.layers_list.upgrade()?,
            properties_box: self.properties_box.upgrade()?,
            history_list: self.history_list.upgrade()?,
            navigator_preview: self.navigator_preview.upgrade()?,
            document: self.document.clone(),
            layer_ids: self.layer_ids.clone(),
            blend_model: self.blend_model.upgrade()?,
            opacity_scale: self.opacity_scale.upgrade()?,
            opacity_adj: self.opacity_adj.upgrade()?,
            mask_button: self.mask_button.upgrade()?,
            delete_button: self.delete_button.upgrade()?,
            selected_layer: self.selected_layer.clone(),
            refreshing: self.refreshing.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// Drawing helpers
// ---------------------------------------------------------------------------

fn draw_navigator_preview(cr: &cairo::Context, width: i32, height: i32, document: &Document) {
    let wf = width as f64;
    let hf = height as f64;
    draw_checkerboard(cr, wf, hf);

    if document.canvas_width == 0 || document.canvas_height == 0 || document.layers.is_empty() {
        cr.set_source_rgba(0.5, 0.5, 0.5, 0.7);
        cr.set_line_width(1.0);
        cr.rectangle(0.5, 0.5, wf - 1.0, hf - 1.0);
        cr.stroke().ok();
        return;
    }

    let frame = build_render_frame(document);
    let flattened = flatten_frame_bgra(&frame);
    if flattened.pixels_bgra.is_empty() {
        return;
    }

    let mut surf = match cairo::ImageSurface::create(
        cairo::Format::ARgb32,
        flattened.width as i32,
        flattened.height as i32,
    ) {
        Ok(surface) => surface,
        Err(_) => return,
    };

    let stride = surf.stride() as usize;
    {
        let mut surf_data = match surf.data() {
            Ok(data) => data,
            Err(_) => return,
        };
        for row in 0..flattened.height as usize {
            for col in 0..flattened.width as usize {
                let src = (row * flattened.width as usize + col) * 4;
                let dst = row * stride + col * 4;
                if src + 3 >= flattened.pixels_bgra.len() || dst + 3 >= surf_data.len() {
                    continue;
                }
                surf_data[dst..dst + 4].copy_from_slice(&flattened.pixels_bgra[src..src + 4]);
            }
        }
    }

    let scale = (wf / flattened.width as f64).min(hf / flattened.height as f64);
    let draw_w = flattened.width as f64 * scale;
    let draw_h = flattened.height as f64 * scale;
    let x = (wf - draw_w) / 2.0;
    let y = (hf - draw_h) / 2.0;

    cr.save().ok();
    cr.translate(x, y);
    cr.scale(scale, scale);
    cr.set_source_surface(&surf, 0.0, 0.0).ok();
    cr.paint().ok();
    cr.restore().ok();

    cr.set_source_rgba(0.5, 0.5, 0.5, 0.7);
    cr.set_line_width(1.0);
    cr.rectangle(x + 0.5, y + 0.5, draw_w - 1.0, draw_h - 1.0);
    cr.stroke().ok();
}

fn draw_layer_thumbnail(
    cr: &cairo::Context,
    width: i32,
    height: i32,
    kind: &LayerKind,
    raster_data: Option<&(u32, u32, Vec<u8>)>,
    fill_color: Option<&[f32; 4]>,
) {
    let wf = width as f64;
    let hf = height as f64;
    let pad = 2.0;

    // Checkerboard background
    draw_checkerboard(cr, wf, hf);

    match kind {
        LayerKind::Raster(_) => {
            if let Some((rw, rh, data)) = raster_data {
                draw_raster_thumbnail(cr, wf, hf, pad, *rw, *rh, data);
            }
        }
        LayerKind::Fill(_) => {
            if let Some(c) = fill_color {
                cr.set_source_rgba(c[0] as f64, c[1] as f64, c[2] as f64, c[3] as f64);
                cr.rectangle(pad, pad, wf - pad * 2.0, hf - pad * 2.0);
                cr.fill().ok();
            }
        }
        LayerKind::Text(_) => {
            draw_kind_icon(cr, wf, hf, pad, "T");
        }
        LayerKind::Group(_) => {
            draw_kind_icon(cr, wf, hf, pad, "G");
        }
        LayerKind::Adjustment(_) => {
            draw_kind_icon(cr, wf, hf, pad, "fx");
        }
    }

    // Thin border
    cr.set_source_rgba(0.5, 0.5, 0.5, 0.6);
    cr.set_line_width(1.0);
    cr.rectangle(pad, pad, wf - pad * 2.0, hf - pad * 2.0);
    cr.stroke().ok();
}

fn draw_checkerboard(cr: &cairo::Context, w: f64, h: f64) {
    let cs = 6.0;
    let dark = (0.12, 0.12, 0.12);
    let light = (0.18, 0.18, 0.18);

    let mut y = 0.0;
    let mut row = 0;
    while y < h {
        let mut x = 0.0;
        let mut col = row % 2;
        while x < w {
            let color = if col % 2 == 0 { dark } else { light };
            cr.set_source_rgb(color.0, color.1, color.2);
            let rect_w = if x + cs > w { w - x } else { cs };
            let rect_h = if y + cs > h { h - y } else { cs };
            cr.rectangle(x, y, rect_w, rect_h);
            cr.fill().ok();
            x += cs;
            col += 1;
        }
        y += cs;
        row += 1;
    }
}

fn draw_raster_thumbnail(
    cr: &cairo::Context,
    w: f64,
    h: f64,
    pad: f64,
    rw: u32,
    rh: u32,
    data: &[u8],
) {
    if rw == 0 || rh == 0 || data.len() < (rw * rh * 4) as usize {
        return;
    }

    // Create a surface from RGBA data
    let mut surf = match cairo::ImageSurface::create(cairo::Format::ARgb32, rw as i32, rh as i32) {
        Ok(s) => s,
        Err(_) => return,
    };

    let stride = surf.stride() as usize;
    {
        let mut surf_data = surf.data().expect("surface data");

        for y in 0..rh as usize {
            for x in 0..rw as usize {
                let src_idx = (y * rw as usize + x) * 4;
                let dst_idx = y * stride + x * 4;

                let r = data[src_idx] as u32;
                let g = data[src_idx + 1] as u32;
                let b = data[src_idx + 2] as u32;
                let a = data[src_idx + 3] as u32;

                // Cairo ARgb32 is native-endian BGRA
                let pixel: u32 = (b) | (g << 8) | (r << 16) | (a << 24);
                surf_data[dst_idx..dst_idx + 4].copy_from_slice(&pixel.to_ne_bytes());
            }
        }
    }

    let dest_w = w - pad * 2.0;
    let dest_h = h - pad * 2.0;
    let scale_x = dest_w / (rw as f64);
    let scale_y = dest_h / (rh as f64);
    let scale = scale_x.min(scale_y);
    let offset_x = pad + (dest_w - rw as f64 * scale) / 2.0;
    let offset_y = pad + (dest_h - rh as f64 * scale) / 2.0;

    cr.save().ok();
    cr.rectangle(pad, pad, dest_w, dest_h);
    cr.clip();
    cr.translate(offset_x, offset_y);
    cr.scale(scale, scale);
    cr.set_source_surface(&surf, 0.0, 0.0).ok();
    cr.paint().ok();
    cr.restore().ok();
}

fn draw_kind_icon(cr: &cairo::Context, w: f64, h: f64, pad: f64, label: &str) {
    let bg_color = match label {
        "T" => (0.70, 0.45, 0.30),
        "G" => (0.55, 0.55, 0.40),
        "fx" => (0.55, 0.30, 0.65),
        _ => (0.35, 0.50, 0.70),
    };
    cr.set_source_rgb(bg_color.0, bg_color.1, bg_color.2);
    cr.rectangle(pad, pad, w - pad * 2.0, h - pad * 2.0);
    cr.fill().ok();

    // Draw label text
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.9);
    cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(12.0);
    let ext = cr.text_extents(label).ok();
    if let Some(ext) = ext {
        let tx = pad + (w - pad * 2.0 - ext.width()) / 2.0 - ext.x_bearing();
        let ty = pad + (h - pad * 2.0 - ext.height()) / 2.0 - ext.y_bearing();
        cr.move_to(tx, ty);
        cr.show_text(label).ok();
    }
}

fn draw_mask_thumbnail(cr: &cairo::Context, w: i32, h: i32) {
    let wf = w as f64;
    let hf = h as f64;
    let pad = 1.0;

    draw_checkerboard(cr, wf, hf);

    cr.set_source_rgba(0.9, 0.9, 0.9, 1.0);
    cr.rectangle(pad, pad, wf - pad * 2.0, hf - pad * 2.0);
    cr.fill().ok();

    cr.set_source_rgba(0.5, 0.5, 0.5, 0.8);
    cr.set_line_width(1.0);
    cr.rectangle(pad, pad, wf - pad * 2.0, hf - pad * 2.0);
    cr.stroke().ok();
}
