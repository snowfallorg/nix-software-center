mod imp;

use adw::subclass::prelude::*;
use gtk::glib;
use libappstream::prelude::*;

glib::wrapper! {
    pub struct NscAppTile(ObjectSubclass<imp::NscAppTile>)
        @extends gtk::Button, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl NscAppTile {
    pub fn new(component: &libappstream::Component) -> Self {
        let tile: Self = glib::Object::new();
        tile.bind(component);
        tile
    }

    pub fn bind(&self, component: &libappstream::Component) {
        let imp = self.imp();

        imp.component.replace(Some(component.clone()));

        if let Some(name) = component.name() {
            imp.name_label.set_label(name.as_str());
        }

        if let Some(summary) = component.summary() {
            imp.summary_label.set_label(summary.as_str());
        }

        Self::load_icon(imp, component);
    }

    pub fn unbind(&self) {
        let imp = self.imp();
        imp.component.replace(None);
        imp.name_label.set_label("");
        imp.summary_label.set_label("");
        imp.icon.set_icon_name(Some("application-x-executable"));
    }

    fn load_icon(imp: &imp::NscAppTile, component: &libappstream::Component) {
        let size = imp.icon.pixel_size() as u32;
        crate::util::load_component_icon(&imp.icon, component, &[size]);
    }
}
