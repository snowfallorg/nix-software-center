use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, glib};

#[derive(Debug, Default, CompositeTemplate)]
#[template(resource = "/org/snowflakeos/NixSoftwareCenter/ui/apply_dialog.ui")]
pub struct NscApplyDialog {
    #[template_child]
    pub status_page: TemplateChild<adw::StatusPage>,
    #[template_child]
    pub progress_bar: TemplateChild<gtk::ProgressBar>,
    #[template_child]
    pub close_button: TemplateChild<gtk::Button>,
    pub pulse_source: RefCell<Option<glib::SourceId>>,
    pub cancel_sender: RefCell<Option<tokio::sync::oneshot::Sender<()>>>,
}

#[glib::object_subclass]
impl ObjectSubclass for NscApplyDialog {
    const NAME: &'static str = "NscApplyDialog";
    type Type = super::NscApplyDialog;
    type ParentType = adw::Dialog;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for NscApplyDialog {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();

        let spinner =
            adw::SpinnerPaintable::new(Some(self.status_page.upcast_ref::<gtk::Widget>()));
        self.status_page.set_paintable(Some(&spinner));

        let dialog_weak = obj.downgrade();
        self.close_button.connect_clicked(move |_| {
            if let Some(d) = dialog_weak.upgrade() {
                d.set_can_close(true);
                d.close();
            }
        });

        obj.connect_close_attempt(|dialog| {
            let alert = adw::AlertDialog::new(
                Some("Cancel Apply?"),
                Some(
                    "The operation is still in progress. \
                     Are you sure you want to cancel?",
                ),
            );
            alert.add_response("continue", "Keep Running");
            alert.add_response("cancel", "Cancel");
            alert.set_response_appearance("cancel", adw::ResponseAppearance::Destructive);
            alert.set_default_response(Some("continue"));
            alert.set_close_response("continue");

            let dialog_weak = dialog.downgrade();
            alert.connect_response(None, move |_, response| {
                if response == "cancel"
                    && let Some(d) = dialog_weak.upgrade()
                {
                    if let Some(tx) = d.imp().cancel_sender.take() {
                        let _ = tx.send(());
                    }
                    d.set_can_close(true);
                    d.close();
                }
            });

            alert.present(Some(dialog));
        });
    }

    fn dispose(&self) {
        self.obj().stop_pulsing();
    }
}

impl WidgetImpl for NscApplyDialog {}
impl AdwDialogImpl for NscApplyDialog {}
