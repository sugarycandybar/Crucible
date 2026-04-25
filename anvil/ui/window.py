"""
AnvilWindow — Main window with ViewStack for Identity and Stability views.
"""
import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Adw, Gio, GLib, GObject, Gdk  # noqa: E402

from anvil.backend.hardware import SystemSpecs
from anvil.backend.monitor import SystemMonitor
from anvil.backend.stress import StressManager
from anvil.ui.identity_view import IdentityView
from anvil.ui.stability_view import StabilityView


def _pick_icon_name(*candidates: str) -> str:
    """Return the first icon that exists in the active icon theme."""
    display = Gdk.Display.get_default()
    if display:
        icon_theme = Gtk.IconTheme.get_for_display(display)
        for name in candidates:
            if icon_theme.has_icon(name):
                return name
    return candidates[-1] if candidates else "image-missing"


class AnvilWindow(Adw.ApplicationWindow):
    """Main Anvil window with two views."""

    def __init__(
        self,
        specs: SystemSpecs,
        monitor: SystemMonitor,
        stress: StressManager,
        **kwargs,
    ):
        super().__init__(**kwargs)
        self._monitor = monitor
        self._stress = stress

        self.set_title("Anvil")
        self.set_default_size(520, 720)
        self.set_size_request(360, 480)

        # --- Toast overlay wraps everything ---
        self._toast_overlay = Adw.ToastOverlay()

        # --- Main toolbar/content layout ---
        self._toolbar_view = Adw.ToolbarView()

        # --- Header bar with view switcher ---
        header = Adw.HeaderBar()
        header.add_css_class("flat")

        self._view_switcher_title = Adw.ViewSwitcherTitle()
        header.set_title_widget(self._view_switcher_title)

        # Menu button
        menu_button = Gtk.MenuButton()
        menu_button.set_icon_name("open-menu-symbolic")
        menu_model = Gio.Menu()
        menu_model.append("About Anvil", "app.about")
        menu_model.append("Quit", "app.quit")
        menu_button.set_menu_model(menu_model)
        header.pack_end(menu_button)

        self._toolbar_view.add_top_bar(header)

        # --- View stack ---
        self._view_stack = Adw.ViewStack()
        self._view_stack.set_vexpand(True)

        # Identity view (specs)
        self._identity_view = IdentityView(specs)
        self._identity_view.connect_toast(self.show_toast)
        self._view_stack.add_titled_with_icon(
            self._identity_view, "identity", "Specs", "computer-symbolic"
        )

        # Stability view (stress test)
        self._stability_view = StabilityView(monitor, stress, specs)
        self._stability_view.connect_toast(self.show_toast)
        self._stability_view.connect_notification(self.show_system_notification)
        self._view_stack.add_titled_with_icon(
            self._stability_view,
            "stability",
            "Stress Test",
            _pick_icon_name(
                "speedometer-symbolic",
                "system-run-symbolic",
                "media-playback-start-symbolic",
                "computer-symbolic",
            ),
        )

        self._view_switcher_title.set_stack(self._view_stack)
        self._toolbar_view.set_content(self._view_stack)

        # --- Bottom switcher bar for narrow windows ---
        self._switcher_bar = Adw.ViewSwitcherBar()
        self._switcher_bar.set_stack(self._view_stack)
        self._toolbar_view.add_bottom_bar(self._switcher_bar)

        # Bind the title's title-visible to the bar's reveal
        self._view_switcher_title.bind_property(
            "title-visible",
            self._switcher_bar,
            "reveal",
            GObject.BindingFlags.SYNC_CREATE,
        )

        self._toast_overlay.set_child(self._toolbar_view)
        self.set_content(self._toast_overlay)

        # --- Poll timer (1 Hz) ---
        self._poll_id = GLib.timeout_add(1000, self._on_poll)

    # --- toast ----------------------------------------------------------

    def show_toast(self, message: str, timeout: int = 3):
        """Show a toast notification."""
        toast = Adw.Toast(title=message)
        toast.set_timeout(max(1, int(timeout)))
        self._toast_overlay.add_toast(toast)

    def show_system_notification(self, title: str, body: str | None = None):
        """Show a desktop/system notification via Gio.Application."""
        app = self.get_application()
        if not app:
            self.show_toast(title)
            return

        notification = Gio.Notification.new(title)
        if body:
            notification.set_body(body)

        notification_id = f"stress-{GLib.get_monotonic_time()}"
        app.send_notification(notification_id, notification)

    # --- polling --------------------------------------------------------

    def _on_poll(self) -> bool:
        snapshot = self._monitor.poll()
        self._stability_view.update(snapshot)

        return True

    def do_close_request(self):
        """Ensure stress-ng is killed when the window closes."""
        if self._poll_id:
            GLib.source_remove(self._poll_id)
            self._poll_id = None
        self._stress.kill()
        return Adw.ApplicationWindow.do_close_request(self)
