use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{glib, CompositeTemplate};

#[derive(Debug, Default, CompositeTemplate)]
#[template(resource = "/org/snowflakeos/NixSoftwareCenter/ui/installed_page.ui")]
pub struct InstalledPage {}

#[glib::object_subclass]
impl ObjectSubclass for InstalledPage {
    const NAME: &'static str = "NscInstalledPage";
    type Type = super::InstalledPage;
    type ParentType = adw::Bin;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for InstalledPage {}

impl WidgetImpl for InstalledPage {}

impl BinImpl for InstalledPage {}
