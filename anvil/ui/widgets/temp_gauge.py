"""
TempGauge — Temperature display with a LevelBar.
"""
import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Adw, Gdk  # noqa: E402


def _pick_icon_name(*candidates: str) -> str:
    """Return the first icon that exists in the active icon theme."""
    display = Gdk.Display.get_default()
    if display:
        icon_theme = Gtk.IconTheme.get_for_display(display)
        for name in candidates:
            if icon_theme.has_icon(name):
                return name
    return candidates[-1] if candidates else "image-missing"


class TempGauge(Adw.ActionRow):
    """ActionRow showing CPU temperature with a coloured LevelBar."""

    def __init__(self):
        super().__init__(title="CPU Temperature")
        self.set_activatable(False)
        self.add_prefix(
            Gtk.Image.new_from_icon_name(
                _pick_icon_name(
                    "sensors-temperature-symbolic",
                    "temperature-high-symbolic",
                    "weather-clear-symbolic",
                    "dialog-information-symbolic",
                )
            )
        )

        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        box.set_valign(Gtk.Align.CENTER)

        # Numeric label
        self._label = Gtk.Label(label="—", css_classes=["dim-label"])
        self._label.set_width_chars(5)
        self._label.set_xalign(1.0)
        box.append(self._label)

        # Level bar
        self._bar = Gtk.LevelBar()
        self._bar.set_min_value(0)
        self._bar.set_max_value(105)
        self._bar.set_value(0)
        self._bar.set_hexpand(False)
        self._bar.set_size_request(120, -1)
        self._bar.set_valign(Gtk.Align.CENTER)
        self._bar.add_css_class("temp-gauge")

        # Named offsets for colour thresholds
        self._bar.add_offset_value("low", 55)
        self._bar.add_offset_value("high", 75)
        self._bar.add_offset_value("full", 90)

        box.append(self._bar)

        self.add_suffix(box)

    def set_temperature(self, temp_c: float | None):
        """Update the gauge with a new temperature reading."""
        if temp_c is None:
            self._label.set_label("N/A")
            self._bar.set_value(0)
            return

        self._label.set_label(f"{temp_c:.0f} °C")
        self._bar.set_value(min(temp_c, 105))
