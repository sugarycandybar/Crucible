"""
CrucibleApplication — Main Adw.Application subclass.
Handles app lifecycle, CSS loading, actions, and stress-ng cleanup.
"""
from pathlib import Path

import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Adw, Gio, Gdk  # noqa: E402

from crucible.version import __version__
from crucible.backend.hardware import gather_specs
from crucible.backend.monitor import SystemMonitor
from crucible.backend.stress import StressManager
from crucible.ui.window import CrucibleWindow


APP_ID = "io.github.sugarycandybar.Crucible"
APP_NAME = "Crucible"


class CrucibleApplication(Adw.Application):
    """Main Crucible application."""

    def __init__(self):
        super().__init__(
            application_id=APP_ID,
            flags=Gio.ApplicationFlags.DEFAULT_FLAGS,
        )
        self._window = None
        self._specs = None
        self._monitor = SystemMonitor()
        self._stress = StressManager()

    def do_startup(self):
        """Load CSS, gather hardware specs, register actions."""
        Adw.Application.do_startup(self)
        self._load_css()
        self._register_packaged_icons()
        self._specs = gather_specs()
        self._setup_actions()

    def do_activate(self):
        """Show the main window."""
        if not self._window:
            self._window = CrucibleWindow(
                specs=self._specs,
                monitor=self._monitor,
                stress=self._stress,
                application=self,
            )
        self._window.present()

    def do_shutdown(self):
        """Kill stress-ng if it's still running."""
        self._stress.kill()
        Adw.Application.do_shutdown(self)

    # --- CSS ------------------------------------------------------------

    def _load_css(self):
        css_provider = Gtk.CssProvider()
        css_path = Path(__file__).parent / "style.css"
        if css_path.exists():
            css_provider.load_from_path(str(css_path))
            Gtk.StyleContext.add_provider_for_display(
                Gdk.Display.get_default(),
                css_provider,
                Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION,
            )

    def _register_packaged_icons(self):
        """Make packaged icons discoverable."""
        display = Gdk.Display.get_default()
        if not display:
            return
            
        icon_theme = Gtk.IconTheme.get_for_display(display)
        
        # Load GResource if installed
        for p in (Path(__file__).resolve().parents[1], Path(__file__).resolve().parents[2]):
            resource_path = p / "crucible-resources.gresource"
            if resource_path.exists():
                resource = Gio.Resource.load(str(resource_path))
                Gio.resources_register(resource)
                icon_theme.add_resource_path("/io/github/sugarycandybar/Crucible/icons")
                break

        # Load from source tree during development
        icon_dir = Path(__file__).resolve().parents[2] / "packaging" / "linux"
        if icon_dir.exists():
            icon_theme = Gtk.IconTheme.get_for_display(display)
            icon_theme.add_search_path(str(icon_dir))

    # --- actions --------------------------------------------------------

    def _setup_actions(self):
        action_about = Gio.SimpleAction.new("about", None)
        action_about.connect("activate", self._on_about)
        self.add_action(action_about)

        action_quit = Gio.SimpleAction.new("quit", None)
        action_quit.connect("activate", self._on_quit)
        self.add_action(action_quit)

        self.set_accels_for_action("app.quit", ["<Primary>q"])

    def _on_about(self, _action, _param):
        dialog = Adw.AboutDialog(
            application_name=APP_NAME,
            application_icon=APP_ID,
            version=__version__,
            developer_name="Sugarycandybar",
            website="https://github.com/sugarycandybar/Crucible",
            issue_url="https://github.com/sugarycandybar/Crucible/issues",
            license_type=Gtk.License.GPL_3_0,
            comments="View hardware specs and stress test your system.",
        )
        dialog.present(self._window)

    def _on_quit(self, _action, _param):
        self.quit()
