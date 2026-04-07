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
        desktop_ids: &HashSet<String>,
    ) -> Self {
        let tile: Self = glib::Object::new();
        tile.bind(component, nixos_attrs, hm_attrs, profile_attrs, desktop_ids);
        tile
    }

    pub fn bind(
        &self,
        component: &libappstream::Component,
        nixos_attrs: &HashSet<String>,
        hm_attrs: &HashSet<String>,
        profile_attrs: &HashSet<String>,
        desktop_ids: &HashSet<String>,
    ) {
        let imp = self.imp();

        imp.component.replace(Some(component.clone()));

        if let Some(name) = component.name() {
            imp.name_label.set_label(name.as_str());
        }

        if let Some(summary) = component.summary() {
            imp.summary_label.set_label(summary.as_str());
        }

        self.load_icon(component);
        self.update_install_badge(component, nixos_attrs, hm_attrs, profile_attrs, desktop_ids);
    }

    pub fn unbind(&self) {
        let imp = self.imp();
        imp.component.replace(None);
        imp.icon_generation
            .set(imp.icon_generation.get().wrapping_add(1));
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
        desktop_ids: &HashSet<String>,
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

        if util::has_system_desktop_file(component, desktop_ids) {
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
        let desktop_ids = app.system_desktop_ids().borrow();
        self.update_install_badge(
            &component,
            &nixos_attrs,
            &hm_attrs,
            &profile_attrs,
            &desktop_ids,
        );
    }

    fn load_icon(&self, component: &libappstream::Component) {
        let imp = self.imp();
        let size = imp.icon.pixel_size() as u32;
        let source = util::resolve_component_icon(component, &[size]);
        let generation = imp.icon_generation.get().wrapping_add(1);
        imp.icon_generation.set(generation);
        let weak = self.downgrade();
        util::load_icon_async(&imp.icon, source, generation, move || {
            weak.upgrade()
                .map(|t| t.imp().icon_generation.get())
                .unwrap_or(u64::MAX)
        });
    }
}
