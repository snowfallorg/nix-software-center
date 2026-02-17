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

        if let Some(name) = component.name() {
            imp.name_label.set_label(name.as_str());
        }

        if let Some(version) = &package.version {
            imp.version_label.set_label(version.as_str());
        }

        Self::load_icon(imp, component);

        row
    }

    fn load_icon(imp: &imp::NscInstalledAppRow, component: &libappstream::Component) {
        if let Some(icon) =
            component.icon_by_size(imp.icon.pixel_size() as u32, imp.icon.pixel_size() as u32)
        {
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
    }
}
