use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{glib, CompositeTemplate};

#[derive(Debug, Default, CompositeTemplate)]
#[template(resource = "/org/snowflakeos/NixSoftwareCenter/ui/app_tile.ui")]
pub struct NscAppTile {
    #[template_child]
    pub icon: TemplateChild<gtk::Image>,
    #[template_child]
    pub name_label: TemplateChild<gtk::Label>,
    #[template_child]
    pub summary_label: TemplateChild<gtk::Label>,
}

#[glib::object_subclass]
impl ObjectSubclass for NscAppTile {
    const NAME: &'static str = "NscAppTile";
    type Type = super::NscAppTile;
    type ParentType = gtk::Button;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for NscAppTile {}
impl WidgetImpl for NscAppTile {}
impl ButtonImpl for NscAppTile {}
