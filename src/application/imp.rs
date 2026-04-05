use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::WeakRef;
use gtk::glib;
use libappstream::{Component, Pool};
use libsnow::metadata::Metadata;
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use tracing::debug;

use crate::config::APP_ID;
use crate::window::NscWindow;

#[derive(Default)]
pub struct NscApplication {
    pub window: OnceCell<WeakRef<NscWindow>>,
    pub metadata: RefCell<Option<Metadata>>,
    pub appstream_pool: RefCell<Option<Pool>>,
    pub pkgname_map: RefCell<HashMap<String, Component>>,
    pub unavailable_pkgnames: RefCell<HashSet<String>>,
    pub installed_nixos_attrs: RefCell<HashSet<String>>,
    pub installed_hm_attrs: RefCell<HashSet<String>>,
    pub installed_profile_attrs: RefCell<HashSet<String>>,
    pub profile_ops_in_flight: RefCell<HashSet<String>>,
    pub nixos_configured: Cell<bool>,
    pub hm_configured: Cell<bool>,
    pub views_populated: Cell<bool>,
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

        // Data may have loaded before the window was created
        app.try_populate_views();
        app.main_window().present();
    }

    fn startup(&self) {
        debug!("AdwApplication<NscApplication>::startup");
        self.parent_startup();
        let app = self.obj();

        // Set icons for shell
        gtk::Window::set_default_icon_name(*APP_ID);

        app.setup_css();
        app.setup_gactions();
        app.setup_accels();
        app.detect_available_targets();
        app.load_metadata();
        app.load_appstream();
    }
}

impl GtkApplicationImpl for NscApplication {}

impl AdwApplicationImpl for NscApplication {}
