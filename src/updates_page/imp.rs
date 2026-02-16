use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{glib, CompositeTemplate};

#[derive(Debug, Default, CompositeTemplate)]
#[template(resource = "/org/snowflakeos/NixSoftwareCenter/ui/updates_page.ui")]
pub struct UpdatesPage {}

#[glib::object_subclass]
impl ObjectSubclass for UpdatesPage {
    const NAME: &'static str = "NscUpdatesPage";
    type Type = super::UpdatesPage;
    type ParentType = adw::Bin;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for UpdatesPage {}

impl WidgetImpl for UpdatesPage {}

impl BinImpl for UpdatesPage {}
