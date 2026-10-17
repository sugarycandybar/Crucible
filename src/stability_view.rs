use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::backend::gpu_stress::GpuStressManager;
use crate::backend::monitor::MonitorSnapshot;
use crate::backend::stress::StressManager;
use crate::ui::widgets::temp_gauge::{
    create_temp_widgets, pick_temperature_icon, set_temp_widgets, TempWidgets,
};

const DURATIONS: &[(u64, &str)] = &[
    (300, "5 minutes"),
    (600, "10 minutes"),
    (1800, "30 minutes"),
    (0, "Until stopped"),
];

const TEMP_LIMITS_C: &[u64] = &[100, 95, 90];
const DISABLED_INDEX: u32 = 3;
const TEMP_LIMIT_LABELS: &[&str] = &["100°C", "95°C", "90°C", "Disabled"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum StressType {
    Cpu,
    Gpu,
    Both,
}

impl StressType {
    fn from_index(i: u32) -> Self {
        match i {
            0 => StressType::Cpu,
            1 => StressType::Gpu,
            _ => StressType::Both,
        }
    }

    #[allow(dead_code)]
    fn to_index(self) -> u32 {
        match self {
            StressType::Cpu => 0,
            StressType::Gpu => 1,
            StressType::Both => 2,
        }
    }
}

pub struct StabilityState {
    pub stress: StressManager,
    pub gpu_stress: GpuStressManager,
    pub was_running: bool,

    pub toggle_group: libadwaita::ToggleGroup,

    // Unified sensors group - no animations, instant switch
    pub sensors_group: libadwaita::PreferencesGroup,
    // Wide layout: a single "Temperature" row holding both gauges side by side.
    pub temp_row: libadwaita::ActionRow,
    pub cpu_wide_column: gtk4::Box,
    pub gpu_wide_column: gtk4::Box,
    pub cpu_wide_widgets: TempWidgets,
    pub gpu_wide_widgets: TempWidgets,
    // Narrow layout: one row per chip so nothing gets cramped.
    pub cpu_temp_row: libadwaita::ActionRow,
    pub gpu_temp_row: libadwaita::ActionRow,
    pub cpu_narrow_widgets: TempWidgets,
    pub gpu_narrow_widgets: TempWidgets,
    pub narrow: bool,

    pub usage_row: libadwaita::ActionRow,
    pub usage_label: gtk4::Label,
    pub usage_icon: gtk4::Image,

    pub memory_row: libadwaita::ActionRow,
    pub memory_label: gtk4::Label,

    // Cached snapshot for instant layout switch
    pub last_snapshot: Option<MonitorSnapshot>,
    // Latched visual hot state to avoid flicker at ~90°C (enter 90, exit 85)
    pub cpu_visual_hot: bool,
    pub gpu_visual_hot: bool,

    // Controls - merged temperature limit (no separate switch)
    pub action_button: gtk4::Button,
    pub duration_row: libadwaita::ComboRow,
    pub temp_limit_row: libadwaita::ComboRow,
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

    fn overheat_cutoff_c(&self) -> Option<f64> {
        let idx = self.temp_limit_row.selected();
        if idx == DISABLED_INDEX {
            None
        } else {
            let i = (idx as usize).min(TEMP_LIMITS_C.len() - 1);
            Some(TEMP_LIMITS_C[i] as f64)
        }
    }

    #[allow(dead_code)]
    fn is_overheat_protection_enabled(&self) -> bool {
        self.overheat_cutoff_c().is_some()
    }

    fn stress_type(&self) -> StressType {
        StressType::from_index(self.toggle_group.active())
    }

    fn any_running(&mut self) -> bool {
        self.stress.is_running() || self.gpu_stress.is_running()
    }

    fn stop_all(&mut self, cause: &str) {
        self.stress.stop(cause);
        self.gpu_stress.stop(cause);
    }

    fn elapsed_display(&mut self) -> i64 {
        let cpu_elapsed = self.stress.elapsed_seconds();
        let gpu_elapsed = self.gpu_stress.elapsed_seconds();
        let elapsed = if cpu_elapsed > 0.0 {
            cpu_elapsed
        } else {
            gpu_elapsed
        };
        (elapsed as i64).max(0)
    }

    fn last_elapsed_display(&mut self) -> f64 {
        let cpu_elapsed = self.stress.last_elapsed_seconds();
        let gpu_elapsed = self.gpu_stress.last_elapsed_seconds();
        if cpu_elapsed > 0.0 {
            cpu_elapsed
        } else {
            gpu_elapsed
        }
    }

    fn refresh_button_state(&mut self) {
        if self.any_running() {
            let elapsed = self.elapsed_display();
            let mins = elapsed / 60;
            let secs = elapsed % 60;
            self.action_button
                .set_label(&format!("Stop Test  {mins:02}:{secs:02}"));
            self.action_button.remove_css_class("suggested-action");
            self.action_button.add_css_class("destructive-action");
            self.duration_row.set_sensitive(false);
        } else {
            let label = match self.stress_type() {
                StressType::Cpu => "Start CPU Test",
                StressType::Gpu => "Start GPU Test",
                StressType::Both => "Start Stress Test",
            };
            self.action_button.set_label(label);
            self.action_button.remove_css_class("destructive-action");
            self.action_button.add_css_class("suggested-action");
            self.duration_row.set_sensitive(true);
        }
    }

    fn refresh_sensor_labels(&mut self, snapshot: &MonitorSnapshot) {
        // Both layouts exist at once (only one is visible), so keep every gauge
        // in sync. Values don't need prefixes - the row title says which chip.
        for widgets in [&self.cpu_wide_widgets, &self.cpu_narrow_widgets] {
            set_temp_widgets(widgets, snapshot.cpu_temp_c);
        }
        for widgets in [&self.gpu_wide_widgets, &self.gpu_narrow_widgets] {
            set_temp_widgets(widgets, snapshot.gpu_temp_c);
        }

        let mode = self.stress_type();
        match mode {
            StressType::Cpu => {
                self.usage_label
                    .set_label(&format!("{:.0} %", snapshot.cpu_usage_pct));
                let ram_used_gb = snapshot.ram_used_bytes as f64 / 1_000_000_000.0;
                let ram_total_gb = snapshot.ram_total_bytes as f64 / 1_000_000_000.0;
                self.memory_label
                    .set_label(&format!("{ram_used_gb:.1} / {ram_total_gb:.1} GB"));
            }
            StressType::Gpu => {
                self.usage_label.set_label(
                    &snapshot
                        .gpu_usage_pct
                        .map(|u| format!("{u:.0} %"))
                        .unwrap_or_else(|| "—".to_string()),
                );
                let vram_used_gb = snapshot.vram_used_bytes as f64 / 1_000_000_000.0;
                let vram_total_gb = snapshot.vram_total_bytes as f64 / 1_000_000_000.0;
                if snapshot.vram_total_bytes > 0 {
                    self.memory_label
                        .set_label(&format!("{vram_used_gb:.1} / {vram_total_gb:.1} GB"));
                } else {
                    self.memory_label.set_label("—");
                }
            }
            StressType::Both => {
                let cpu_s = format!("CPU {:.0}%", snapshot.cpu_usage_pct);
                let gpu_s = snapshot
                    .gpu_usage_pct
                    .map(|u| format!("GPU {u:.0}%"))
                    .unwrap_or_else(|| "GPU —".to_string());
                self.usage_label.set_label(&format!("{cpu_s} · {gpu_s}"));

                let ram_used_gb = snapshot.ram_used_bytes as f64 / 1_000_000_000.0;
                let ram_total_gb = snapshot.ram_total_bytes as f64 / 1_000_000_000.0;
                let ram_s = format!("RAM {ram_used_gb:.1}/{ram_total_gb:.1}");
                let vram_used_gb = snapshot.vram_used_bytes as f64 / 1_000_000_000.0;
                let vram_total_gb = snapshot.vram_total_bytes as f64 / 1_000_000_000.0;
                let vram_s = if snapshot.vram_total_bytes > 0 {
                    format!("VRAM {vram_used_gb:.1}/{vram_total_gb:.1}")
                } else {
                    "VRAM —".to_string()
                };
                self.memory_label.set_label(&format!("{ram_s} · {vram_s}"));
            }
        }

        // Visual hot is fixed at 90°C (not Temperature Limit) with hysteresis
        // enter 90, exit 85 to avoid split-second flicker when hovering around limit
        const CRITICAL: f64 = 90.0;
        const HYSTERESIS_EXIT: f64 = 85.0;
        let update_latched = |latched: &mut bool, temp: Option<f64>| {
            if let Some(t) = temp {
                if !*latched && t >= CRITICAL {
                    *latched = true;
                } else if *latched && t < HYSTERESIS_EXIT {
                    *latched = false;
                }
            } else if *latched {
                // No sensor - clear
                *latched = false;
            }
        };
        update_latched(&mut self.cpu_visual_hot, snapshot.cpu_temp_c);
        update_latched(&mut self.gpu_visual_hot, snapshot.gpu_temp_c);

        // Keep text red via latched state (no warning icon per request), and
        // override the LevelBar color to match so the bar stays red instead of
        // flickering blue when temp hovers around 90 or cutoff.
        for (latched, widgets) in [
            (self.cpu_visual_hot, &self.cpu_wide_widgets),
            (self.cpu_visual_hot, &self.cpu_narrow_widgets),
            (self.gpu_visual_hot, &self.gpu_wide_widgets),
            (self.gpu_visual_hot, &self.gpu_narrow_widgets),
        ] {
            if latched {
                widgets.label.add_css_class("error");
                widgets.bar.remove_css_class("temp-low");
                widgets.bar.remove_css_class("temp-high");
                widgets.bar.remove_css_class("temp-full");
                widgets.bar.add_css_class("temp-full");
            } else {
                widgets.label.remove_css_class("error");
            }
        }
    }

    fn apply_monitor_layout(&mut self) {
        let mode = self.stress_type();
        let narrow = self.narrow;

        // Wide combined row: two gauges share the row, so they're a touch
        // narrower in Both. Narrow rows hold a single gauge each, so they can
        // use the full width without cramping.
        let wide_width = if matches!(mode, StressType::Both) {
            96
        } else {
            120
        };
        self.cpu_wide_widgets.bar.set_size_request(wide_width, -1);
        self.gpu_wide_widgets.bar.set_size_request(wide_width, -1);
        self.cpu_narrow_widgets.bar.set_size_request(120, -1);
        self.gpu_narrow_widgets.bar.set_size_request(120, -1);

        // Group title always "Sensors" per user feedback
        self.sensors_group.set_title("Sensors");

        // Wide combined row title, and which of its two gauges are shown.
        match mode {
            StressType::Cpu => {
                self.temp_row.set_title("CPU Temperature");
                self.cpu_wide_column.set_visible(true);
                self.gpu_wide_column.set_visible(false);
            }
            StressType::Gpu => {
                self.temp_row.set_title("GPU Temperature");
                self.cpu_wide_column.set_visible(false);
                self.gpu_wide_column.set_visible(true);
            }
            StressType::Both => {
                self.temp_row.set_title("Temperature");
                self.cpu_wide_column.set_visible(true);
                self.gpu_wide_column.set_visible(true);
            }
        }

        // Wide keeps the combined row; narrow splits it into one row per chip.
        self.temp_row.set_visible(!narrow);
        self.cpu_temp_row
            .set_visible(narrow && !matches!(mode, StressType::Gpu));
        self.gpu_temp_row
            .set_visible(narrow && !matches!(mode, StressType::Cpu));

        match mode {
            StressType::Cpu => {
                self.usage_row.set_title("CPU Usage");
                self.memory_row.set_title("Memory Usage");
                self.usage_icon.set_icon_name(Some("processor-symbolic"));
            }
            StressType::Gpu => {
                self.usage_row.set_title("GPU Usage");
                self.memory_row.set_title("VRAM Usage");
                self.usage_icon.set_icon_name(Some("pci-card-symbolic"));
            }
            StressType::Both => {
                self.usage_row.set_title("Processor Load");
                self.memory_row.set_title("Memory");
                self.usage_icon.set_icon_name(Some("processor-symbolic"));
            }
        }

        if let Some(snapshot) = self.last_snapshot.clone() {
            self.refresh_sensor_labels(&snapshot);
        }
        // Update Start label when switching targets while idle
        if !self.any_running() {
            self.refresh_button_state();
        }
    }

    /// Called by the window's breakpoint when it crosses the narrow threshold.
    pub fn set_narrow(&mut self, narrow: bool) {
        if self.narrow == narrow {
            return;
        }
        self.narrow = narrow;
        self.apply_monitor_layout();
    }

    pub fn update(&mut self, snapshot: &MonitorSnapshot) {
        self.last_snapshot = Some(snapshot.clone());
        self.refresh_sensor_labels(snapshot);

        if let Some(cutoff) = self.overheat_cutoff_c() {
            match self.stress_type() {
                StressType::Cpu => {
                    if self.stress.is_running()
                        && snapshot.cpu_temp_c.is_some()
                        && snapshot.cpu_temp_c.unwrap() >= cutoff
                    {
                        self.stress.stop("overheat");
                    }
                }
                StressType::Gpu => {
                    if self.gpu_stress.is_running()
                        && snapshot.gpu_temp_c.is_some()
                        && snapshot.gpu_temp_c.unwrap() >= cutoff
                    {
                        self.gpu_stress.stop("overheat");
                    }
                }
                StressType::Both => {
                    let cpu_over = self.stress.is_running()
                        && snapshot.cpu_temp_c.is_some()
                        && snapshot.cpu_temp_c.unwrap() >= cutoff;
                    let gpu_over = self.gpu_stress.is_running()
                        && snapshot.gpu_temp_c.is_some()
                        && snapshot.gpu_temp_c.unwrap() >= cutoff;
                    if cpu_over || gpu_over {
                        self.stop_all("overheat");
                    }
                }
            }
        }

        let running_now = self.any_running();
        if self.was_running && !running_now {
            let elapsed = format_elapsed(self.last_elapsed_display());
            let cpu_cause = self.stress.last_stop_cause();
            let gpu_cause = self.gpu_stress.last_stop_cause();
            let cause = cpu_cause.or(gpu_cause);
            match cause {
                Some("overheat") => {
                    // Determine which device overheated from the snapshot that triggered the stop
                    let cutoff = self.overheat_cutoff_c();
                    let cpu_hot = cutoff.zip(snapshot.cpu_temp_c).is_some_and(|(c, t)| t >= c);
                    let gpu_hot = cutoff.zip(snapshot.gpu_temp_c).is_some_and(|(c, t)| t >= c);
                    let device = match (cpu_hot, gpu_hot) {
                        (true, true) => "CPU/GPU",
                        (true, false) => "CPU",
                        (false, true) => "GPU",
                        _ => match self.stress_type() {
                            StressType::Cpu => "CPU",
                            StressType::Gpu => "GPU",
                            StressType::Both => "CPU/GPU",
                        },
                    };
                    self.send_desktop_notification(&format!(
                        "Test stopped to prevent {device} overheating (Elapsed: {elapsed})"
                    ));
                }
                Some("completed") => {
                    self.send_desktop_notification(&format!(
                        "Test completed successfully (Elapsed: {elapsed})"
                    ));
                }
                Some("error") => {
                    let reason = self
                        .gpu_stress
                        .last_error()
                        .unwrap_or("unknown error")
                        .to_string();
                    self.send_desktop_notification(&format!("GPU test failed: {reason}"));
                }
                _ => {
                    self.send_desktop_notification(&format!("Test stopped (Elapsed: {elapsed})"));
                }
            }
        }

        self.was_running = running_now;
        self.refresh_button_state();
    }
}

pub fn create_stability_view(
    stress: StressManager,
    gpu_stress: GpuStressManager,
    app: Option<libadwaita::Application>,
) -> (gtk4::ScrolledWindow, Rc<RefCell<StabilityState>>) {
    let settings = gio::Settings::new("io.github.sugarycandybar.Crucible.Stability");

    let scrolled = gtk4::ScrolledWindow::new();
    scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let clamp = libadwaita::Clamp::new();
    clamp.set_maximum_size(600);
    clamp.set_margin_top(8);
    clamp.set_margin_bottom(16);
    clamp.set_margin_start(12);
    clamp.set_margin_end(12);

    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 8);

    let toggle_group = libadwaita::ToggleGroup::new();
    toggle_group.set_halign(gtk4::Align::Center);
    toggle_group.set_homogeneous(true);
    toggle_group.add_css_class("round");

    let cpu_tog = libadwaita::Toggle::new();
    cpu_tog.set_label(Some("CPU"));
    toggle_group.add(cpu_tog);

    let gpu_tog = libadwaita::Toggle::new();
    gpu_tog.set_label(Some("GPU"));
    toggle_group.add(gpu_tog);

    let both_tog = libadwaita::Toggle::new();
    both_tog.set_label(Some("Both"));
    toggle_group.add(both_tog);

    content.append(&toggle_group);

    let sensors_group = libadwaita::PreferencesGroup::new();
    sensors_group.set_title("Sensors");

    let temp_icon_name = pick_temperature_icon();

    // Wide layout: a single "Temperature" row with both gauges side by side.
    let temp_row = libadwaita::ActionRow::new();
    temp_row.set_title("CPU Temperature");
    temp_row.set_activatable(false);
    temp_row.add_prefix(&gtk4::Image::from_icon_name(&temp_icon_name));

    let temp_suffix = gtk4::Box::new(gtk4::Orientation::Horizontal, 16);
    temp_suffix.set_valign(gtk4::Align::Center);
    temp_suffix.set_halign(gtk4::Align::End);

    let cpu_wide_column = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    cpu_wide_column.set_valign(gtk4::Align::Center);
    cpu_wide_column.set_halign(gtk4::Align::End);
    let cpu_wide_widgets = create_temp_widgets(false);
    cpu_wide_column.append(&cpu_wide_widgets.container);

    let gpu_wide_column = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    gpu_wide_column.set_valign(gtk4::Align::Center);
    gpu_wide_column.set_halign(gtk4::Align::End);
    let gpu_wide_widgets = create_temp_widgets(false);
    gpu_wide_column.append(&gpu_wide_widgets.container);

    temp_suffix.append(&cpu_wide_column);
    temp_suffix.append(&gpu_wide_column);
    temp_row.add_suffix(&temp_suffix);
    sensors_group.add(&temp_row);

    // Narrow layout: one row per chip (hidden until the breakpoint flips).
    let cpu_temp_row = libadwaita::ActionRow::new();
    cpu_temp_row.set_title("CPU Temperature");
    cpu_temp_row.set_activatable(false);
    cpu_temp_row.add_prefix(&gtk4::Image::from_icon_name(&temp_icon_name));
    let cpu_narrow_widgets = create_temp_widgets(false);
    cpu_temp_row.add_suffix(&cpu_narrow_widgets.container);
    cpu_temp_row.set_visible(false);
    sensors_group.add(&cpu_temp_row);

    let gpu_temp_row = libadwaita::ActionRow::new();
    gpu_temp_row.set_title("GPU Temperature");
    gpu_temp_row.set_activatable(false);
    gpu_temp_row.add_prefix(&gtk4::Image::from_icon_name(&temp_icon_name));
    let gpu_narrow_widgets = create_temp_widgets(false);
    gpu_temp_row.add_suffix(&gpu_narrow_widgets.container);
    gpu_temp_row.set_visible(false);
    sensors_group.add(&gpu_temp_row);

    let usage_row = libadwaita::ActionRow::new();
    usage_row.set_title("CPU Usage");
    usage_row.set_activatable(false);
    let usage_icon = gtk4::Image::from_icon_name("processor-symbolic");
    usage_row.add_prefix(&usage_icon);
    let usage_label = gtk4::Label::new(Some("—"));
    usage_label.add_css_class("dim-label");
    usage_label.set_xalign(1.0);
    usage_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    usage_row.add_suffix(&usage_label);
    sensors_group.add(&usage_row);

    let memory_row = libadwaita::ActionRow::new();
    memory_row.set_title("Memory Usage");
    memory_row.set_activatable(false);
    memory_row.add_prefix(&gtk4::Image::from_icon_name("memory-symbolic"));
    let memory_label = gtk4::Label::new(Some("—"));
    memory_label.add_css_class("dim-label");
    memory_label.set_xalign(1.0);
    memory_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    memory_row.add_suffix(&memory_label);
    sensors_group.add(&memory_row);

    content.append(&sensors_group);

    let ctrl_group = libadwaita::PreferencesGroup::new();
    ctrl_group.set_title("Settings");
    ctrl_group.set_margin_top(8);

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

    let temp_limit_row = libadwaita::ComboRow::new();
    temp_limit_row.set_title("Temperature Limit");
    temp_limit_row.add_prefix(&gtk4::Image::from_icon_name("stop-sign-large-symbolic"));
    let temp_limit_model = gtk4::StringList::new(&[]);
    for label in TEMP_LIMIT_LABELS {
        temp_limit_model.append(label);
    }
    temp_limit_row.set_model(Some(&temp_limit_model));
    let mut initial_temp_idx = settings.uint("temp-limit-index");
    if initial_temp_idx > DISABLED_INDEX {
        initial_temp_idx = DISABLED_INDEX;
    }
    let stop_enabled = settings.boolean("stop-at-temp");
    if !stop_enabled {
        initial_temp_idx = DISABLED_INDEX;
    }
    temp_limit_row.set_selected(initial_temp_idx);
    ctrl_group.add(&temp_limit_row);

    content.append(&ctrl_group);

    let action_bin = libadwaita::Bin::new();
    action_bin.set_margin_top(4);

    let action_button = gtk4::Button::with_label("Start Test");
    action_button.set_hexpand(true);
    action_button.set_halign(gtk4::Align::Fill);
    action_button.add_css_class("suggested-action");
    action_button.add_css_class("button-row");
    action_button.set_tooltip_text(Some("Runs a stress test to check cooling and stability"));
    action_bin.set_child(Some(&action_button));
    content.append(&action_bin);

    if !StressManager::is_available() {
        let banner =
            libadwaita::Banner::new("stress-ng is not installed — CPU stress not available");
        banner.set_button_label(Some(""));
        banner.set_revealed(true);
        content.prepend(&banner);
    }
    if !GpuStressManager::is_available() {
        let banner = libadwaita::Banner::new("Vulkan not available — GPU stress not available");
        banner.set_button_label(Some(""));
        banner.set_revealed(true);
        content.prepend(&banner);
    }

    clamp.set_child(Some(&content));
    scrolled.set_child(Some(&clamp));

    let state = Rc::new(RefCell::new(StabilityState {
        stress,
        gpu_stress,
        was_running: false,
        toggle_group: toggle_group.clone(),
        sensors_group: sensors_group.clone(),
        temp_row: temp_row.clone(),
        cpu_wide_column: cpu_wide_column.clone(),
        gpu_wide_column: gpu_wide_column.clone(),
        cpu_wide_widgets,
        gpu_wide_widgets,
        cpu_temp_row: cpu_temp_row.clone(),
        gpu_temp_row: gpu_temp_row.clone(),
        cpu_narrow_widgets,
        gpu_narrow_widgets,
        narrow: false,
        usage_row: usage_row.clone(),
        usage_label: usage_label.clone(),
        usage_icon: usage_icon.clone(),
        memory_row: memory_row.clone(),
        memory_label: memory_label.clone(),
        last_snapshot: None,
        cpu_visual_hot: false,
        gpu_visual_hot: false,
        action_button: action_button.clone(),
        duration_row: duration_row.clone(),
        temp_limit_row: temp_limit_row.clone(),
        app,
    }));

    toggle_group.set_active(settings.uint("stress-type-index"));

    toggle_group.connect_active_notify({
        let state_clone = state.clone();
        move |tg| {
            state_clone.borrow_mut().apply_monitor_layout();
            let idx = tg.active();
            glib::idle_add_local(move || {
                let s = gio::Settings::new("io.github.sugarycandybar.Crucible.Stability");
                let _ = s.set_uint("stress-type-index", idx);
                glib::ControlFlow::Break
            });
        }
    });

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
            let enabled = val != DISABLED_INDEX;
            let _ = s.set_boolean("stop-at-temp", enabled);
            glib::ControlFlow::Break
        });
    });

    let sig_state = state.clone();
    action_button.connect_clicked(move |_| {
        let mut st = sig_state.borrow_mut();

        if st.any_running() {
            st.stop_all("manual");
            let elapsed = format_elapsed(st.last_elapsed_display());
            st.send_desktop_notification(&format!("Test stopped manually (Elapsed: {elapsed})"));
            st.was_running = false;
            st.refresh_button_state();
            return;
        }

        let idx = st.duration_row.selected();
        let duration = DURATIONS[idx as usize].0;

        let ok = match st.stress_type() {
            StressType::Cpu => st.stress.start(duration),
            StressType::Gpu => st.gpu_stress.start(duration),
            StressType::Both => {
                let cpu_ok = st.stress.start(duration);
                let gpu_ok = st.gpu_stress.start(duration);
                cpu_ok || gpu_ok
            }
        };

        if ok {
            st.was_running = true;
            st.refresh_button_state();
        } else {
            st.send_desktop_notification("Could not start test");
        }
    });

    state.borrow_mut().apply_monitor_layout();

    (scrolled, state)
}

fn format_elapsed(total_seconds: f64) -> String {
    let elapsed = (total_seconds as i64).max(0);
    let mins = elapsed / 60;
    let secs = elapsed % 60;
    format!("{mins:02}:{secs:02}")
}
