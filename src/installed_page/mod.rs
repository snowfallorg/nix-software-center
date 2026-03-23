mod imp;

use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gio, glib};
use libappstream::prelude::ComponentExt;
use std::collections::HashMap;

use crate::application::NscApplication;
use crate::installed_app_row::NscInstalledAppRow;
use crate::pending_item::InstallTarget;
use crate::window::NscWindow;
use crate::{app_detail, runtime};

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
        profile_pkgs: &[libsnow::Package],
        pkgname_map: &HashMap<String, libappstream::Component>,
    ) {
        let imp = self.imp();

        let nixos_count = Self::fill_system_list(
            &imp.nixos_list_box,
            nixos_pkgs,
            pkgname_map,
            InstallTarget::NixOS,
        );
        imp.nixos_section.set_visible(nixos_count > 0);

        let hm_count = Self::fill_system_list(
            &imp.hm_list_box,
            hm_pkgs,
            pkgname_map,
            InstallTarget::HomeManager,
        );
        imp.hm_section.set_visible(hm_count > 0);

        let profile_count =
            Self::fill_profile_list(&imp.profile_list_box, profile_pkgs, pkgname_map);
        imp.profile_section.set_visible(profile_count > 0);
    }

    pub fn refresh_profile_section(
        &self,
        profile_pkgs: &[libsnow::Package],
        pkgname_map: &HashMap<String, libappstream::Component>,
    ) {
        let imp = self.imp();
        let profile_count =
            Self::fill_profile_list(&imp.profile_list_box, profile_pkgs, pkgname_map);
        imp.profile_section.set_visible(profile_count > 0);
    }

    fn sorted_components<'a>(
        packages: &'a [libsnow::Package],
        pkgname_map: &'a HashMap<String, libappstream::Component>,
    ) -> Vec<(libappstream::Component, &'a libsnow::Package)> {
        let mut sorted = Vec::new();
        for pkg in packages {
            let attr = pkg.attr.to_string();
            if let Some(component) = pkgname_map.get(&attr) {
                sorted.push((component.clone(), pkg));
            }
        }
        sorted.sort_by_key(|(component, _)| {
            component
                .name()
                .unwrap_or_default()
                .to_string()
                .to_lowercase()
        });
        sorted
    }

    fn fill_system_list(
        list_box: &gtk::ListBox,
        packages: &[libsnow::Package],
        pkgname_map: &HashMap<String, libappstream::Component>,
        target: InstallTarget,
    ) -> usize {
        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }

        let sorted = Self::sorted_components(packages, pkgname_map);
        let count = sorted.len();

        for (component, pkg) in sorted {
            let row = NscInstalledAppRow::new(&component, pkg);

            let comp = component.clone();
            let button = gtk::Button::builder()
                .icon_name("user-trash-symbolic")
                .valign(gtk::Align::Center)
                .tooltip_text("Remove")
                .build();
            button.connect_clicked(move |button| {
                let Some(app) = gio::Application::default().and_downcast::<NscApplication>() else {
                    return;
                };
                let Some(window) = app.active_window().and_downcast::<NscWindow>() else {
                    return;
                };
                let pending = window.pending_changes();
                if pending.contains(&comp, target) {
                    pending.remove_by_component(&comp, target);
                    button.remove_css_class("destructive-action");
                } else {
                    pending.add_remove(&comp, target);
                    button.add_css_class("destructive-action");
                }
            });
            row.add_action(&button);
            list_box.append(&row);
        }
        count
    }

    fn fill_profile_list(
        list_box: &gtk::ListBox,
        packages: &[libsnow::Package],
        pkgname_map: &HashMap<String, libappstream::Component>,
    ) -> usize {
        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }

        let sorted = Self::sorted_components(packages, pkgname_map);
        let count = sorted.len();

        for (component, pkg) in sorted {
            let row = NscInstalledAppRow::new(&component, pkg);

            let attr = pkg.attr.to_string();
            let button = gtk::Button::builder()
                .icon_name("user-trash-symbolic")
                .valign(gtk::Align::Center)
                .tooltip_text("Remove")
                .build();
            button.connect_clicked(move |button| {
                Self::profile_remove_clicked(button, &attr);
            });
            row.add_action(&button);
            list_box.append(&row);
        }
        count
    }

    fn profile_remove_clicked(button: &gtk::Button, attr: &str) {
        let Some(app) = gio::Application::default().and_downcast::<NscApplication>() else {
            return;
        };
        if !app
            .profile_ops_in_flight()
            .borrow_mut()
            .insert(attr.to_string())
        {
            return;
        }

        let spinner = adw::Spinner::new();
        button.set_child(Some(&spinner));
        button.set_sensitive(false);

        let attr_owned = attr.to_string();
        let attr_for_task = attr_owned.clone();
        let (sender, receiver) = async_channel::bounded::<Result<(), String>>(1);

        runtime::runtime().spawn(async move {
            let result = libsnow::profile::remove::remove(&[&attr_for_task])
                .await
                .map_err(|e| e.to_string());
            let _ = sender.send(result).await;
        });
        glib::spawn_future_local(async move {
            let Ok(result) = receiver.recv().await else {
                return;
            };

            if let Some(app) = gio::Application::default().and_downcast::<NscApplication>() {
                app.profile_ops_in_flight().borrow_mut().remove(&attr_owned);
            }

            match result {
                Ok(()) => {
                    tracing::info!("Profile remove from installed page succeeded");
                    let Some(app) = gio::Application::default().and_downcast::<NscApplication>()
                    else {
                        return;
                    };
                    let profile_pkgs = libsnow::profile::list::list().unwrap_or_default();
                    *app.installed_profile_attrs().borrow_mut() =
                        profile_pkgs.iter().map(|p| p.attr.to_string()).collect();

                    let window = app.main_window();
                    let pkgname_map = app.pkgname_map().borrow();
                    window
                        .installed_page()
                        .refresh_profile_section(&profile_pkgs, &pkgname_map);

                    window.explore_page().refresh_badges();
                    window.search_page().refresh_badges();

                    app.refresh_updates();

                    app_detail::NscAppDetail::finish_profile_op_on_visible_detail(
                        &attr_owned,
                        true,
                        false,
                    );
                }
                Err(err) => {
                    tracing::warn!("Profile remove from installed page failed: {err}");
                    if let Some(app) = gio::Application::default().and_downcast::<NscApplication>()
                    {
                        app.main_window()
                            .show_toast(&format!("Remove failed: {err}"));
                    }
                }
            }
        });
    }
}
