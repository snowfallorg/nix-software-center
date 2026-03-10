use gtk::subclass::prelude::*;
use gtk::{gio, glib, prelude::*};

use crate::pending_item::PendingItem;

pub struct PendingChanges {
    pub store: gio::ListStore,
}

impl Default for PendingChanges {
    fn default() -> Self {
        Self {
            store: gio::ListStore::new::<PendingItem>(),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for PendingChanges {
    const NAME: &'static str = "NscPendingChanges";
    type Type = super::PendingChanges;
    type ParentType = glib::Object;
    type Interfaces = (gio::ListModel,);
}

impl ObjectImpl for PendingChanges {
    fn constructed(&self) {
        self.parent_constructed();

        let obj_weak = self.obj().downgrade();
        self.store
            .connect_items_changed(move |_, pos, removed, added| {
                if let Some(obj) = obj_weak.upgrade() {
                    obj.items_changed(pos, removed, added);
                }
            });
    }
}

impl ListModelImpl for PendingChanges {
    fn item_type(&self) -> glib::Type {
        PendingItem::static_type()
    }

    fn n_items(&self) -> u32 {
        self.store.n_items()
    }

    fn item(&self, position: u32) -> Option<glib::Object> {
        self.store.item(position)
    }
}
