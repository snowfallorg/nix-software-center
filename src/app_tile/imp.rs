use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, gio, glib};

use crate::app_detail::NscAppDetail;
use crate::application::NscApplication;

#[derive(Debug, Default, CompositeTemplate)]
#[template(resource = "/org/snowflakeos/NixSoftwareCenter/ui/app_tile.ui")]
pub struct NscAppTile {
    #[template_child]
    pub icon: TemplateChild<gtk::Image>,
    #[template_child]
    pub name_label: TemplateChild<gtk::Label>,
    #[template_child]
    pub summary_label: TemplateChild<gtk::Label>,
    pub component: RefCell<Option<libappstream::Component>>,
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

impl ObjectImpl for NscAppTile {
    fn constructed(&self) {
        self.parent_constructed();

        self.obj().connect_clicked(|button| {
            let component = button.imp().component.borrow();
            let Some(component) = component.as_ref() else {
                return;
            };

            let Some(app) = gio::Application::default().and_downcast::<NscApplication>() else {
                tracing::warn!("NscAppTile clicked but no NscApplication found");
                return;
            };

            let metadata_ref = app.metadata().borrow();
            let Some(metadata) = metadata_ref.as_ref() else {
                tracing::warn!("NscAppTile clicked but metadata not loaded yet");
                return;
            };

            let Some(nav_view) = crate::util::find_navigation_view(button) else {
                tracing::warn!("NscAppTile clicked but no NavigationView ancestor found");
                return;
            };
            let detail = NscAppDetail::new(component, metadata);
            nav_view.push(&detail);
        });
    }
}

impl WidgetImpl for NscAppTile {}
impl ButtonImpl for NscAppTile {}
