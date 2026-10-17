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

pub fn pick_temperature_icon() -> String {
    pick_icon_name(&[
        "thermometer-symbolic",
        "sensors-temperature-symbolic",
        "temperature-high-symbolic",
        "weather-clear-symbolic",
        "dialog-information-symbolic",
    ])
}

pub struct TempWidgets {
    pub label: gtk4::Label,
    pub bar: gtk4::LevelBar,
    pub container: gtk4::Box,
}

pub fn create_temp_widgets(compact: bool) -> TempWidgets {
    let container = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    container.set_valign(gtk4::Align::Center);
    container.set_halign(gtk4::Align::End);

    let label = gtk4::Label::new(Some("—"));
    label.add_css_class("dim-label");
    label.add_css_class("temp-label");
    label.set_xalign(1.0);
    container.append(&label);

    let bar = gtk4::LevelBar::new();
    bar.set_min_value(0.0);
    bar.set_max_value(105.0);
    bar.set_value(0.0);
    bar.set_hexpand(false);
    bar.set_size_request(if compact { 96 } else { 120 }, -1);
    bar.set_valign(gtk4::Align::Center);
    bar.add_css_class("temp-gauge");
    bar.add_offset_value("low", 55.0);
    bar.add_offset_value("high", 75.0);
    bar.add_offset_value("full", 90.0);
    container.append(&bar);

    TempWidgets {
        label,
        bar,
        container,
    }
}

pub fn set_temp_widgets(widgets: &TempWidgets, temp_c: Option<f64>) {
    set_temp_widgets_with_prefix(widgets, temp_c, None)
}

fn update_temp_bar_style(bar: &gtk4::LevelBar, temp_c: Option<f64>) {
    bar.remove_css_class("temp-low");
    bar.remove_css_class("temp-high");
    bar.remove_css_class("temp-full");
    if let Some(temp) = temp_c {
        // Keep in sync with LevelBar offsets but ensure visually correct
        if temp >= 90.0 {
            bar.add_css_class("temp-full");
        } else if temp >= 60.0 {
            bar.add_css_class("temp-high");
        } else {
            bar.add_css_class("temp-low");
        }
    }
}

pub fn set_temp_widgets_with_prefix(
    widgets: &TempWidgets,
    temp_c: Option<f64>,
    prefix: Option<&str>,
) {
    match (temp_c, prefix) {
        (None, Some(p)) => {
            widgets.label.set_label(&format!("{p} N/A"));
            widgets.bar.set_value(0.0);
        }
        (None, None) => {
            widgets.label.set_label("N/A");
            widgets.bar.set_value(0.0);
        }
        (Some(temp), Some(p)) => {
            widgets.label.set_label(&format!("{p} {:.0} °C", temp));
            widgets.bar.set_value(temp.min(105.0));
        }
        (Some(temp), None) => {
            widgets.label.set_label(&format!("{:.0} °C", temp));
            widgets.bar.set_value(temp.min(105.0));
        }
    }
    update_temp_bar_style(&widgets.bar, temp_c);
}

#[allow(dead_code)]
pub struct TempGauge {
    pub row: libadwaita::ActionRow,
    label: gtk4::Label,
    bar: gtk4::LevelBar,
}

#[allow(dead_code)]
impl TempGauge {
    pub fn new() -> Self {
        let row = libadwaita::ActionRow::new();
        row.set_title("Temperature");
        row.set_activatable(false);

        let icon_name = pick_temperature_icon();
        row.add_prefix(&gtk4::Image::from_icon_name(&icon_name));

        let widgets = create_temp_widgets(false);
        row.add_suffix(&widgets.container);

        TempGauge {
            row,
            label: widgets.label,
            bar: widgets.bar,
        }
    }

    pub fn set_title(&self, title: &str) {
        self.row.set_title(title);
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
        update_temp_bar_style(&self.bar, temp_c);
    }

    pub fn widget(&self) -> &libadwaita::ActionRow {
        &self.row
    }
}
