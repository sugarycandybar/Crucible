use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::backend::monitor::MonitorSnapshot;
use crate::backend::stress::StressManager;
use crate::ui::widgets::temp_gauge::TempGauge;

const DURATIONS: &[(u64, &str)] = &[
    (300, "5 minutes"),
    (600, "10 minutes"),
    (1800, "30 minutes"),
    (0, "Until stopped"),
];

const TEMP_LIMITS_C: &[u64] = &[100, 95, 90];

pub struct StabilityState {
    pub stress: StressManager,
    pub was_running: bool,
    pub temp_gauge: TempGauge,
    pub usage_label: gtk4::Label,
    pub ram_label: gtk4::Label,
    pub action_button: gtk4::Button,
    pub duration_row: libadwaita::ComboRow,
    pub temp_limit_row: libadwaita::ComboRow,
    pub stop_at_temp_row: libadwaita::SwitchRow,
    pub app: Option<libadwaita::Application>,
}

impl StabilityState {
    fn send_desktop_notification(&self, body: &str) {
        if let Some(ref app) = self.app {
            let notification = gio::Notification::new("Crucible");
            notification.set_body(Some(body));
            app.send_notification(Some("stress-notification"), &notification);
        }
    }

    fn overheat_cutoff_c(&self) -> f64 {
        TEMP_LIMITS_C[self.temp_limit_row.selected() as usize] as f64
    }

    fn refresh_button_state(&mut self) {
        if self.stress.is_running() {
            let elapsed = self.stress.elapsed_seconds() as i64;
            let mins = elapsed / 60;
            let secs = elapsed % 60;
            self.action_button
                .set_label(&format!("Stop Test  {mins:02}:{secs:02}"));
            self.action_button.remove_css_class("suggested-action");
            self.action_button.add_css_class("destructive-action");
            self.duration_row.set_sensitive(false);
        } else {
            self.action_button.set_label("Start Test");
            self.action_button.remove_css_class("destructive-action");
            self.action_button.add_css_class("suggested-action");
            if StressManager::is_available() {
                self.duration_row.set_sensitive(true);
            }
        }
    }

    pub fn update(&mut self, snapshot: &MonitorSnapshot) {
        self.temp_gauge.set_temperature(snapshot.cpu_temp_c);
        self.usage_label
            .set_label(&format!("{:.0} %", snapshot.cpu_usage_pct));

        let used_gb = snapshot.ram_used_bytes as f64 / 1_000_000_000.0;
        let total_gb = snapshot.ram_total_bytes as f64 / 1_000_000_000.0;
        self.ram_label
            .set_label(&format!("{used_gb:.1} / {total_gb:.1} GB"));

        if self.stress.is_running()
            && self.stop_at_temp_row.is_active()
            && snapshot.cpu_temp_c.is_some()
            && snapshot.cpu_temp_c.unwrap() >= self.overheat_cutoff_c()
        {
            self.stress.stop("overheat");
        }

        let running_now = self.stress.is_running();
        if self.was_running && !running_now {
            let elapsed = format_elapsed(self.stress.last_elapsed_seconds());
            match self.stress.last_stop_cause() {
                Some("overheat") => {
                    self.send_desktop_notification(&format!(
                        "Test stopped to prevent overheating (Elapsed: {elapsed})"
                    ));
                }
                Some("completed") => {
                    self.send_desktop_notification(&format!(
                        "Test completed successfully (Elapsed: {elapsed})"
                    ));
                }
                _ => {
                    self.send_desktop_notification(&format!(
                        "Test stopped (Elapsed: {elapsed})"
                    ));
                }
            }
        }

        self.was_running = running_now;
        self.refresh_button_state();
    }
}

