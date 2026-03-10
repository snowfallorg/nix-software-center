mod imp;

use glib::subclass::prelude::*;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use libappstream::prelude::ComponentExt;

use crate::pending_item::{InstallTarget, PendingItem};

glib::wrapper! {
    pub struct PendingChanges(ObjectSubclass<imp::PendingChanges>)
        @implements gio::ListModel;
}

impl PendingChanges {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn store(&self) -> &gio::ListStore {
        &self.imp().store
    }

    pub fn add_install(&self, component: &libappstream::Component, target: InstallTarget) -> bool {
        if self.contains(component, target) {
            return false;
        }
        self.store().append(&PendingItem::new_install(component, target));
        true
    }

    pub fn add_remove(&self, component: &libappstream::Component, target: InstallTarget) -> bool {
        if self.contains(component, target) {
            return false;
        }
        self.store().append(&PendingItem::new_remove(component, target));
        true
    }

    pub fn remove_by_component(
        &self,
        component: &libappstream::Component,
        target: InstallTarget,
    ) -> bool {
        let target_pkgname = component.pkgname();
        for i in 0..self.n_items() {
            if let Some(item) = self.item(i).and_downcast::<PendingItem>()
                && item.pkgname() == target_pkgname
                && item.target() == target
            {
                self.store().remove(i);
                return true;
            }
        }
        false
    }

    pub fn contains(&self, component: &libappstream::Component, target: InstallTarget) -> bool {
        let target_pkgname = component.pkgname();
        for i in 0..self.n_items() {
            if let Some(item) = self.item(i).and_downcast::<PendingItem>()
                && item.pkgname() == target_pkgname
                && item.target() == target
            {
                return true;
            }
        }
        false
    }

    pub fn items_for_target(&self, target: InstallTarget) -> Vec<PendingItem> {
        let mut items = Vec::new();
        for i in 0..self.n_items() {
            if let Some(item) = self.item(i).and_downcast::<PendingItem>()
                && item.target() == target
            {
                items.push(item);
            }
        }
        items
    }
}

impl Default for PendingChanges {
    fn default() -> Self {
        Self::new()
    }
}
