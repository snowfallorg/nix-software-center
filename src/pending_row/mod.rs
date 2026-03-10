mod imp;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use libappstream::prelude::ComponentExt;

use crate::pending_item::{ChangeKind, InstallTarget, PendingItem};

glib::wrapper! {
    pub struct NscPendingRow(ObjectSubclass<imp::NscPendingRow>)
        @extends gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl NscPendingRow {
    pub fn new(item: &PendingItem) -> Self {
        let row: Self = glib::Object::new();
        let imp = row.imp();

        if let Some(component) = item.component() {
            if let Some(name) = component.name() {
                imp.name_label.set_label(name.as_str());
            }

            crate::util::load_component_icon(&imp.icon, &component, &[48]);
            imp.component.replace(Some(component));
        }

        imp.target.set(item.target());

        let (kind_text, css_class) = match item.kind() {
            ChangeKind::Install => ("Install", "pending-install"),
            ChangeKind::Remove => ("Remove", "pending-remove"),
        };
        imp.kind_label.set_label(kind_text);
        row.add_css_class(css_class);

        row
    }

    pub fn component(&self) -> Option<libappstream::Component> {
        self.imp().component.borrow().clone()
    }

    pub fn target(&self) -> InstallTarget {
        self.imp().target.get()
    }

    pub fn connect_remove<F: Fn(&Self) + 'static>(&self, f: F) {
        let row_weak = self.downgrade();
        self.imp().remove_button.connect_clicked(move |_| {
            if let Some(row) = row_weak.upgrade() {
                f(&row);
            }
        });
    }
}
