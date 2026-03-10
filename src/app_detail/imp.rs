use std::cell::{Cell, RefCell};

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use glib::translate::IntoGlib;
use gtk::{CompositeTemplate, glib, pango};

use crate::height_clamp::NscHeightClamp;

/// The collapsed height for the description clamp (in pixels)
const DESCRIPTION_COLLAPSED_HEIGHT: i32 = 180;

#[derive(Debug, Default, CompositeTemplate)]
#[template(resource = "/org/snowflakeos/NixSoftwareCenter/ui/app_detail.ui")]
pub struct NscAppDetail {
    pub component: RefCell<Option<libappstream::Component>>,

    // Header bar
    #[template_child]
    pub detail_headerbar: TemplateChild<adw::HeaderBar>,
    #[template_child]
    pub sidebar_button: TemplateChild<gtk::ToggleButton>,

    // Header
    #[template_child]
    pub icon: TemplateChild<gtk::Image>,
    #[template_child]
    pub name_label: TemplateChild<gtk::Label>,
    #[template_child]
    pub summary_label: TemplateChild<gtk::Label>,
    #[template_child]
    pub developer_label: TemplateChild<gtk::Label>,
    #[template_child]
    pub support_button: TemplateChild<gtk::Button>,

    // Actions
    #[template_child]
    pub install_button: TemplateChild<gtk::Button>,
    #[template_child]
    pub trash_button: TemplateChild<gtk::Button>,
    #[template_child]
    pub run_button: TemplateChild<gtk::Button>,
    #[template_child]
    pub target_dropdown: TemplateChild<gtk::DropDown>,

    pub installed_nixos: Cell<bool>,
    pub installed_hm: Cell<bool>,

    pub run_cancel: RefCell<Option<crate::app_detail::RunCancel>>,

    pub pending_changed_handler: RefCell<
        Option<(
            glib::SignalHandlerId,
            crate::pending_changes::PendingChanges,
        )>,
    >,

    // Screenshots
    #[template_child]
    pub screenshot_box: TemplateChild<gtk::Box>,
    #[template_child]
    pub screenshot_carousel: TemplateChild<adw::Carousel>,
    #[template_child]
    pub screenshot_dots: TemplateChild<adw::CarouselIndicatorDots>,
    #[template_child]
    pub screenshot_prev: TemplateChild<gtk::Button>,
    #[template_child]
    pub screenshot_prev_revealer: TemplateChild<gtk::Revealer>,
    #[template_child]
    pub screenshot_next: TemplateChild<gtk::Button>,
    #[template_child]
    pub screenshot_next_revealer: TemplateChild<gtk::Revealer>,

    // Description
    #[template_child]
    pub description_section: TemplateChild<gtk::Box>,
    #[template_child]
    pub description_clamp: TemplateChild<NscHeightClamp>,
    #[template_child]
    pub description_view: TemplateChild<gtk::TextView>,
    #[template_child]
    pub description_toggle: TemplateChild<gtk::Button>,

    // Details
    #[template_child]
    pub details_group: TemplateChild<adw::PreferencesGroup>,
    #[template_child]
    pub version_row: TemplateChild<adw::ActionRow>,
    #[template_child]
    pub version_label: TemplateChild<gtk::Label>,
    #[template_child]
    pub license_row: TemplateChild<adw::ActionRow>,
    #[template_child]
    pub license_label: TemplateChild<gtk::Label>,
    #[template_child]
    pub package_row: TemplateChild<adw::ActionRow>,
    #[template_child]
    pub package_label: TemplateChild<gtk::Label>,

    // Links
    #[template_child]
    pub links_group: TemplateChild<adw::PreferencesGroup>,
    #[template_child]
    pub homepage_row: TemplateChild<adw::ActionRow>,
    #[template_child]
    pub bugtracker_row: TemplateChild<adw::ActionRow>,
    #[template_child]
    pub help_row: TemplateChild<adw::ActionRow>,
    #[template_child]
    pub donate_row: TemplateChild<adw::ActionRow>,
}

