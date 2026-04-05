mod imp;

use gettextrs::gettext;
use tracing::{info, warn};

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, gio, glib};
use libappstream::prelude::*;

use crate::app_detail::NscAppDetail;
use crate::config::{APP_ID, PKGDATADIR, PROFILE, VERSION};
use crate::runtime::runtime;
use crate::util;
use crate::window::NscWindow;

glib::wrapper! {
    pub struct NscApplication(ObjectSubclass<imp::NscApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl NscApplication {
    pub fn main_window(&self) -> NscWindow {
        self.imp()
            .window
            .get()
            .expect("window must be set before calling main_window")
            .upgrade()
            .expect("window must not be finalized")
    }

    pub fn metadata(&self) -> &std::cell::RefCell<Option<libsnow::metadata::Metadata>> {
        &self.imp().metadata
    }

    pub fn installed_nixos_attrs(&self) -> &std::cell::RefCell<std::collections::HashSet<String>> {
        &self.imp().installed_nixos_attrs
    }

    pub fn installed_hm_attrs(&self) -> &std::cell::RefCell<std::collections::HashSet<String>> {
        &self.imp().installed_hm_attrs
    }

    pub fn installed_profile_attrs(
        &self,
    ) -> &std::cell::RefCell<std::collections::HashSet<String>> {
        &self.imp().installed_profile_attrs
    }

    pub fn profile_ops_in_flight(&self) -> &std::cell::RefCell<std::collections::HashSet<String>> {
        &self.imp().profile_ops_in_flight
    }

    pub fn nixos_configured(&self) -> bool {
        self.imp().nixos_configured.get()
    }

    pub fn hm_configured(&self) -> bool {
        self.imp().hm_configured.get()
    }

    pub fn refresh_installed_attrs(&self) {
        let imp = self.imp();
        let metadata_ref = imp.metadata.borrow();
        let Some(md) = metadata_ref.as_ref() else {
            return;
        };

        let nixos_pkgs = if imp.nixos_configured.get() {
            libsnow::nixos::list::list_systempackages(md).unwrap_or_default()
        } else {
            Vec::new()
        };
        let hm_pkgs = if imp.hm_configured.get() {
            libsnow::homemanager::list::list(md).unwrap_or_default()
        } else {
            Vec::new()
        };
        let profile_pkgs = libsnow::profile::list::list().unwrap_or_default();

        *imp.installed_nixos_attrs.borrow_mut() =
            nixos_pkgs.iter().map(|p| p.attr.to_string()).collect();
        *imp.installed_hm_attrs.borrow_mut() = hm_pkgs.iter().map(|p| p.attr.to_string()).collect();
        *imp.installed_profile_attrs.borrow_mut() =
            profile_pkgs.iter().map(|p| p.attr.to_string()).collect();
    }

    pub fn refresh_after_system_apply(&self) {
        self.refresh_installed_attrs();

        let imp = self.imp();
        let metadata_ref = imp.metadata.borrow();
        let Some(md) = metadata_ref.as_ref() else {
            return;
        };

        let window = self.main_window();
        let pkgname_map = imp.pkgname_map.borrow();

        let nixos_pkgs = if imp.nixos_configured.get() {
            libsnow::nixos::list::list_systempackages(md).unwrap_or_default()
        } else {
            Vec::new()
        };
        let hm_pkgs = if imp.hm_configured.get() {
            libsnow::homemanager::list::list(md).unwrap_or_default()
        } else {
            Vec::new()
        };
        let profile_pkgs = libsnow::profile::list::list().unwrap_or_default();
        window
            .installed_page()
            .populate(&nixos_pkgs, &hm_pkgs, &profile_pkgs, &pkgname_map);

        window.explore_page().refresh_badges();
        window.search_page().refresh_badges();

        let nixos_attrs = imp.installed_nixos_attrs.borrow();
        let hm_attrs = imp.installed_hm_attrs.borrow();
        let profile_attrs = imp.installed_profile_attrs.borrow();

        if let Some(detail) = window
            .imp()
            .navigation_view
            .visible_page()
            .and_downcast::<NscAppDetail>()
            && let Some(pkgname) = detail
                .imp()
                .component
                .borrow()
                .as_ref()
                .and_then(libappstream::prelude::ComponentExt::pkgname)
        {
            let attr = pkgname.as_str();
            detail.imp().installed_nixos.set(nixos_attrs.contains(attr));
            detail.imp().installed_hm.set(hm_attrs.contains(attr));
            detail
                .imp()
                .installed_profile
                .set(profile_attrs.contains(attr));
            NscAppDetail::sync_button_states_public(&detail);
        }

        drop(nixos_attrs);
        drop(hm_attrs);
        drop(profile_attrs);
        drop(pkgname_map);
        drop(metadata_ref);

        self.refresh_updates();
    }

    pub fn refresh_updates(&self) {
        let imp = self.imp();
        let metadata_ref = imp.metadata.borrow();
        let Some(md) = metadata_ref.as_ref() else {
            return;
        };

        let window = self.main_window();
        let nixos_attrs: Vec<String> = imp.installed_nixos_attrs.borrow().iter().cloned().collect();
        let hm_attrs: Vec<String> = imp.installed_hm_attrs.borrow().iter().cloned().collect();
        let profile_attrs: Vec<String> = imp
            .installed_profile_attrs
            .borrow()
            .iter()
            .cloned()
            .collect();
        let pkgname_map = imp.pkgname_map.borrow();

        window.updates_page().check_for_updates(
            md,
            &nixos_attrs,
            &hm_attrs,
            &profile_attrs,
            &pkgname_map,
        );
    }

    pub fn unavailable_pkgnames(&self) -> &std::cell::RefCell<std::collections::HashSet<String>> {
        &self.imp().unavailable_pkgnames
    }

    pub fn pkgname_map(
        &self,
    ) -> &std::cell::RefCell<std::collections::HashMap<String, libappstream::Component>> {
        &self.imp().pkgname_map
    }

    fn setup_gactions(&self) {
        let action_quit = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| {
                app.main_window().close();
                app.quit();
            })
            .build();

        let action_about = gio::ActionEntry::builder("about")
            .activate(|app: &Self, _, _| {
                app.show_about_dialog();
            })
            .build();
        self.add_action_entries([action_quit, action_about]);
    }

    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<Control>q"]);
        self.set_accels_for_action("win.search", &["<Control>f"]);
    }

    fn setup_css(&self) {
        let provider = gtk::CssProvider::new();
        provider.load_from_resource("/org/snowflakeos/NixSoftwareCenter/style.css");
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    fn authors() -> Vec<&'static str> {
        // Authors are defined in Cargo.toml
        env!("CARGO_PKG_AUTHORS").split(':').collect()
    }

    fn show_about_dialog(&self) {
        let dialog = adw::AboutDialog::builder()
            .application_icon(*APP_ID)
            .version(*VERSION)
            .translator_credits(gettext("translator-credits"))
            .developers(Self::authors())
            .designers(vec!["Victor Fuentes"])
            .build();

        dialog.present(Some(&self.main_window()));
    }

    fn detect_available_targets(&self) {
        let imp = self.imp();
        match libsnow::config::configfile::get_config() {
            Ok(config) => {
                let nixos = config.nixos_configured();
                let hm = config.home_manager_configured();
                imp.nixos_configured.set(nixos);
                imp.hm_configured.set(hm);
                info!(
                    "Available targets: NixOS={}, Home Manager={}, Profile=always",
                    nixos, hm
                );
            }
            Err(err) => {
                info!("No libsnow config found ({err}), only profile target available");
            }
        }
    }

    fn load_metadata(&self) {
        let (sender, receiver) = async_channel::bounded(1);

        runtime().spawn(async move {
            let result = libsnow::metadata::Metadata::connect().await;
            sender
                .send(result)
                .await
                .expect("metadata channel must be open");
        });

        let app = self.clone();
        glib::spawn_future_local(async move {
            while let Ok(result) = receiver.recv().await {
                match result {
                    Ok(metadata) => {
                        info!("Metadata loaded successfully");
                        app.imp().metadata.replace(Some(metadata));
                        app.try_populate_views();
                    }
                    Err(err) => {
                        warn!("Failed to load metadata: {}", err);
                    }
                }
            }
        });
    }

    fn load_appstream(&self) {
        let data_path = match std::env::var("NSC_APPSTREAM_DATA") {
            Ok(path) => path,
            Err(_) => {
                warn!("NSC_APPSTREAM_DATA not set, skipping AppStream loading");
                return;
            }
        };

        let pool = libappstream::Pool::new();
        pool.set_load_std_data_locations(false);
        pool.add_extra_data_location(&data_path, libappstream::FormatStyle::Catalog);

        let app = self.clone();
        glib::spawn_future_local(async move {
            match pool.load_future().await {
                Ok(()) => {
                    let count = pool.components().map(|cbox| cbox.size()).unwrap_or(0);
                    info!("AppStream pool loaded: {} components", count);
                    app.imp().appstream_pool.replace(Some(pool));
                    app.try_populate_views();
                }
                Err(err) => {
                    warn!("Failed to load AppStream pool: {}", err);
                }
            }
        });
    }

    fn try_populate_views(&self) {
        let imp = self.imp();

        if imp.views_populated.get() {
            return;
        }

        let metadata = imp.metadata.borrow();
        let pool = imp.appstream_pool.borrow();

        if let (Some(md), Some(pool)) = (metadata.as_ref(), pool.as_ref())
            && let Some(window) = imp.window.get()
            && let Some(window) = window.upgrade()
        {
            let mut pkgname_map_new = std::collections::HashMap::new();
            let mut unavailable_pkgnames = std::collections::HashSet::new();
            if let Some(cbox) = pool.components() {
                for component in cbox.as_array() {
                    if let Some(pkgname) = component.pkgname() {
                        let attr = pkgname.as_str();
                        pkgname_map_new.insert(pkgname.to_string(), component);
                        if md.get(attr).is_err()
                            && md.get(util::strip_nix_output_suffix(attr)).is_err()
                        {
                            unavailable_pkgnames.insert(pkgname.to_string());
                        }
                    }
                }
            }
            info!(
                "Built pkgname map: {} entries ({} unavailable)",
                pkgname_map_new.len(),
                unavailable_pkgnames.len()
            );
            imp.unavailable_pkgnames.replace(unavailable_pkgnames);
            imp.pkgname_map.replace(pkgname_map_new);

            let pkgname_map = imp.pkgname_map.borrow();
            let nixos_pkgs = if imp.nixos_configured.get() {
                libsnow::nixos::list::list_systempackages(md).unwrap_or_default()
            } else {
                Vec::new()
            };
            let hm_pkgs = if imp.hm_configured.get() {
                libsnow::homemanager::list::list(md).unwrap_or_default()
            } else {
                Vec::new()
            };
            let profile_pkgs = libsnow::profile::list::list().unwrap_or_default();

            *imp.installed_nixos_attrs.borrow_mut() =
                nixos_pkgs.iter().map(|p| p.attr.to_string()).collect();
            *imp.installed_hm_attrs.borrow_mut() =
                hm_pkgs.iter().map(|p| p.attr.to_string()).collect();
            *imp.installed_profile_attrs.borrow_mut() =
                profile_pkgs.iter().map(|p| p.attr.to_string()).collect();

            let nixos_attrs = imp.installed_nixos_attrs.borrow();
            let hm_attrs = imp.installed_hm_attrs.borrow();
            let profile_attrs = imp.installed_profile_attrs.borrow();
            let unavailable = imp.unavailable_pkgnames.borrow();
            window.explore_page().populate(
                md,
                pool,
                &nixos_attrs,
                &hm_attrs,
                &profile_attrs,
                &unavailable,
            );
            window
                .installed_page()
                .populate(&nixos_pkgs, &hm_pkgs, &profile_pkgs, &pkgname_map);
            window.search_page().set_pool(pool);

            // start update check
            let nixos_attr_vec: Vec<String> = nixos_attrs.iter().cloned().collect();
            let hm_attr_vec: Vec<String> = hm_attrs.iter().cloned().collect();
            let profile_attr_vec: Vec<String> = profile_attrs.iter().cloned().collect();
            window.updates_page().check_for_updates(
                md,
                &nixos_attr_vec,
                &hm_attr_vec,
                &profile_attr_vec,
                &pkgname_map,
            );

            window.show_content();
            imp.views_populated.set(true);
        }
    }

    pub fn run(&self) -> glib::ExitCode {
        info!("Nix Software Center ({})", *APP_ID);
        info!("Version: {} ({})", *VERSION, *PROFILE);
        info!("Datadir: {}", *PKGDATADIR);

        ApplicationExtManual::run(self)
    }
}

impl Default for NscApplication {
    fn default() -> Self {
        glib::Object::builder()
            .property("application-id", *APP_ID)
            .property("resource-base-path", "/org/snowflakeos/NixSoftwareCenter/")
            .build()
    }
}
