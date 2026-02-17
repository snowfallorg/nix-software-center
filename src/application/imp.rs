use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::WeakRef;
use gtk::glib;
use libappstream::Pool;
use libsnow::metadata::Metadata;
use std::cell::{OnceCell, RefCell};
use tracing::debug;

use crate::config::APP_ID;
use crate::window::NscWindow;

#[derive(Default)]
pub struct NscApplication {
    pub window: OnceCell<WeakRef<NscWindow>>,
    pub metadata: RefCell<Option<Metadata>>,
    pub appstream_pool: RefCell<Option<Pool>>,
}

#[glib::object_subclass]
impl ObjectSubclass for NscApplication {
    const NAME: &'static str = "NscApplication";
    type Type = super::NscApplication;
    type ParentType = adw::Application;
}

impl ObjectImpl for NscApplication {}

impl ApplicationImpl for NscApplication {
    fn activate(&self) {
        debug!("AdwApplication<NscApplication>::activate");
        self.parent_activate();
        let app = self.obj();

        if let Some(window) = self.window.get() {
            let window = window.upgrade().expect("window must not be finalized");
            window.present();
            return;
        }

        let window = NscWindow::new(&app);
        self.window
            .set(window.downgrade())
            .expect("Window already set.");

        app.main_window().present();
    }

    fn startup(&self) {
        debug!("AdwApplication<NscApplication>::startup");
        self.parent_startup();
        let app = self.obj();

        // Set icons for shell
        gtk::Window::set_default_icon_name(*APP_ID);

        app.setup_css();
        app.load_metadata();
        app.load_appstream();
        app.setup_gactions();
        app.setup_accels();
    }
}

impl GtkApplicationImpl for NscApplication {}

impl AdwApplicationImpl for NscApplication {}
