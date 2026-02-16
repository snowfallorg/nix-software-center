mod imp;

use gtk::glib;

glib::wrapper! {
    pub struct ExplorePage(ObjectSubclass<imp::ExplorePage>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
