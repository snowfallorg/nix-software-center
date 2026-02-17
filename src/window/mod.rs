mod imp;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

use crate::application::NscApplication;
use crate::explore_page::ExplorePage;

glib::wrapper! {
    pub struct NscWindow(ObjectSubclass<imp::NscWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap,
                    gtk::Root, gtk::Native, gtk::ShortcutManager,
                    gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl NscWindow {
    pub fn new(app: &NscApplication) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    pub fn explore_page(&self) -> ExplorePage {
        self.imp()
            .view_stack
            .child_by_name("explore")
            .expect("explore page must exist in view stack")
            .downcast::<ExplorePage>()
            .expect("explore page must be an ExplorePage")
    }

    pub fn installed_page(&self) -> crate::installed_page::InstalledPage {
        self.imp()
            .view_stack
            .child_by_name("installed")
            .expect("installed page must exist in view stack")
            .downcast::<crate::installed_page::InstalledPage>()
            .expect("installed page must be an InstalledPage")
    }

    fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let imp = self.imp();

        let (width, height) = self.default_size();

        imp.settings.set_int("window-width", width)?;
        imp.settings.set_int("window-height", height)?;

        imp.settings
            .set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let imp = self.imp();

        let width = imp.settings.int("window-width");
        let height = imp.settings.int("window-height");
        let is_maximized = imp.settings.boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
    }
}
