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
        let imp = tile.imp();

        if let Some(name) = component.name() {
            imp.name_label.set_label(name.as_str());
        }

        if let Some(summary) = component.summary() {
            imp.summary_label.set_label(summary.as_str());
        }

        if let Some(icon) = component.icon_by_size(64, 64) {
            use libappstream::prelude::IconExt;
            match IconExt::kind(&icon) {
                libappstream::IconKind::Cached => {
                    if let Some(filename) = icon.filename() {
                        imp.icon.set_from_file(Some(filename.as_str()));
                    }
                }
                libappstream::IconKind::Stock => {
                    if let Some(name) = IconExt::name(&icon) {
                        imp.icon.set_icon_name(Some(name.as_str()));
                    }
                }
                _ => {}
            }
        } else if let Some(icon) = component.icon_stock() {
            use libappstream::prelude::IconExt;
            if let Some(name) = IconExt::name(&icon) {
                imp.icon.set_icon_name(Some(name.as_str()));
            }
        } else {
            imp.icon.set_icon_name(Some("application-x-executable"));
        }

        tile
    }
}
