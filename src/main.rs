#![allow(clippy::all)]

use gtk4::{gio, prelude::*};

use crate::ui::window::MainWindow;

mod document;
mod image;
mod tile;
mod tools;
mod ui;

pub struct App {
    app: adw::Application,
}

impl App {
    pub fn new() -> Self {
        let app = adw::Application::builder()
            .application_id("com.slate.editor")
            .flags(gio::ApplicationFlags::HANDLES_OPEN)
            .resource_base_path("/com/slate/editor")
            .build();

        app.connect_startup(|_| {
            let display = gdk4::Display::default().expect("no display");
            let provider = gtk4::CssProvider::new();
            provider.load_from_string(".dim-label { opacity: 0.6; }");
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
            gtk4::IconTheme::for_display(&display)
                .add_search_path(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icons"));

            let style_mgr = adw::StyleManager::default();
            style_mgr.set_color_scheme(adw::ColorScheme::Default);
        });

        app.connect_activate(move |app| {
            let main_window = MainWindow::new(app);
            crate::ui::hardening::install(&main_window);
            main_window.window.present();
        });

        app.connect_open(move |app, files, _hint| {
            let main_window = MainWindow::new(app);
            crate::ui::hardening::install(&main_window);
            if let Some(path) = files.first().and_then(|file| file.path()) {
                crate::ui::hardening::open_path(&main_window, &path);
            }
            main_window.window.present();
        });

        Self { app }
    }

    pub fn run(&self) {
        self.app.run();
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let app = App::new();
    app.run();
}
