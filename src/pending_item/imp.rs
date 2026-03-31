use std::cell::{Cell, RefCell};

use glib::subclass::prelude::*;
use gtk::glib;

use super::{ChangeKind, InstallTarget};

#[derive(Default)]
pub struct PendingItem {
    pub component: RefCell<Option<libappstream::Component>>,
    pub kind: Cell<ChangeKind>,
    pub target: Cell<InstallTarget>,
}

#[glib::object_subclass]
impl ObjectSubclass for PendingItem {
    const NAME: &'static str = "NscPendingItem";
    type Type = super::PendingItem;
    type ParentType = glib::Object;
}

impl ObjectImpl for PendingItem {}
