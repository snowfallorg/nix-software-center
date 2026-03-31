mod imp;

use std::collections::HashSet;

use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gio, glib};
use libappstream::prelude::*;

use crate::application::NscApplication;
use crate::util;

glib::wrapper! {
    pub struct NscAppTile(ObjectSubclass<imp::NscAppTile>)
        @extends gtk::Button, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl NscAppTile {
    pub fn new(
        component: &libappstream::Component,
        nixos_attrs: &HashSet<String>,
        hm_attrs: &HashSet<String>,
        profile_attrs: &HashSet<String>,
    ) -> Self {
        let tile: Self = glib::Object::new();
        tile.bind(component, nixos_attrs, hm_attrs, profile_attrs);
        tile
    }

    pub fn bind(
        &self,
        component: &libappstream::Component,
        nixos_attrs: &HashSet<String>,
        hm_attrs: &HashSet<String>,
        profile_attrs: &HashSet<String>,
    ) {
        let imp = self.imp();

        imp.component.replace(Some(component.clone()));

        if let Some(name) = component.name() {
            imp.name_label.set_label(name.as_str());
        }

        if let Some(summary) = component.summary() {
            imp.summary_label.set_label(summary.as_str());
        }

        Self::load_icon(imp, component);
        self.update_install_badge(component, nixos_attrs, hm_attrs, profile_attrs);
    }

    pub fn unbind(&self) {
        let imp = self.imp();
        imp.component.replace(None);
        imp.name_label.set_label("");
        imp.summary_label.set_label("");
        imp.icon.set_icon_name(Some("application-x-executable"));
        imp.install_badge.set_visible(false);
        imp.install_badge.remove_css_class("install-badge-nix");
        imp.install_badge.remove_css_class("install-badge-system");
    }

    fn update_install_badge(
        &self,
        component: &libappstream::Component,
        nixos_attrs: &HashSet<String>,
        hm_attrs: &HashSet<String>,
        profile_attrs: &HashSet<String>,
    ) {
        let imp = self.imp();
        let badge = &*imp.install_badge;

        badge.remove_css_class("install-badge-nix");
        badge.remove_css_class("install-badge-system");

        let nix_installed = component.pkgname().is_some_and(|p| {
            nixos_attrs.contains(p.as_str())
                || hm_attrs.contains(p.as_str())
                || profile_attrs.contains(p.as_str())
        });

        if nix_installed {
            badge.set_icon_name(Some("nsc-installed-symbolic"));
            badge.add_css_class("install-badge-nix");
            badge.set_tooltip_text(Some("Installed with Nix"));
            badge.set_visible(true);
            return;
        }

        let has_desktop_file = util::has_system_desktop_file(component);
        if has_desktop_file {
            badge.set_icon_name(Some("nsc-installed-symbolic"));
            badge.add_css_class("install-badge-system");
            badge.set_tooltip_text(Some("Installed on system"));
            badge.set_visible(true);
            return;
        }

        badge.set_visible(false);
    }

    /// Re-read the global installed attr sets and update the badge.
    pub fn refresh_badge(&self) {
        let imp = self.imp();
        let Some(component) = imp.component.borrow().clone() else {
            return;
        };

        let Some(app) = gio::Application::default().and_downcast::<NscApplication>() else {
            return;
        };

        let nixos_attrs = app.installed_nixos_attrs().borrow();
        let hm_attrs = app.installed_hm_attrs().borrow();
        let profile_attrs = app.installed_profile_attrs().borrow();
        self.update_install_badge(&component, &nixos_attrs, &hm_attrs, &profile_attrs);
    }

    fn load_icon(imp: &imp::NscAppTile, component: &libappstream::Component) {
        let size = imp.icon.pixel_size() as u32;
        util::load_component_icon(&imp.icon, component, &[size]);
    }
}
