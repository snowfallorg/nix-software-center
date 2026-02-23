mod imp;

use adw::subclass::prelude::*;
use gtk::glib;
use libappstream::prelude::*;

glib::wrapper! {
    pub struct NscInstalledAppRow(ObjectSubclass<imp::NscInstalledAppRow>)
        @extends gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl NscInstalledAppRow {
    pub fn new(component: &libappstream::Component, package: &libsnow::Package) -> Self {
        let row: Self = glib::Object::new();
        let imp = row.imp();

        imp.component.replace(Some(component.clone()));

        if let Some(name) = component.name() {
            imp.name_label.set_label(name.as_str());
        }

        if let Some(version) = &package.version {
            imp.version_label.set_label(version.as_str());
        }

        Self::load_icon(imp, component);

        row
    }

    pub fn component(&self) -> Option<libappstream::Component> {
        self.imp().component.borrow().clone()
    }

    fn load_icon(imp: &imp::NscInstalledAppRow, component: &libappstream::Component) {
        let size = imp.icon.pixel_size() as u32;
        crate::util::load_component_icon(&imp.icon, component, &[size]);
    }
}
