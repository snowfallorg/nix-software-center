mod imp;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

use crate::application::NscApplication;
use crate::explore_page::ExplorePage;
use crate::installed_page;
use crate::pending_changes::PendingChanges;
use crate::search_page::SearchPage;
use crate::updates_page::UpdatesPage;

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

    pub fn show_toast(&self, message: &str) {
        let toast = adw::Toast::new(message);
        self.imp().toast_overlay.add_toast(toast);
    }

    pub fn explore_page(&self) -> ExplorePage {
        self.imp()
            .view_stack
            .child_by_name("explore")
            .expect("explore page must exist in view stack")
            .downcast::<ExplorePage>()
            .expect("explore page must be an ExplorePage")
    }

    pub fn installed_page(&self) -> installed_page::InstalledPage {
        self.imp()
            .view_stack
            .child_by_name("installed")
            .expect("installed page must exist in view stack")
            .downcast::<installed_page::InstalledPage>()
            .expect("installed page must be an InstalledPage")
    }

    pub fn updates_page(&self) -> UpdatesPage {
        self.imp()
            .view_stack
            .child_by_name("updates")
            .expect("updates page must exist in view stack")
            .downcast::<UpdatesPage>()
            .expect("updates page must be an UpdatesPage")
    }

    pub fn search_page(&self) -> SearchPage {
        self.imp().search_page.clone()
    }

    pub fn show_content(&self) {
        self.imp().loading_stack.set_visible_child_name("content");
    }

    pub fn pending_changes(&self) -> &PendingChanges {
        &self.imp().pending_changes
    }

    pub fn shake_widget(widget: &impl IsA<gtk::Widget>) {
        imp::NscWindow::shake_widget(widget);
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
