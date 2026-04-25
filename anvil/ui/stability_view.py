"""
StabilityView — Stress test controls and live monitoring dashboard.
"""
from __future__ import annotations

import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Adw, Gdk  # noqa: E402

from anvil.backend.hardware import SystemSpecs
from anvil.backend.monitor import MonitorSnapshot, SystemMonitor
from anvil.backend.stress import StressManager
from anvil.ui.widgets.temp_gauge import TempGauge


# Duration choices: label → seconds (0 = until stopped)
_DURATIONS = [
    ("5 minutes", 300),
    ("10 minutes", 600),
    ("30 minutes", 1800),
    ("Until stopped", 0),
]

_TEMP_LIMITS_C = [100, 95, 90, 50]


def _pick_icon_name(*candidates: str) -> str:
    """Return the first icon that exists in the current theme."""
    display = Gdk.Display.get_default()
    if display:
        icon_theme = Gtk.IconTheme.get_for_display(display)
        for name in candidates:
            if icon_theme.has_icon(name):
                return name
    return candidates[-1] if candidates else "image-missing"


class StabilityView(Gtk.ScrolledWindow):
    """Stress test dashboard with live monitoring."""

    def __init__(
        self,
        monitor: SystemMonitor,
        stress: StressManager,
        specs: SystemSpecs,
    ):
        super().__init__()
        self._monitor = monitor
        self._stress = stress
        self._specs = specs
        self._toast_fn = None
        self._notification_fn = None
        self._was_running = False

        self.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)

        clamp = Adw.Clamp()
        clamp.set_maximum_size(600)
        clamp.set_margin_top(12)
        clamp.set_margin_bottom(24)
        clamp.set_margin_start(12)
        clamp.set_margin_end(12)

        content = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=24)

        # --- Monitoring group ---
        mon_group = Adw.PreferencesGroup(title="Monitoring")

        # Temperature gauge
        self._temp_gauge = TempGauge()
        mon_group.add(self._temp_gauge)

        # CPU usage row
        self._usage_row = Adw.ActionRow(title="CPU Usage")
        self._usage_row.add_prefix(
            Gtk.Image.new_from_icon_name(
                _pick_icon_name(
                    "utilities-system-monitor-symbolic",
                    "computer-symbolic",
                )
            )
        )
        self._usage_label = Gtk.Label(label="—", css_classes=["dim-label"])
        self._usage_row.add_suffix(self._usage_label)
        self._usage_row.set_activatable(False)
        mon_group.add(self._usage_row)

        # RAM row
        self._ram_row = Adw.ActionRow(title="Memory Usage")
        self._ram_row.add_prefix(
            Gtk.Image.new_from_icon_name("drive-harddisk-solidstate-symbolic")
        )
        self._ram_label = Gtk.Label(label="—", css_classes=["dim-label"])
        self._ram_row.add_suffix(self._ram_label)
        self._ram_row.set_activatable(False)
        mon_group.add(self._ram_row)

        content.append(mon_group)

        # --- Controls group ---
        ctrl_group = Adw.PreferencesGroup(title="Settings")

        # Duration combo row
        self._duration_row = Adw.ComboRow(title="Duration")
        self._duration_row.add_prefix(
            Gtk.Image.new_from_icon_name(
                _pick_icon_name(
                    "timer-symbolic",
                    "alarm-symbolic",
                    "preferences-system-time-symbolic",
                    "computer-symbolic",
                )
            )
        )
        duration_model = Gtk.StringList()
        for label, _ in _DURATIONS:
            duration_model.append(label)
        self._duration_row.set_model(duration_model)
        self._duration_row.set_selected(0)
        ctrl_group.add(self._duration_row)

        self._stop_at_temp_row = Adw.SwitchRow(title="Stop at 95C")
        self._stop_at_temp_row.set_active(True)
        self._stop_at_temp_row.add_prefix(
            Gtk.Image.new_from_icon_name(
                _pick_icon_name(
                    "temperature-high-symbolic",
                    "dialog-warning-symbolic",
                )
            )
        )
        ctrl_group.add(self._stop_at_temp_row)

        self._temp_limit_row = Adw.ComboRow(title="Temperature Limit")
        temp_limit_model = Gtk.StringList()
        for temp_c in _TEMP_LIMITS_C:
            label = f"{temp_c}C"
            if temp_c == 50:
                label += " (test)"
            temp_limit_model.append(label)
        self._temp_limit_row.set_model(temp_limit_model)
        self._temp_limit_row.set_selected(1)
        self._temp_limit_row.connect("notify::selected", self._on_temp_limit_changed)
        ctrl_group.add(self._temp_limit_row)

        content.append(ctrl_group)

        # --- Primary action button ---
        action_bin = Adw.Bin()
        action_bin.set_margin_top(8)

        self._action_button = Gtk.Button(label="Start Test")
        self._action_button.set_hexpand(True)
        self._action_button.set_halign(Gtk.Align.FILL)
        self._action_button.add_css_class("suggested-action")
        self._action_button.add_css_class("button-row")
        self._action_button.connect("clicked", self._on_action_clicked)
        self._action_button.set_tooltip_text(
            "Runs a stress test to check cooling and stability"
        )

        action_bin.set_child(self._action_button)
        content.append(action_bin)

        # --- stress-ng not found banner ---
        if not StressManager.is_available():
            self._action_button.set_sensitive(False)
            self._duration_row.set_sensitive(False)

            banner = Adw.Banner()
            banner.set_title("stress-ng is not installed")
            banner.set_button_label("")
            banner.set_revealed(True)
            # Insert banner at the top
            content.prepend(banner)

        clamp.set_child(content)
        self.set_child(clamp)

    # --- public API -----------------------------------------------------

    def connect_toast(self, toast_fn):
        self._toast_fn = toast_fn

    def connect_notification(self, notification_fn):
        self._notification_fn = notification_fn

    @property
    def overheat_cutoff_enabled(self) -> bool:
        return self._stop_at_temp_row.get_active()

    @property
    def overheat_cutoff_c(self) -> int:
        return _TEMP_LIMITS_C[self._temp_limit_row.get_selected()]

    def update(self, snapshot: MonitorSnapshot):
        """Called every poll tick with fresh sensor data."""
        self._temp_gauge.set_temperature(snapshot.cpu_temp_c)
        self._usage_label.set_label(f"{snapshot.cpu_usage_pct:.0f} %")

        used_gb = snapshot.ram_used_bytes / 1_000_000_000
        total_gb = snapshot.ram_total_bytes / 1_000_000_000
        self._ram_label.set_label(f"{used_gb:.1f} / {total_gb:.1f} GB")

        if (
            self._stress.is_running
            and self.overheat_cutoff_enabled
            and snapshot.cpu_temp_c is not None
            and snapshot.cpu_temp_c >= self.overheat_cutoff_c
        ):
            self._stress.stop(cause="overheat")

        running_now = self._stress.is_running
        if self._was_running and not running_now and self._notification_fn:
            elapsed = self._format_elapsed(self._stress.last_elapsed_seconds)
            if self._stress.last_stop_cause == "overheat":
                self._notification_fn("Test stopped to prevent overheating", f"Elapsed Time: {elapsed}")
            elif self._stress.last_stop_cause == "completed":
                self._notification_fn("Test completed successfully", f"Elapsed Time: {elapsed}")

        self._was_running = running_now

        # Keep button state in sync (process may exit on its own)
        self.refresh_button_state()

    def refresh_button_state(self):
        """Sync button label and style with current stress state."""
        if self._stress.is_running:
            elapsed = int(self._stress.elapsed_seconds)
            mins, secs = divmod(elapsed, 60)
            self._action_button.set_label(f"Stop Test  {mins:02d}:{secs:02d}")
            self._action_button.remove_css_class("suggested-action")
            self._action_button.add_css_class("destructive-action")
            self._duration_row.set_sensitive(False)
        else:
            self._action_button.set_label("Start Test")
            self._action_button.remove_css_class("destructive-action")
            self._action_button.add_css_class("suggested-action")
            if StressManager.is_available():
                self._duration_row.set_sensitive(True)

    # --- callbacks ------------------------------------------------------

    def _on_action_clicked(self, _button):
        if self._stress.is_running:
            self._stress.stop()
            self._was_running = False
            self.refresh_button_state()
            if self._toast_fn:
                self._toast_fn("Test stopped")
            return

        _, duration_secs = _DURATIONS[self._duration_row.get_selected()]

        ok = self._stress.start(duration_seconds=duration_secs)
        if ok:
            self._was_running = True
            self.refresh_button_state()
            if self._toast_fn:
                self._toast_fn("Test started")
        else:
            if self._toast_fn:
                self._toast_fn("Could not start stress-ng")

    def _on_temp_limit_changed(self, *_args):
        self._stop_at_temp_row.set_title(f"Stop at {self.overheat_cutoff_c}C")

    @staticmethod
    def _format_elapsed(total_seconds: float) -> str:
        elapsed = max(0, int(total_seconds))
        mins, secs = divmod(elapsed, 60)
        return f"{mins:02d}:{secs:02d}"