pub fn create_stability_view(
    stress: StressManager,
    app: Option<libadwaita::Application>,
) -> (gtk4::ScrolledWindow, Rc<RefCell<StabilityState>>) {
    let settings = gio::Settings::new("io.github.sugarycandybar.Crucible.Stability");

    let scrolled = gtk4::ScrolledWindow::new();
    scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let clamp = libadwaita::Clamp::new();
    clamp.set_maximum_size(600);
    clamp.set_margin_top(12);
    clamp.set_margin_bottom(24);
    clamp.set_margin_start(12);
    clamp.set_margin_end(12);

    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 24);

    // --- Monitoring group ---
    let mon_group = libadwaita::PreferencesGroup::new();
    mon_group.set_title("Monitoring");

    let temp_gauge = TempGauge::new();
    mon_group.add(temp_gauge.widget());

    let usage_row = libadwaita::ActionRow::new();
    usage_row.set_title("CPU Usage");
    usage_row.set_activatable(false);
    usage_row.add_prefix(&gtk4::Image::from_icon_name("processor-symbolic"));
    let usage_label = gtk4::Label::new(Some("—"));
    usage_label.add_css_class("dim-label");
    usage_row.add_suffix(&usage_label);
    mon_group.add(&usage_row);

    let ram_row = libadwaita::ActionRow::new();
    ram_row.set_title("Memory Usage");
    ram_row.set_activatable(false);
    ram_row.add_prefix(&gtk4::Image::from_icon_name("memory-symbolic"));
    let ram_label = gtk4::Label::new(Some("—"));
    ram_label.add_css_class("dim-label");
    ram_row.add_suffix(&ram_label);
    mon_group.add(&ram_row);

    content.append(&mon_group);

    // --- Settings group ---
    let ctrl_group = libadwaita::PreferencesGroup::new();
    ctrl_group.set_title("Settings");

    let duration_row = libadwaita::ComboRow::new();
    duration_row.set_title("Duration");
    duration_row.add_prefix(&gtk4::Image::from_icon_name("alarm-symbolic"));
    let duration_model = gtk4::StringList::new(&[]);
    for (_, label) in DURATIONS {
        duration_model.append(label);
    }
    duration_row.set_model(Some(&duration_model));
    duration_row.set_selected(settings.uint("duration-index"));
    ctrl_group.add(&duration_row);

    let stop_at_temp_row = libadwaita::SwitchRow::new();
    stop_at_temp_row.set_title("Stop at Temperature Limit");
    stop_at_temp_row.set_active(settings.boolean("stop-at-temp"));
    stop_at_temp_row.add_prefix(&gtk4::Image::from_icon_name("stop-sign-large-symbolic"));
    ctrl_group.add(&stop_at_temp_row);

    let temp_limit_row = libadwaita::ComboRow::new();
    temp_limit_row.set_title("Temperature Limit");
    let temp_limit_model = gtk4::StringList::new(&[]);
    for temp in TEMP_LIMITS_C {
        temp_limit_model.append(&format!("{temp}C"));
    }
    temp_limit_row.set_model(Some(&temp_limit_model));
    temp_limit_row.set_selected(settings.uint("temp-limit-index"));
    ctrl_group.add(&temp_limit_row);

    content.append(&ctrl_group);

    // --- Action button ---
    let action_bin = libadwaita::Bin::new();
    action_bin.set_margin_top(8);

    let action_button = gtk4::Button::with_label("Start Test");
    action_button.set_hexpand(true);
    action_button.set_halign(gtk4::Align::Fill);
    action_button.add_css_class("suggested-action");
    action_button.add_css_class("button-row");
    action_button.set_tooltip_text(Some("Runs a stress test to check cooling and stability"));
    action_bin.set_child(Some(&action_button));
    content.append(&action_bin);

    // stress-ng not found banner
    if !StressManager::is_available() {
        action_button.set_sensitive(false);
        duration_row.set_sensitive(false);
        let banner = libadwaita::Banner::new("stress-ng is not installed");
        banner.set_button_label(Some(""));
        banner.set_revealed(true);
        content.prepend(&banner);
    }

    clamp.set_child(Some(&content));
    scrolled.set_child(Some(&clamp));

    let state = Rc::new(RefCell::new(StabilityState {
        stress,
        was_running: false,
        temp_gauge,
        usage_label,
        ram_label,
        action_button: action_button.clone(),
        duration_row: duration_row.clone(),
        temp_limit_row: temp_limit_row.clone(),
        stop_at_temp_row: stop_at_temp_row.clone(),
        app,
    }));

    duration_row.connect_notify(Some("selected"), move |row, _| {
        let val = row.selected();
        glib::idle_add_local(move || {
            let s = gio::Settings::new("io.github.sugarycandybar.Crucible.Stability");
            let _ = s.set_uint("duration-index", val);
            glib::ControlFlow::Break
        });
    });

    temp_limit_row.connect_notify(Some("selected"), move |row, _| {
        let val = row.selected();
        glib::idle_add_local(move || {
            let s = gio::Settings::new("io.github.sugarycandybar.Crucible.Stability");
            let _ = s.set_uint("temp-limit-index", val);
            glib::ControlFlow::Break
        });
    });

    stop_at_temp_row.connect_notify(Some("active"), move |row, _| {
        let active = row.is_active();
        glib::idle_add_local(move || {
            let s = gio::Settings::new("io.github.sugarycandybar.Crucible.Stability");
            let _ = s.set_boolean("stop-at-temp", active);
            glib::ControlFlow::Break
        });
    });

    let sig_state = state.clone();
    action_button.connect_clicked(move |_| {
        let mut st = sig_state.borrow_mut();
        if st.stress.is_running() {
            st.stress.stop("manual");
            let elapsed = format_elapsed(st.stress.last_elapsed_seconds());
            st.send_desktop_notification(&format!("Test stopped manually (Elapsed: {elapsed})"));
            st.was_running = false;
            st.refresh_button_state();
            return;
        }
        let idx = st.duration_row.selected();
        let duration = DURATIONS[idx as usize].0;
        if st.stress.start(duration) {
            st.was_running = true;
            st.refresh_button_state();
        } else {
            st.send_desktop_notification("Could not start stress-ng");
        }
    });

    (scrolled, state)
}

fn format_elapsed(total_seconds: f64) -> String {
    let elapsed = (total_seconds as i64).max(0);
    let mins = elapsed / 60;
    let secs = elapsed % 60;
    format!("{mins:02}:{secs:02}")
}
