use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, glib};
use std::cell::{Cell, RefCell};

use crate::pending_item::InstallTarget;

#[derive(Debug, Default, CompositeTemplate)]
#[template(resource = "/org/snowflakeos/NixSoftwareCenter/ui/pending_row.ui")]
pub struct NscPendingRow {
    #[template_child]
    pub icon: TemplateChild<gtk::Image>,
    #[template_child]
    pub name_label: TemplateChild<gtk::Label>,
    #[template_child]
    pub kind_label: TemplateChild<gtk::Label>,
    #[template_child]
    pub remove_button: TemplateChild<gtk::Button>,
    pub component: RefCell<Option<libappstream::Component>>,
    pub target: Cell<InstallTarget>,
}

#[glib::object_subclass]
impl ObjectSubclass for NscPendingRow {
    const NAME: &'static str = "NscPendingRow";
    type Type = super::NscPendingRow;
    type ParentType = gtk::ListBoxRow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for NscPendingRow {}
impl WidgetImpl for NscPendingRow {}
impl ListBoxRowImpl for NscPendingRow {}
