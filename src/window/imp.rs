use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, gdk, gio, glib};

use crate::config::{APP_ID, PROFILE};
use crate::explore_page::ExplorePage;
use crate::installed_page::InstalledPage;
use crate::search_page::SearchPage;
use crate::updates_page::UpdatesPage;

#[derive(Debug, CompositeTemplate)]
#[template(resource = "/org/snowflakeos/NixSoftwareCenter/ui/window.ui")]
pub struct NscWindow {
    #[template_child]
    pub split_view: TemplateChild<adw::OverlaySplitView>,
    #[template_child]
    pub headerbar: TemplateChild<adw::HeaderBar>,
    #[template_child]
    pub navigation_view: TemplateChild<adw::NavigationView>,
    #[template_child]
    pub view_stack: TemplateChild<adw::ViewStack>,
    #[template_child]
    pub search_button: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    pub sidebar_button: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    pub search_bar: TemplateChild<gtk::SearchBar>,
    #[template_child]
    pub search_entry: TemplateChild<gtk::SearchEntry>,
    #[template_child]
    pub search_page: TemplateChild<SearchPage>,
    pub settings: gio::Settings,
    pub last_tab: RefCell<String>,
}

impl Default for NscWindow {
    fn default() -> Self {
        Self {
            split_view: TemplateChild::default(),
            headerbar: TemplateChild::default(),
            navigation_view: TemplateChild::default(),
            view_stack: TemplateChild::default(),
            search_button: TemplateChild::default(),
            sidebar_button: TemplateChild::default(),
            search_bar: TemplateChild::default(),
            search_entry: TemplateChild::default(),
            search_page: TemplateChild::default(),
            settings: gio::Settings::new(*APP_ID),
            last_tab: RefCell::new("explore".to_string()),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for NscWindow {
    const NAME: &'static str = "NscWindow";
    type Type = super::NscWindow;
    type ParentType = adw::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        ExplorePage::ensure_type();
        InstalledPage::ensure_type();
        UpdatesPage::ensure_type();
        SearchPage::ensure_type();

        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for NscWindow {
    fn constructed(&self) {
        self.parent_constructed();
        let obj = self.obj();

        // Devel Profile
        if *PROFILE == "Devel" {
            obj.add_css_class("devel");
        }

        // Load latest window state
        obj.load_window_size();

        // Search action activates search; if already active, re-focuses the entry
        let search_button = self.search_button.clone();
        let search_entry = self.search_entry.clone();
        let nav_view = self.navigation_view.clone();
        let action_search = gio::ActionEntry::builder("search")
            .activate(move |win: &super::NscWindow, _, _| {
                if let Some(focus) = gtk::prelude::GtkWindowExt::focus(win)
                    && !gtk::prelude::WidgetExt::is_ancestor(&focus, &nav_view)
                {
                    return;
                }
                search_button.set_active(true);
                search_entry.grab_focus();
            })
            .build();
        obj.add_action_entries([action_search]);

        // Accept keyboard input even when search bar is hidden
        self.search_bar.connect_entry(&*self.search_entry);

        let search_button_ref = self.search_button.clone();
        let search_entry_ref = self.search_entry.clone();
        let search_bar_ref = self.search_bar.clone();
        let nav_view_ref = self.navigation_view.clone();
        let key_controller = gtk::EventControllerKey::new();
        key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        key_controller.connect_key_pressed(move |controller, keyval, _keycode, state| {
            // Only handle keys when focus is within the main content
            if let Some(window) = controller.widget().and_downcast::<gtk::Window>()
                && let Some(focus) = gtk::prelude::GtkWindowExt::focus(&window)
                && !gtk::prelude::WidgetExt::is_ancestor(&focus, &nav_view_ref)
            {
                return glib::Propagation::Proceed;
            }

            // Ignore shortcuts
            let shortcut = state & (gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::ALT_MASK);
            if !shortcut.is_empty() {
                return glib::Propagation::Proceed;
            }

            if let Some(ch) = keyval.to_unicode()
                && !ch.is_control()
            {
                // If search is already active, recapture stray keys to the entry
                if search_bar_ref.is_search_mode() && !search_entry_ref.has_focus() {
                    search_entry_ref.grab_focus();
                    search_entry_ref.delete_selection();
                    let mut pos = search_entry_ref.position();
                    let s = ch.to_string();
                    search_entry_ref.insert_text(&s, &mut pos);
                    search_entry_ref.set_position(pos);
                    return glib::Propagation::Stop;
                }

                // If search is not active, activate it and start typing
                if !search_bar_ref.is_search_mode() {
                    search_button_ref.set_active(true);
                    search_entry_ref.grab_focus();
                    let s = ch.to_string();
                    search_entry_ref.insert_text(&s, &mut 0);
                    search_entry_ref.set_position(s.len() as i32);
                    return glib::Propagation::Stop;
                }
            }
            glib::Propagation::Proceed
        });
        obj.add_controller(key_controller);

        // When search mode is enabled, focus the entry
        // When text is typed, switch to the search page
        // When disabled, restore the previous tab if we're still on the search page
        let view_stack = self.view_stack.clone();
        let search_entry = self.search_entry.clone();
        let last_tab = self.last_tab.clone();
        self.search_bar
            .connect_search_mode_enabled_notify(glib::clone!(
                #[weak]
                view_stack,
                #[weak]
                search_entry,
                move |bar| {
                    if bar.is_search_mode() {
                        if let Some(name) = view_stack.visible_child_name()
                            && name != "search"
                        {
                            *last_tab.borrow_mut() = name.to_string();
                        }
                        search_entry.grab_focus();
                    } else if let Some(name) = view_stack.visible_child_name()
                        && name == "search"
                    {
                        let tab = last_tab.borrow().clone();
                        view_stack.set_visible_child_name(&tab);
                    }
                }
            ));

        // When search text changes, switch to the search page and run the query
        let search_page = self.search_page.clone();
        let search_bar = self.search_bar.clone();
        let view_stack = self.view_stack.clone();
        self.search_entry.connect_search_changed(glib::clone!(
            #[weak]
            search_page,
            #[weak]
            search_bar,
            #[weak]
            view_stack,
            move |entry| {
                if !search_bar.is_search_mode() {
                    return;
                }
                let query = entry.text();
                view_stack.set_visible_child_name("search");
                search_page.perform_search(&query);
            }
        ));

        // Pressing Escape in search entry closes search
        let search_button = self.search_button.clone();
        self.search_entry.connect_stop_search(glib::clone!(
            #[weak]
            search_button,
            move |_| {
                search_button.set_active(false);
            }
        ));

        // Clicking a visible ViewStack tab dismisses search.
        let search_button = self.search_button.clone();
        self.view_stack.connect_visible_child_notify(glib::clone!(
            #[weak]
            search_button,
            move |stack| {
                if let Some(name) = stack.visible_child_name()
                    && name != "search"
                {
                    search_button.set_active(false);
                }
            }
        ));
    }
}

impl WidgetImpl for NscWindow {}

impl WindowImpl for NscWindow {
    fn close_request(&self) -> glib::Propagation {
        if let Err(err) = self.obj().save_window_size() {
            tracing::warn!("Failed to save window state, {}", &err);
        }

        self.parent_close_request()
    }
}

impl ApplicationWindowImpl for NscWindow {}

impl AdwApplicationWindowImpl for NscWindow {}
