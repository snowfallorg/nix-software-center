use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, glib};

use crate::installed_app_row::NscInstalledAppRow;

#[derive(Debug, Default, CompositeTemplate)]
#[template(resource = "/org/snowflakeos/NixSoftwareCenter/ui/installed_page.ui")]
pub struct InstalledPage {
    #[template_child]
    pub nixos_section: TemplateChild<gtk::Box>,
    #[template_child]
    pub nixos_list_box: TemplateChild<gtk::ListBox>,
    #[template_child]
    pub hm_section: TemplateChild<gtk::Box>,
    #[template_child]
    pub hm_list_box: TemplateChild<gtk::ListBox>,
    #[template_child]
    pub profile_section: TemplateChild<gtk::Box>,
    #[template_child]
    pub profile_list_box: TemplateChild<gtk::ListBox>,
}

#[glib::object_subclass]
impl ObjectSubclass for InstalledPage {
    const NAME: &'static str = "NscInstalledPage";
    type Type = super::InstalledPage;
    type ParentType = adw::Bin;

    fn class_init(klass: &mut Self::Class) {
        NscInstalledAppRow::ensure_type();
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for InstalledPage {}

impl WidgetImpl for InstalledPage {}

impl BinImpl for InstalledPage {}
