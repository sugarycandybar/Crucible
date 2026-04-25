"""
IdentityView — Hardware specs display with Copy Specs button.
"""
from __future__ import annotations

import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Adw, Gdk  # noqa: E402

from anvil.backend.hardware import SystemSpecs


class IdentityView(Gtk.ScrolledWindow):
    """Scrollable page showing system hardware specs."""

    def __init__(self, specs: SystemSpecs):
        super().__init__()
        self._specs = specs
        self._toast_fn = None

        self.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)

        # Outer clamp for consistent width
        clamp = Adw.Clamp()
        clamp.set_maximum_size(600)
        clamp.set_margin_top(12)
        clamp.set_margin_bottom(24)
        clamp.set_margin_start(12)
        clamp.set_margin_end(12)

        content = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=24)

        # --- System group ---
        sys_group = Adw.PreferencesGroup(title="System")
        sys_group.add(self._make_row(
            "Operating System",
            f"{specs.os_name}",
            "drive-harddisk-symbolic",
        ))
        sys_group.add(self._make_row(
            "Kernel",
            specs.kernel,
            "application-x-firmware-symbolic",
        ))
        content.append(sys_group)

        # --- Processor group ---
        cpu_group = Adw.PreferencesGroup(title="Processor")
        cpu_group.add(self._make_row(
            "Model",
            specs.cpu_model,
            (
                "processor-symbolic",
                "computer-symbolic",
            ),
        ))
        cpu_group.add(self._make_row(
            "Cores",
            f"{specs.cpu_cores_physical} cores, {specs.cpu_cores_logical} threads",
            "view-grid-symbolic",
        ))
        if specs.cpu_freq_max_ghz > 0:
            cpu_group.add(self._make_row(
                "Max Frequency",
                f"{specs.cpu_freq_max_ghz:.2f} GHz",
                "media-playlist-consecutive-symbolic",
            ))
        content.append(cpu_group)

        # --- Graphics group ---
        gpu_group = Adw.PreferencesGroup(title="Graphics")
        if specs.gpus:
            for i, gpu in enumerate(specs.gpus):
                subtitle_parts = []
                if gpu.vendor:
                    subtitle_parts.append(gpu.vendor)
                if gpu.is_integrated:
                    subtitle_parts.append("Integrated")
                subtitle = " · ".join(subtitle_parts) if subtitle_parts else ""
                gpu_group.add(self._make_row(
                    f"GPU {i}",
                    gpu.name,
                    "video-display-symbolic",
                    subtitle=subtitle,
                ))
        else:
            gpu_group.add(self._make_row(
                "GPU",
                "Unknown GPU",
                "dialog-question-symbolic",
                subtitle="Could not detect a graphics card",
            ))
        content.append(gpu_group)

        # --- Memory group ---
        mem_group = Adw.PreferencesGroup(title="Memory")
        ram_total_gb = specs.ram_total_bytes / 1_000_000_000
        mem_group.add(self._make_row(
            "Total RAM",
            f"{ram_total_gb:.1f} GB",
            "drive-harddisk-solidstate-symbolic",
        ))
        content.append(mem_group)

        # --- Copy Specs button ---
        button_bin = Adw.Bin()
        button_bin.set_margin_top(8)

        copy_button = Gtk.Button(label="Copy Specs")
        copy_button.set_hexpand(True)
        copy_button.set_halign(Gtk.Align.FILL)
        copy_button.add_css_class("suggested-action")
        copy_button.add_css_class("button-row")
        copy_button.set_tooltip_text("Copy hardware specs as plain text")

        copy_button_content = Gtk.Box(
            orientation=Gtk.Orientation.HORIZONTAL,
            spacing=8,
            halign=Gtk.Align.CENTER,
        )
        copy_button_content.append(Gtk.Image.new_from_icon_name("edit-copy-symbolic"))
        copy_button_content.append(Gtk.Label(label="Copy Specs"))
        copy_button.set_child(copy_button_content)

        copy_button.connect("clicked", self._on_copy_clicked)

        button_bin.set_child(copy_button)
        content.append(button_bin)

        clamp.set_child(content)
        self.set_child(clamp)

    def connect_toast(self, toast_fn):
        """Register toast callback from the parent window."""
        self._toast_fn = toast_fn

    @staticmethod
    def _make_row(
        title: str,
        value: str,
        icon_name: str | tuple[str, ...],
        subtitle: str = "",
    ) -> Adw.ActionRow:
        row = Adw.ActionRow(title=title)
        if subtitle:
            row.set_subtitle(subtitle)
        if isinstance(icon_name, str):
            icon_candidates = (icon_name,)
        else:
            icon_candidates = icon_name

        display = Gdk.Display.get_default()
        resolved_icon_name = icon_candidates[-1]
        if display:
            icon_theme = Gtk.IconTheme.get_for_display(display)
            for candidate in icon_candidates:
                if icon_theme.has_icon(candidate):
                    resolved_icon_name = candidate
                    break

        row.add_prefix(Gtk.Image.new_from_icon_name(resolved_icon_name))
        row.add_suffix(Gtk.Label(
            label=value,
            selectable=True,
            css_classes=["dim-label"],
            xalign=1.0,
            wrap=True,
            wrap_mode=2,  # WORD_CHAR
            max_width_chars=28,
        ))
        row.set_activatable(False)
        return row

    def _on_copy_clicked(self, _button):
        text = self._specs.to_plain_text()
        clipboard = Gdk.Display.get_default().get_clipboard()
        clipboard.set(text)
        if self._toast_fn:
            self._toast_fn("Specs copied to clipboard")
