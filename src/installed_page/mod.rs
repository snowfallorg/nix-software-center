mod imp;

use gtk::glib;

glib::wrapper! {
    pub struct InstalledPage(ObjectSubclass<imp::InstalledPage>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
