use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, gio, glib};

use crate::app_detail::NscAppDetail;
use crate::application::NscApplication;
use crate::installed_app_row::NscInstalledAppRow;

#[derive(Debug, Default, CompositeTemplate)]
#[template(resource = "/org/snowflakeos/NixSoftwareCenter/ui/updates_page.ui")]
pub struct UpdatesPage {
    #[template_child]
    pub loading_stack: TemplateChild<gtk::Stack>,
    #[template_child]
    pub error_status: TemplateChild<adw::StatusPage>,
    #[template_child]
    pub warnings_banner: TemplateChild<adw::Banner>,
    #[template_child]
    pub update_everything_section: TemplateChild<gtk::Box>,
    #[template_child]
    pub update_everything_subtitle: TemplateChild<gtk::Label>,
    #[template_child]
    pub update_everything_button: TemplateChild<gtk::Button>,

    #[template_child]
    pub system_header_section: TemplateChild<gtk::Box>,
    #[template_child]
    pub system_header_subtitle: TemplateChild<gtk::Label>,
    #[template_child]
    pub system_update_button: TemplateChild<gtk::Button>,

    #[template_child]
    pub nixos_updates_section: TemplateChild<gtk::Box>,
    #[template_child]
    pub nixos_updates_list: TemplateChild<gtk::ListBox>,

    #[template_child]
    pub hm_updates_section: TemplateChild<gtk::Box>,
    #[template_child]
    pub hm_updates_list: TemplateChild<gtk::ListBox>,

    #[template_child]
    pub profile_separator: TemplateChild<gtk::Separator>,
    #[template_child]
    pub profile_updates_section: TemplateChild<gtk::Box>,
    #[template_child]
    pub profile_header_subtitle: TemplateChild<gtk::Label>,
    #[template_child]
    pub profile_update_button: TemplateChild<gtk::Button>,
    #[template_child]
    pub profile_now_section: TemplateChild<gtk::Box>,
    #[template_child]
    pub profile_now_list: TemplateChild<gtk::ListBox>,
    #[template_child]
    pub profile_after_system_section: TemplateChild<gtk::Box>,
    #[template_child]
    pub profile_after_system_list: TemplateChild<gtk::ListBox>,
}

#[glib::object_subclass]
impl ObjectSubclass for UpdatesPage {
    const NAME: &'static str = "NscUpdatesPage";
    type Type = super::UpdatesPage;
    type ParentType = adw::BreakpointBin;

    fn class_init(klass: &mut Self::Class) {
        NscInstalledAppRow::ensure_type();
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for UpdatesPage {
    fn constructed(&self) {
        self.parent_constructed();

        let connect_row_activated = |list_box: &gtk::ListBox| {
            list_box.connect_row_activated(|_list_box, row| {
                let Some(row) = row.downcast_ref::<NscInstalledAppRow>() else {
                    return;
                };
                let Some(component) = row.component() else {
                    return;
                };

                let Some(app) = gio::Application::default().and_downcast::<NscApplication>() else {
                    return;
                };

                let metadata_ref = app.metadata().borrow();
                let Some(metadata) = metadata_ref.as_ref() else {
                    return;
                };

                let Some(nav_view) = crate::util::find_navigation_view(row) else {
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

        connect_row_activated(&self.nixos_updates_list);
        connect_row_activated(&self.hm_updates_list);
        connect_row_activated(&self.profile_now_list);
        connect_row_activated(&self.profile_after_system_list);

        let sort_by_name = |a: &gtk::ListBoxRow, b: &gtk::ListBoxRow| -> gtk::Ordering {
            let is_issue_a = a.has_css_class("issue-row");
            let is_issue_b = b.has_css_class("issue-row");

            let key_a = (
                is_issue_a,
                a.downcast_ref::<NscInstalledAppRow>()
                    .map(|r| r.name().to_lowercase()),
            );
            let key_b = (
                is_issue_b,
                b.downcast_ref::<NscInstalledAppRow>()
                    .map(|r| r.name().to_lowercase()),
            );
            match key_a.cmp(&key_b) {
                std::cmp::Ordering::Less => gtk::Ordering::Smaller,
                std::cmp::Ordering::Equal => gtk::Ordering::Equal,
                std::cmp::Ordering::Greater => gtk::Ordering::Larger,
            }
        };
        self.nixos_updates_list.set_sort_func(sort_by_name);
        self.hm_updates_list.set_sort_func(sort_by_name);
        self.profile_now_list.set_sort_func(sort_by_name);
        self.profile_after_system_list.set_sort_func(sort_by_name);
    }
}

impl WidgetImpl for UpdatesPage {}

impl BreakpointBinImpl for UpdatesPage {}
