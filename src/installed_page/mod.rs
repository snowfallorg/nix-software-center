mod imp;

use std::collections::HashMap;

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use libappstream::prelude::ComponentExt;

use crate::installed_app_row::NscInstalledAppRow;

glib::wrapper! {
    pub struct InstalledPage(ObjectSubclass<imp::InstalledPage>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl InstalledPage {
    pub fn populate(
        &self,
        nixos_pkgs: &[libsnow::Package],
        hm_pkgs: &[libsnow::Package],
        pkgname_map: &HashMap<String, libappstream::Component>,
    ) {
        let imp = self.imp();

        let nixos_count = Self::fill_list_box(&imp.nixos_list_box, nixos_pkgs, pkgname_map);
        imp.nixos_section.set_visible(nixos_count > 0);

        let hm_count = Self::fill_list_box(&imp.hm_list_box, hm_pkgs, pkgname_map);
        imp.hm_section.set_visible(hm_count > 0);

        let profile_pkgs = libsnow::profile::list::list().unwrap_or_default();
        let profile_count = Self::fill_list_box(&imp.profile_list_box, &profile_pkgs, pkgname_map);
        imp.profile_section.set_visible(profile_count > 0);
    }

    fn fill_list_box(
        list_box: &gtk::ListBox,
        packages: &[libsnow::Package],
        pkgname_map: &HashMap<String, libappstream::Component>,
    ) -> usize {
        // Clear any existing rows
        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }

        let mut count = 0;
        let mut sorted_components = Vec::new();
        for pkg in packages {
            let attr = pkg.attr.to_string();
            if let Some(component) = pkgname_map.get(&attr) {
                sorted_components.push((component.clone(), pkg));
            }
        }

        sorted_components.sort_by_key(|(component, _pkg)| {
            component
                .name()
                .unwrap_or_default()
                .to_string()
                .to_lowercase()
        });

        for (component, pkg) in sorted_components {
            let row = NscInstalledAppRow::new(&component, pkg);
            list_box.append(&row);
            count += 1;
        }
        count
    }
}
