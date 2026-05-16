use gtk4::prelude::*;
use libadwaita::prelude::*;

fn pick_icon_name(candidates: &[&str]) -> String {
    if let Some(display) = gtk4::gdk::Display::default() {
        let icon_theme = gtk4::IconTheme::for_display(&display);
        for name in candidates {
            if icon_theme.has_icon(name) {
                return name.to_string();
            }
        }
    }
    candidates
        .last()
        .unwrap_or(&"dialog-information-symbolic")
        .to_string()
}

pub struct TempGauge {
    pub row: libadwaita::ActionRow,
    label: gtk4::Label,
    bar: gtk4::LevelBar,
}

impl TempGauge {
    pub fn new() -> Self {
        let row = libadwaita::ActionRow::new();
        row.set_title("CPU Temperature");
        row.set_activatable(false);

        let icon_name = pick_icon_name(&[
            "thermometer-symbolic",
            "sensors-temperature-symbolic",
            "temperature-high-symbolic",
            "weather-clear-symbolic",
            "dialog-information-symbolic",
        ]);
        row.add_prefix(&gtk4::Image::from_icon_name(&icon_name));

        let box_ = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        box_.set_valign(gtk4::Align::Center);

        let label = gtk4::Label::new(Some("—"));
        label.add_css_class("dim-label");
        label.set_width_chars(5);
        label.set_xalign(1.0);
        box_.append(&label);

        let bar = gtk4::LevelBar::new();
        bar.set_min_value(0.0);
        bar.set_max_value(105.0);
        bar.set_value(0.0);
        bar.set_hexpand(false);
        bar.set_size_request(120, -1);
        bar.set_valign(gtk4::Align::Center);
        bar.add_css_class("temp-gauge");
        bar.add_offset_value("low", 55.0);
        bar.add_offset_value("high", 75.0);
        bar.add_offset_value("full", 90.0);
        box_.append(&bar);

        row.add_suffix(&box_);

        TempGauge { row, label, bar }
    }

    pub fn set_temperature(&self, temp_c: Option<f64>) {
        match temp_c {
            None => {
                self.label.set_label("N/A");
                self.bar.set_value(0.0);
            }
            Some(temp) => {
                self.label.set_label(&format!("{:.0} °C", temp));
                self.bar.set_value(temp.min(105.0));
            }
        }
    }

    pub fn widget(&self) -> &libadwaita::ActionRow {
        &self.row
    }
}
