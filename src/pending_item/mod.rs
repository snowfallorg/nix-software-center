mod imp;

use glib::subclass::prelude::*;
use gtk::glib;

glib::wrapper! {
    pub struct PendingItem(ObjectSubclass<imp::PendingItem>);
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    #[default]
    Install,
    Remove,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InstallTarget {
    #[default]
    NixOS,
    HomeManager,
}

impl PendingItem {
    pub fn new_install(component: &libappstream::Component, target: InstallTarget) -> Self {
        let item: Self = glib::Object::new();
        let imp = item.imp();
        imp.component.replace(Some(component.clone()));
        imp.kind.set(ChangeKind::Install);
        imp.target.set(target);
        item
    }

    pub fn new_remove(component: &libappstream::Component, target: InstallTarget) -> Self {
        let item: Self = glib::Object::new();
        let imp = item.imp();
        imp.component.replace(Some(component.clone()));
        imp.kind.set(ChangeKind::Remove);
        imp.target.set(target);
        item
    }

    pub fn component(&self) -> Option<libappstream::Component> {
        self.imp().component.borrow().clone()
    }

    pub fn kind(&self) -> ChangeKind {
        self.imp().kind.get()
    }

    pub fn target(&self) -> InstallTarget {
        self.imp().target.get()
    }

    pub fn pkgname(&self) -> Option<glib::GString> {
        self.component()
            .and_then(|c| libappstream::prelude::ComponentExt::pkgname(&c))
    }
}
