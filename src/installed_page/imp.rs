use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, gio, glib};

use crate::app_detail::NscAppDetail;
use crate::application::NscApplication;
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

impl ObjectImpl for InstalledPage {
    fn constructed(&self) {
        self.parent_constructed();

        // Connect row-activated on each list box to navigate to the detail page
        let connect_row_activated = |list_box: &gtk::ListBox| {
            list_box.connect_row_activated(|_list_box, row| {
                let Some(row) = row.downcast_ref::<NscInstalledAppRow>() else {
                    return;
                };
                let Some(component) = row.component() else {
                    return;
                };

                let Some(app) = gio::Application::default().and_downcast::<NscApplication>() else {
                    tracing::warn!("InstalledAppRow activated but no NscApplication found");
                    return;
                };

                let metadata_ref = app.metadata().borrow();
                let Some(metadata) = metadata_ref.as_ref() else {
                    tracing::warn!("InstalledAppRow activated but metadata not loaded yet");
                    return;
                };

                let Some(nav_view) = crate::util::find_navigation_view(row) else {
                    tracing::warn!(
                        "InstalledAppRow activated but no NavigationView ancestor found"
                    );
                    return;
                };
                let nixos_attrs = app.installed_nixos_attrs().borrow();
                let hm_attrs = app.installed_hm_attrs().borrow();
                let profile_attrs = app.installed_profile_attrs().borrow();
                let detail = NscAppDetail::new(
                    &component,
                    metadata,
                    &nixos_attrs,
                    &hm_attrs,
                    &profile_attrs,
                );
                nav_view.push(&detail);
            });
        };

        connect_row_activated(&self.nixos_list_box);
        connect_row_activated(&self.hm_list_box);
        connect_row_activated(&self.profile_list_box);
    }
}

impl WidgetImpl for InstalledPage {}

impl BinImpl for InstalledPage {}
