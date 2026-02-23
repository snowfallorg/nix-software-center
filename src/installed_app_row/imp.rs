use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, glib};
use std::cell::RefCell;

#[derive(Debug, Default, CompositeTemplate)]
#[template(resource = "/org/snowflakeos/NixSoftwareCenter/ui/installed_app_row.ui")]
pub struct NscInstalledAppRow {
    #[template_child]
    pub icon: TemplateChild<gtk::Image>,
    #[template_child]
    pub name_label: TemplateChild<gtk::Label>,
    #[template_child]
    pub version_label: TemplateChild<gtk::Label>,
    pub component: RefCell<Option<libappstream::Component>>,
}

#[glib::object_subclass]
impl ObjectSubclass for NscInstalledAppRow {
    const NAME: &'static str = "NscInstalledAppRow";
    type Type = super::NscInstalledAppRow;
    type ParentType = gtk::ListBoxRow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for NscInstalledAppRow {}
impl WidgetImpl for NscInstalledAppRow {}
impl ListBoxRowImpl for NscInstalledAppRow {}
