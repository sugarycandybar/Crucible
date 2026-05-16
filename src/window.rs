use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::backend::hardware::SystemSpecs;
use crate::backend::monitor::SystemMonitor;
use crate::backend::stress::StressManager;
use crate::identity_view;
use crate::stability_view;

pub fn create_window(
    specs: SystemSpecs,
    monitor: Rc<RefCell<SystemMonitor>>,
    app: &libadwaita::Application,
) -> (
    libadwaita::ApplicationWindow,
    Rc<RefCell<stability_view::StabilityState>>,
) {
    let window = libadwaita::ApplicationWindow::new(app);
    window.set_title(Some("Crucible"));
    window.set_default_width(520);
    window.set_default_height(600);
    window.set_size_request(360, 480);

    let toast_overlay = libadwaita::ToastOverlay::new();
    let toolbar_view = libadwaita::ToolbarView::new();

    // Header bar
    let header = libadwaita::HeaderBar::new();
    header.add_css_class("flat");

    let view_switcher = libadwaita::ViewSwitcher::new();
    view_switcher.set_policy(libadwaita::ViewSwitcherPolicy::Wide);
    header.set_title_widget(Some(&view_switcher));

    // Menu button
    let menu_button = gtk4::MenuButton::new();
    menu_button.set_icon_name("open-menu-symbolic");
    let menu = gio::Menu::new();
    menu.append(Some("About Crucible"), Some("app.about"));
    menu.append(Some("Quit"), Some("app.quit"));
    menu_button.set_menu_model(Some(&menu));
    header.pack_end(&menu_button);

    toolbar_view.add_top_bar(&header);

    // View stack
    let view_stack = libadwaita::ViewStack::new();
    view_stack.set_vexpand(true);

    // Identity view (specs)
    let toast_for_identity = toast_overlay.clone();
    let show_toast_identity = Box::new(move |msg: String| {
        let toast = libadwaita::Toast::new(&msg);
        toast.set_timeout(3);
        toast_for_identity.add_toast(toast);
    });

    let identity_scrolled =
        identity_view::create_identity_view(&specs, show_toast_identity);
    view_stack.add_titled_with_icon(
        &identity_scrolled,
        Some("identity"),
        "Specs",
        "computer-symbolic",
    );

    // Stability view (stress test)
    let stress = StressManager::new();
    let (stability_scrolled, stability_state) =
        stability_view::create_stability_view(stress);

    // Set toast callback
    {
        let toast_for_stability = toast_overlay.clone();
        stability_state.borrow_mut().toast_fn = Some(Box::new(move |msg: String| {
            let toast = libadwaita::Toast::new(&msg);
            toast.set_timeout(3);
            toast_for_stability.add_toast(toast);
        }));
    }

    view_stack.add_titled_with_icon(
        &stability_scrolled,
        Some("stability"),
        "Stress Test",
        "system-run-symbolic",
    );

    view_stack.set_visible_child_name("stability");

    view_switcher.set_stack(Some(&view_stack));
    toolbar_view.set_content(Some(&view_stack));

    toast_overlay.set_child(Some(&toolbar_view));
    window.set_content(Some(&toast_overlay));

    // Poll timer (1 Hz)
    let stability_for_timer = stability_state.clone();
    let monitor_for_timer = monitor.clone();
    glib::timeout_add_seconds_local(1, move || {
        let snapshot = {
            let mut mon = monitor_for_timer.borrow_mut();
            mon.poll()
        };
        stability_for_timer.borrow_mut().update(&snapshot);
        glib::ControlFlow::Continue
    });

    // Kill stress-ng on window close
    let stability_for_close = stability_state.clone();
    window.connect_close_request(move |_| {
        stability_for_close.borrow_mut().stress.kill();
        glib::Propagation::Proceed
    });

    (window, stability_state)
}
