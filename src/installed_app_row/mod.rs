mod imp;

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use libappstream::prelude::*;

use crate::util;

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

    pub fn name(&self) -> String {
        self.imp().name_label.label().to_string()
    }

    pub fn add_action(&self, widget: &impl IsA<gtk::Widget>) {
        self.imp().action_area.append(widget);
    }

    fn load_icon(imp: &imp::NscInstalledAppRow, component: &libappstream::Component) {
        let size = imp.icon.pixel_size() as u32;
        util::load_component_icon(&imp.icon, component, &[size]);
    }
}
