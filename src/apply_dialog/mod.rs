mod imp;

use adw::prelude::*;
use glib::subclass::prelude::*;
use gtk::{gdk, glib};

glib::wrapper! {
    pub struct NscApplyDialog(ObjectSubclass<imp::NscApplyDialog>)
        @extends adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl NscApplyDialog {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn present_apply(&self, parent: &impl IsA<gtk::Widget>) {
        self.start_pulsing();
        self.present(Some(parent));
    }

    pub fn set_success(&self) {
        self.stop_pulsing();
        let imp = self.imp();
        imp.status_page.set_paintable(gdk::Paintable::NONE);
        imp.status_page.set_icon_name(Some("nsc-success-symbolic"));
        imp.status_page.set_title("Changes Applied");
        imp.status_page
            .set_description(Some("Your system configuration has been updated"));
        imp.progress_bar.set_visible(false);
        imp.close_button.set_visible(true);
        self.set_can_close(true);
    }

    pub fn set_failed(&self, error: &str) {
        self.stop_pulsing();
        let imp = self.imp();
        imp.status_page.set_paintable(gdk::Paintable::NONE);
        imp.status_page.set_icon_name(Some("nsc-failed-symbolic"));
        imp.status_page.set_title("Failed to Apply Changes");
        imp.status_page.set_description(Some(error));
        imp.progress_bar.set_visible(false);
        imp.close_button.set_visible(true);
        self.set_can_close(true);
    }

    fn start_pulsing(&self) {
        let pb = self.imp().progress_bar.clone();
        let source_id = glib::timeout_add_local(std::time::Duration::from_millis(80), move || {
            pb.pulse();
            glib::ControlFlow::Continue
        });
        self.imp().pulse_source.replace(Some(source_id));
    }

    fn stop_pulsing(&self) {
        if let Some(id) = self.imp().pulse_source.take() {
            id.remove();
        }
    }
}
