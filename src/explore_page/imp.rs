use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, glib};

#[derive(Debug, Default, CompositeTemplate)]
#[template(resource = "/org/snowflakeos/NixSoftwareCenter/ui/explore_page.ui")]
pub struct ExplorePage {
    #[template_child]
    pub flow_box: TemplateChild<gtk::FlowBox>,
}

#[glib::object_subclass]
impl ObjectSubclass for ExplorePage {
    const NAME: &'static str = "NscExplorePage";
    type Type = super::ExplorePage;
    type ParentType = adw::BreakpointBin;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for ExplorePage {}

impl WidgetImpl for ExplorePage {}

impl BreakpointBinImpl for ExplorePage {}
