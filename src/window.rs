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
    menu_button.set_tooltip_text(Some("Main Menu"));
    let menu = gio::Menu::new();
    menu.append(Some("Keyboard Shortcuts"), Some("win.show-shortcuts"));
    menu.append(Some("About Crucible"), Some("app.about"));
    menu_button.set_menu_model(Some(&menu));
    header.pack_end(&menu_button);

    let menu_action = gio::SimpleAction::new("menu", None);
    {
        let menu_button = menu_button.clone();
        menu_action.connect_activate(move |_, _| {
            menu_button.popup();
        });
    }
    window.add_action(&menu_action);

    let show_shortcuts_action = gio::SimpleAction::new("show-shortcuts", None);
    {
        let window = window.clone();
        show_shortcuts_action.connect_activate(move |_, _| {
            let shortcuts = libadwaita::ShortcutsDialog::builder()
                .title("Keyboard Shortcuts")
                .build();

            let section = libadwaita::ShortcutsSection::new(Some("General"));
            section.add(libadwaita::ShortcutsItem::from_action(
                "Open Menu",
                "win.menu",
            ));
            section.add(libadwaita::ShortcutsItem::from_action(
                "Keyboard Shortcuts",
                "win.show-shortcuts",
            ));
            section.add(libadwaita::ShortcutsItem::from_action(
                "Close Window",
                "app.quit",
            ));
            shortcuts.add(section);
            shortcuts.present(Some(&window));
        });
    }
    window.add_action(&show_shortcuts_action);

    toolbar_view.add_top_bar(&header);

    // View stack
    let view_stack = libadwaita::ViewStack::new();
    view_stack.set_vexpand(true);

    // Identity view (specs)
    let identity_scrolled = identity_view::create_identity_view(&specs);
    view_stack.add_titled_with_icon(
        &identity_scrolled,
        Some("identity"),
        "Specs",
        "computer-symbolic",
    );

    // Stability view (stress test)
    let stress = StressManager::new();
    let (stability_scrolled, stability_state) =
        stability_view::create_stability_view(stress, Some(app.clone()));

    view_stack.add_titled_with_icon(
        &stability_scrolled,
        Some("stability"),
        "Stress Test",
        "system-run-symbolic",
    );

    view_stack.set_visible_child_name("stability");

    view_switcher.set_stack(Some(&view_stack));
    toolbar_view.set_content(Some(&view_stack));

    window.set_content(Some(&toolbar_view));

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
