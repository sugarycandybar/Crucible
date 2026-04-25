"""
FreqGraph — Live CPU frequency line graph using Cairo.
"""
from __future__ import annotations

import math

import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Gdk  # noqa: E402


class FreqGraph(Gtk.DrawingArea):
    """Draws a smooth line graph of CPU frequency history."""

    def __init__(self):
        super().__init__()
        self._data: list[float] = []
        self.set_content_width(400)
        self.set_content_height(140)
        self.add_css_class("freq-graph")
        self.set_draw_func(self._draw)

    def set_data(self, freq_history: list[float]):
        """Update the data buffer and request a redraw."""
        self._data = freq_history
        self.queue_draw()

    def _draw(self, area, cr, width, height):
        """Cairo draw callback."""
        if not self._data or width < 2 or height < 2:
            self._draw_empty(cr, width, height)
            return

        data = self._data
        n = len(data)

        # Determine Y range
        min_val = min(data) if data else 0
        max_val = max(data) if data else 1
        if max_val - min_val < 100:
            mid = (max_val + min_val) / 2
            min_val = mid - 50
            max_val = mid + 50
        min_val = max(0, min_val)

        padding_top = 24
        padding_bottom = 24
        padding_left = 8
        padding_right = 8
        plot_w = width - padding_left - padding_right
        plot_h = height - padding_top - padding_bottom

        if plot_w <= 0 or plot_h <= 0:
            return

        # Resolve theme colors
        style = self.get_style_context()
        fg_ok, fg_color = style.lookup_color("accent_bg_color")
        if not fg_ok:
            fg_color = Gdk.RGBA()
            fg_color.parse("#3584e4")

        text_ok, text_color = style.lookup_color("window_fg_color")
        if not text_ok:
            text_color = Gdk.RGBA()
            text_color.parse("#ffffff")

        # Build point list
        def to_x(i):
            return padding_left + (i / max(n - 1, 1)) * plot_w

        def to_y(v):
            frac = (v - min_val) / (max_val - min_val) if max_val > min_val else 0.5
            return padding_top + plot_h * (1 - frac)

        # --- gradient fill under the line ---
        cr.move_to(to_x(0), to_y(data[0]))
        for i in range(1, n):
            cr.line_to(to_x(i), to_y(data[i]))
        # Close path along the bottom
        cr.line_to(to_x(n - 1), padding_top + plot_h)
        cr.line_to(to_x(0), padding_top + plot_h)
        cr.close_path()

        import cairo
        grad = cairo.LinearGradient(0, padding_top, 0, padding_top + plot_h)
        grad.add_color_stop_rgba(0, fg_color.red, fg_color.green, fg_color.blue, 0.25)
        grad.add_color_stop_rgba(1, fg_color.red, fg_color.green, fg_color.blue, 0.02)
        cr.set_source(grad)
        cr.fill()

        # --- line ---
        cr.set_line_width(2)
        cr.set_source_rgba(fg_color.red, fg_color.green, fg_color.blue, 0.9)
        cr.move_to(to_x(0), to_y(data[0]))
        for i in range(1, n):
            cr.line_to(to_x(i), to_y(data[i]))
        cr.stroke()

        # --- labels ---
        cr.set_source_rgba(
            text_color.red, text_color.green, text_color.blue, 0.6
        )
        cr.select_font_face("Sans", 0, 0)
        cr.set_font_size(10)

        # Current value (top right)
        current = data[-1]
        label = f"{current:.0f} MHz"
        cr.move_to(width - padding_right - 60, padding_top - 6)
        cr.show_text(label)

        # Min (bottom left)
        cr.move_to(padding_left, padding_top + plot_h + 14)
        cr.show_text(f"{min_val:.0f}")

        # Max (top left)
        cr.move_to(padding_left, padding_top - 6)
        cr.show_text(f"{max_val:.0f}")

    def _draw_empty(self, cr, width, height):
        """Draw placeholder when there is no data."""
        style = self.get_style_context()
        ok, color = style.lookup_color("window_fg_color")
        if not ok:
            color = Gdk.RGBA()
            color.parse("#888888")

        cr.set_source_rgba(color.red, color.green, color.blue, 0.3)
        cr.select_font_face("Sans", 0, 0)
        cr.set_font_size(12)

        text = "Waiting for data…"
        extents = cr.text_extents(text)
        cr.move_to(
            (width - extents.width) / 2,
            (height + extents.height) / 2,
        )
        cr.show_text(text)
