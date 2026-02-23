use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, glib};

#[derive(Debug, Default, CompositeTemplate)]
#[template(resource = "/org/snowflakeos/NixSoftwareCenter/ui/screenshot_slot.ui")]
pub struct NscScreenshotSlot {
    #[template_child]
    pub stack: TemplateChild<gtk::Stack>,
    #[template_child]
    pub picture: TemplateChild<gtk::Picture>,
}

#[glib::object_subclass]
impl ObjectSubclass for NscScreenshotSlot {
    const NAME: &'static str = "NscScreenshotSlot";
    type Type = super::NscScreenshotSlot;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
        klass.set_layout_manager_type::<gtk::BinLayout>();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for NscScreenshotSlot {
    fn dispose(&self) {
        self.stack.unparent();
    }
}

impl WidgetImpl for NscScreenshotSlot {}
