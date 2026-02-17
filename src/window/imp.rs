use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, gio, glib};

use crate::config::{APP_ID, PROFILE};
use crate::explore_page::ExplorePage;
use crate::installed_page::InstalledPage;
use crate::updates_page::UpdatesPage;

#[derive(Debug, CompositeTemplate)]
#[template(resource = "/org/snowflakeos/NixSoftwareCenter/ui/window.ui")]
pub struct NscWindow {
    #[template_child]
    pub headerbar: TemplateChild<adw::HeaderBar>,
    #[template_child]
    pub navigation_view: TemplateChild<adw::NavigationView>,
    #[template_child]
    pub view_stack: TemplateChild<adw::ViewStack>,
    pub settings: gio::Settings,
}

impl Default for NscWindow {
    fn default() -> Self {
        Self {
            headerbar: TemplateChild::default(),
            navigation_view: TemplateChild::default(),
            view_stack: TemplateChild::default(),
            settings: gio::Settings::new(*APP_ID),
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
