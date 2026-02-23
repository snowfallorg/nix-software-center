mod imp;

use gtk::glib;
use gtk::subclass::prelude::ObjectSubclassIsExt;

glib::wrapper! {
    pub struct NscHeightClamp(ObjectSubclass<imp::NscHeightClamp>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for NscHeightClamp {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl NscHeightClamp {
    /// Animate `max-height` to a new value over 250ms.
    pub fn animate_max_height(&self, new_max_height: i32) {
        self.imp().animate_max_height(new_max_height);
    }
}
