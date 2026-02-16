mod imp;

use gtk::glib;

glib::wrapper! {
    pub struct UpdatesPage(ObjectSubclass<imp::UpdatesPage>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
