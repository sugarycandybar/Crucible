use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::backend::hardware::gather_specs;
use crate::backend::monitor::SystemMonitor;
use crate::config;
use crate::stability_view;
use crate::window;

fn ensure_schema() {
    let _schema_id = "io.github.sugarycandybar.Crucible.Stability";
    let _already_set = std::env::var("GSETTINGS_SCHEMA_DIR")
        .ok()
        .is_some_and(|d| !d.is_empty());

    // Try to find and compile the schema from the source tree
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));
    let search_paths = vec![
        std::path::PathBuf::from("data"),
        exe_dir
            .as_ref()
            .map(|d| d.join("../../data"))
            .unwrap_or_default(),
        exe_dir
            .as_ref()
            .map(|d| d.join("../../../data"))
            .unwrap_or_default(),
    ];

    for path in &search_paths {
        let schema_path = path.join("io.github.sugarycandybar.Crucible.Stability.gschema.xml");
        if schema_path.exists() {
            let _ = std::process::Command::new("glib-compile-schemas")
                .arg(path)
                .status();
            let abs = std::fs::canonicalize(path).ok();
            if let Some(abs_path) = abs {
                let existing = std::env::var("GSETTINGS_SCHEMA_DIR").unwrap_or_default();
                let combined = if existing.is_empty() {
                    abs_path.to_string_lossy().to_string()
                } else {
                    format!("{}:{}", abs_path.to_string_lossy(), existing)
                };
                std::env::set_var("GSETTINGS_SCHEMA_DIR", &combined);
            }
            break;
        }
    }
}

pub fn run_app() {
    ensure_schema();
    let app = libadwaita::Application::new(
        Some(config::APP_ID),
        gio::ApplicationFlags::default(),
    );
    app.set_resource_base_path(Some("/io/github/sugarycandybar/Crucible"));

    let monitor = Rc::new(RefCell::new(SystemMonitor::new()));
    let stability_state_holder: Rc<RefCell<Option<Rc<RefCell<stability_view::StabilityState>>>>>
        = Rc::new(RefCell::new(None));

    app.connect_startup(|app| {
        // Load CSS
        let provider = gtk4::CssProvider::new();
        provider.load_from_string(include_str!("style.css"));

        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        // Register embedded GResource icons
        let resource_data = include_bytes!(concat!(env!("OUT_DIR"), "/crucible.gresource"));
        let bytes = glib::Bytes::from_static(resource_data);
        if let Ok(resource) = gio::Resource::from_data(&bytes) {
            gio::resources_register(&resource);
            if let Some(display) = gtk4::gdk::Display::default() {
                let icon_theme = gtk4::IconTheme::for_display(&display);
                icon_theme.add_resource_path("/io/github/sugarycandybar/Crucible/icons");
            }
        }

        // Setup actions
        let about_action = gio::SimpleAction::new("about", None);
        let app_weak = app.downgrade();
        about_action.connect_activate(move |_, _| {
            if let Some(app) = app_weak.upgrade() {
                let window = app.active_window();
                let dialog = libadwaita::AboutDialog::builder()
                    .application_name(config::APP_NAME)
                    .application_icon(config::APP_ID)
                    .version(config::VERSION)
                    .developer_name("Sugarycandybar")
                    .website("https://github.com/sugarycandybar/Crucible")
                    .issue_url("https://github.com/sugarycandybar/Crucible/issues")
                    .license_type(gtk4::License::Gpl30)
                    .comments("View hardware specs and stress test your system.")
                    .build();
                dialog.add_acknowledgement_section(
                    Some("Acknowledgements"),
                    &["stress-ng https://github.com/ColinIanKing/stress-ng"],
                );
                dialog.add_other_app(
                    "io.github.sugarycandybar.Hosty",
                    "Hosty",
                    "Host Minecraft servers",
                );
                dialog.add_other_app(
                    "io.github.sugarycandybar.Carabiner",
                    "Carabiner",
                    "Create and manage network tunnels",
                );
                dialog.present(window.as_ref());
            }
        });
        app.add_action(&about_action);

        let quit_action = gio::SimpleAction::new("quit", None);
        let app_weak = app.downgrade();
        quit_action.connect_activate(move |_, _| {
            if let Some(app) = app_weak.upgrade() {
                app.quit();
            }
        });
        app.add_action(&quit_action);
        app.set_accels_for_action("app.quit", &["<Primary>q"]);
    });

    let monitor_clone = Rc::clone(&monitor);
    let holder_clone = Rc::clone(&stability_state_holder);

    app.connect_activate(move |app| {
        if !app.windows().is_empty() {
            app.windows().first().unwrap().present();
            return;
        }

        let specs = gather_specs();

        let (window, stability_state) =
            window::create_window(specs, Rc::clone(&monitor_clone), app);

        *holder_clone.borrow_mut() = Some(Rc::clone(&stability_state));
        window.present();
    });

    app.connect_shutdown(move |_| {
        if let Some(ref state) = *stability_state_holder.borrow() {
            state.borrow_mut().stress.kill();
        }
    });

    app.run();
}
