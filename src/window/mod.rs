mod imp;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

use crate::application::NscApplication;
use crate::apply_dialog::NscApplyDialog;
use crate::explore_page::ExplorePage;
use crate::installed_page;
use crate::pending_changes::PendingChanges;
use crate::pending_item::{ChangeKind, InstallTarget, PendingItem};
use crate::runtime::runtime;
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

    pub fn apply_pending_changes(&self) {
        let pending = self.pending_changes();
        if pending.n_items() == 0 {
            return;
        }

        let mut nixos_installs = Vec::new();
        let mut nixos_removes = Vec::new();
        let mut hm_installs = Vec::new();
        let mut hm_removes = Vec::new();

        for i in 0..pending.n_items() {
            let Some(item) = pending.item(i).and_downcast::<PendingItem>() else {
                continue;
            };
            let Some(pkgname) = item.pkgname() else {
                continue;
            };
            let attr = pkgname.to_string();
            match (item.target(), item.kind()) {
                (InstallTarget::NixOS, ChangeKind::Install) => nixos_installs.push(attr),
                (InstallTarget::NixOS, ChangeKind::Remove) => nixos_removes.push(attr),
                (InstallTarget::HomeManager, ChangeKind::Install) => hm_installs.push(attr),
                (InstallTarget::HomeManager, ChangeKind::Remove) => hm_removes.push(attr),
                (InstallTarget::Profile, _) => {}
            }
        }

        if nixos_installs.is_empty()
            && nixos_removes.is_empty()
            && hm_installs.is_empty()
            && hm_removes.is_empty()
        {
            return;
        }

        self.imp().split_view.set_show_sidebar(false);

        let dialog = NscApplyDialog::new();
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
        dialog.imp().cancel_sender.replace(Some(cancel_tx));
        dialog.present_apply(self);

        let (sender, receiver) = async_channel::bounded::<Result<(), String>>(1);

        runtime().spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(apply_changes(
                    nixos_installs,
                    nixos_removes,
                    hm_installs,
                    hm_removes,
                    cancel_rx,
                ))
            })
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r);
            let _ = sender.send(result).await;
        });

        let window_weak = self.downgrade();
        glib::spawn_future_local(async move {
            let result = receiver
                .recv()
                .await
                .unwrap_or(Err("Channel closed".into()));

            if let Some(window) = window_weak.upgrade() {
                window.pending_changes().clear();
            }

            match result {
                Ok(()) => {
                    dialog.set_success();
                }
                Err(ref err) if err == "Cancelled" => {
                    tracing::info!("Apply cancelled by user");
                }
                Err(err) => {
                    tracing::warn!("Apply failed: {err}");
                    dialog.set_failed(&err);
                }
            }

            if let Some(app) = gio::Application::default().and_downcast::<NscApplication>() {
                app.refresh_after_system_apply();
            }
        });
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

fn as_refs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

async fn apply_changes(
    nixos_installs: Vec<String>,
    nixos_removes: Vec<String>,
    hm_installs: Vec<String>,
    hm_removes: Vec<String>,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<(), String> {
    let md = libsnow::metadata::Metadata::connect()
        .await
        .map_err(|e| e.to_string())?;

    let has_nixos = !nixos_installs.is_empty() || !nixos_removes.is_empty();
    let has_hm = !hm_installs.is_empty() || !hm_removes.is_empty();

    let hm_system_managed = libsnow::config::configfile::get_config()
        .map(|c| c.system_for_home_manager)
        .unwrap_or(false);

    if has_nixos && has_hm && hm_system_managed {
        let system_content = libsnow::nixos::batch::prepare(
            &as_refs(&nixos_installs),
            &as_refs(&nixos_removes),
            &md,
        )
        .map_err(|e| e.to_string())?;
        let home_content = libsnow::homemanager::batch::prepare(
            &as_refs(&hm_installs),
            &as_refs(&hm_removes),
            &md,
        )
        .map_err(|e| e.to_string())?;

        tokio::select! {
            result = libsnow::dbus::config_both(&system_content, &home_content, "switch") => {
                result.map_err(|e| e.to_string())?;
            }
            _ = &mut cancel_rx => {
                let _ = libsnow::dbus::cancel().await;
                return Err("Cancelled".to_string());
            }
        }
    } else {
        if has_nixos {
            let content = libsnow::nixos::batch::prepare(
                &as_refs(&nixos_installs),
                &as_refs(&nixos_removes),
                &md,
            )
            .map_err(|e| e.to_string())?;
            tokio::select! {
                result = libsnow::dbus::config(&content, "switch") => {
                    result.map_err(|e| e.to_string())?;
                }
                _ = &mut cancel_rx => {
                    let _ = libsnow::dbus::cancel().await;
                    return Err("Cancelled".to_string());
                }
            }
        }

        if has_hm {
            let content = libsnow::homemanager::batch::prepare(
                &as_refs(&hm_installs),
                &as_refs(&hm_removes),
                &md,
            )
            .map_err(|e| e.to_string())?;
            if hm_system_managed {
                tokio::select! {
                    result = libsnow::dbus::config_system_home(&content, "switch") => {
                        result.map_err(|e| e.to_string())?;
                    }
                    _ = &mut cancel_rx => {
                        let _ = libsnow::dbus::cancel().await;
                        return Err("Cancelled".to_string());
                    }
                }
            } else {
                tokio::select! {
                    result = libsnow::dbus::config_home(&content, "switch") => {
                        result.map_err(|e| e.to_string())?;
                    }
                    _ = &mut cancel_rx => {
                        let _ = libsnow::dbus::cancel_home().await;
                        return Err("Cancelled".to_string());
                    }
                }
            }
        }
    }

    Ok(())
}
