use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::backend::hardware::SystemSpecs;

pub fn create_identity_view(
    specs: &SystemSpecs,
    _toast_fn: Box<dyn Fn(String)>,
) -> gtk4::ScrolledWindow {
    let scrolled = gtk4::ScrolledWindow::new();
    scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let clamp = libadwaita::Clamp::new();
    clamp.set_maximum_size(600);
    clamp.set_margin_top(12);
    clamp.set_margin_bottom(24);
    clamp.set_margin_start(12);
    clamp.set_margin_end(12);

    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 24);

    // System group
    let sys_group = libadwaita::PreferencesGroup::new();
    sys_group.set_title("System");
    sys_group.add(&make_row("Operating System", &specs.os_name, "drive-harddisk-symbolic"));
    sys_group.add(&make_row("Kernel", &specs.kernel, "application-x-firmware-symbolic"));
    content.append(&sys_group);

    // Processor group
    let cpu_group = libadwaita::PreferencesGroup::new();
    cpu_group.set_title("Processor");
    cpu_group.add(&make_row(
        "Model",
        &specs.cpu_model,
        "processor-symbolic",
    ));
    cpu_group.add(&make_row(
        "Cores",
        &format!("{} cores, {} threads", specs.cpu_cores_physical, specs.cpu_cores_logical),
        "view-grid-symbolic",
    ));
    if specs.cpu_freq_max_ghz() > 0.0 {
        cpu_group.add(&make_row(
            "Max Frequency",
            &format!("{:.2} GHz", specs.cpu_freq_max_ghz()),
            "arrow-pointing-at-line-up-symbolic",
        ));
    }
    content.append(&cpu_group);

    // Graphics group
    let gpu_group = libadwaita::PreferencesGroup::new();
    gpu_group.set_title("Graphics");
    if specs.gpus.is_empty() {
        gpu_group.add(&make_row_with_subtitle(
            "GPU",
            "Unknown GPU",
            "dialog-question-symbolic",
            "Could not detect a graphics card",
        ));
    } else {
        for (i, gpu) in specs.gpus.iter().enumerate() {
            let mut subtitle_parts: Vec<&str> = Vec::new();
            if !gpu.vendor.is_empty() {
                subtitle_parts.push(&gpu.vendor);
            }
            if gpu.is_integrated {
                subtitle_parts.push("Integrated");
            }
            let subtitle = subtitle_parts.join(" · ");
            gpu_group.add(&make_row_with_subtitle(
                &format!("GPU {i}"),
                &gpu.name,
                "pci-card-symbolic",
                &subtitle,
            ));
        }
    }
    content.append(&gpu_group);

    // Memory group
    let mem_group = libadwaita::PreferencesGroup::new();
    mem_group.set_title("Memory");
    let ram_total_gb = specs.ram_total_bytes as f64 / 1_000_000_000.0;
    mem_group.add(&make_row(
        "Total RAM",
        &format!("{ram_total_gb:.1} GB"),
        "memory-symbolic",
    ));
    content.append(&mem_group);

    // Copy Specs button
    let button_bin = libadwaita::Bin::new();
    button_bin.set_margin_top(8);

    let copy_button = gtk4::Button::new();
    copy_button.set_hexpand(true);
    copy_button.set_halign(gtk4::Align::Fill);
    copy_button.add_css_class("suggested-action");
    copy_button.add_css_class("button-row");
    copy_button.set_tooltip_text(Some("Copy hardware specs as plain text"));

    let button_content = libadwaita::ButtonContent::new();
    button_content.set_icon_name("edit-copy-symbolic");
    button_content.set_label("Copy Specs");
    copy_button.set_child(Some(&button_content));

    let specs_text = specs.to_plain_text();
    copy_button.connect_clicked(move |button| {
        if let Some(display) = gtk4::gdk::Display::default() {
            display.clipboard().set_text(&specs_text);
        }
        button.add_css_class("success");
        button_content.set_label("Copied!");
        button_content.set_icon_name("object-select-symbolic");

        let btn = button.clone();
        let bcontent = button_content.clone();
        glib::timeout_add_seconds_local(2, move || {
            btn.remove_css_class("success");
            bcontent.set_label("Copy Specs");
            bcontent.set_icon_name("edit-copy-symbolic");
            glib::ControlFlow::Break
        });
    });

    button_bin.set_child(Some(&copy_button));
    content.append(&button_bin);

    clamp.set_child(Some(&content));
    scrolled.set_child(Some(&clamp));

    scrolled
}

fn make_row(title: &str, value: &str, icon_name: &str) -> libadwaita::ActionRow {
    make_row_with_subtitle(title, value, icon_name, "")
}

fn make_row_with_subtitle(
    title: &str,
    value: &str,
    icon_name: &str,
    subtitle: &str,
) -> libadwaita::ActionRow {
    let row = libadwaita::ActionRow::new();
    row.set_title(title);
    if !subtitle.is_empty() {
        row.set_subtitle(subtitle);
    }
    row.set_activatable(false);

    let resolved = if let Some(display) = gtk4::gdk::Display::default() {
        let icon_theme = gtk4::IconTheme::for_display(&display);
        if icon_theme.has_icon(icon_name) {
            icon_name.to_string()
        } else {
            "computer-symbolic".to_string()
        }
    } else {
        icon_name.to_string()
    };

    row.add_prefix(&gtk4::Image::from_icon_name(&resolved));

    let label = gtk4::Label::new(Some(value));
    label.add_css_class("dim-label");
    label.set_selectable(true);
    label.set_xalign(1.0);
    label.set_wrap(true);
    label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
    label.set_max_width_chars(28);
    row.add_suffix(&label);

    row
}