#[glib::object_subclass]
impl ObjectSubclass for NscAppDetail {
    const NAME: &'static str = "NscAppDetail";
    type Type = super::NscAppDetail;
    type ParentType = adw::NavigationPage;

    fn class_init(klass: &mut Self::Class) {
        NscHeightClamp::ensure_type();
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for NscAppDetail {
    fn dispose(&self) {
        super::NscAppDetail::disconnect_pending_changed(self);
    }

    fn constructed(&self) {
        self.parent_constructed();

        let buffer = self.description_view.buffer();
        let tag_table = buffer.tag_table();

        let paragraph = gtk::TextTag::builder()
            .name("paragraph")
            .pixels_below_lines(10)
            .build();
        tag_table.add(&paragraph);

        let bold = gtk::TextTag::builder()
            .name("bold")
            .weight(pango::Weight::Bold.into_glib())
            .build();
        tag_table.add(&bold);

        let monospace = gtk::TextTag::builder()
            .name("monospace")
            .family("monospace")
            .build();
        tag_table.add(&monospace);

        let list_item = gtk::TextTag::builder()
            .name("list-item")
            .pixels_below_lines(5)
            .build();
        tag_table.add(&list_item);

        self.description_clamp
            .bind_property("will-change", &*self.description_toggle, "visible")
            .sync_create()
            .build();

        let sidebar_button = self.sidebar_button.clone();
        self.obj().connect_map(move |page| {
            let mut ancestor = page.parent();
            while let Some(widget) = ancestor {
                if let Ok(split_view) = widget.clone().downcast::<adw::OverlaySplitView>() {
                    split_view
                        .bind_property("show-sidebar", &sidebar_button, "active")
                        .bidirectional()
                        .sync_create()
                        .build();
                    return;
                }
                ancestor = widget.parent();
            }
        });

        // Screenshot carousel navigation buttons
        let carousel = self.screenshot_carousel.clone();
        self.screenshot_prev.connect_clicked(move |_| {
            let pos = carousel.position().round() as u32;
            if pos > 0
                && let Some(child) = nth_carousel_child(&carousel, pos - 1)
            {
                carousel.scroll_to(&child, true);
            }
        });

        let carousel = self.screenshot_carousel.clone();
        self.screenshot_next.connect_clicked(move |_| {
            let pos = carousel.position().round() as u32;
            if pos + 1 < carousel.n_pages()
                && let Some(child) = nth_carousel_child(&carousel, pos + 1)
            {
                carousel.scroll_to(&child, true);
            }
        });

        let prev_rev = self.screenshot_prev_revealer.clone();
        let next_rev = self.screenshot_next_revealer.clone();
        self.screenshot_carousel
            .connect_position_notify(move |carousel| {
                let n = carousel.n_pages();
                let pos = carousel.position();
                let show_prev = n > 1 && pos > 0.5;
                let show_next = n > 1 && pos < (n as f64) - 1.5;
                prev_rev.set_reveal_child(show_prev);
                prev_rev.set_can_target(show_prev);
                next_rev.set_reveal_child(show_next);
                next_rev.set_can_target(show_next);
            });

        let obj = self.obj().clone();
        self.description_toggle.connect_clicked(move |button| {
            let imp = obj.imp();
            let expanded = imp.description_clamp.max_height() == -1;

            if expanded {
                imp.description_clamp
                    .animate_max_height(DESCRIPTION_COLLAPSED_HEIGHT);
                button.set_label("Show More");
            } else {
                imp.description_clamp.animate_max_height(-1);
                button.set_label("Show Less");
            }
        });
    }
}

impl WidgetImpl for NscAppDetail {}

impl NavigationPageImpl for NscAppDetail {}

/// Get the nth child widget of a carousel by walking the sibling chain
fn nth_carousel_child(carousel: &adw::Carousel, index: u32) -> Option<gtk::Widget> {
    let mut child = carousel.first_child();
    for _ in 0..index {
        child = child?.next_sibling();
    }
    child
}
