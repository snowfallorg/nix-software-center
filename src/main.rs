use adw::gio;
use nix_software_center::ui::window::AppModel;
use relm4::*;
fn main() {
    pretty_env_logger::init();
    let res = gio::Resource::load("@PKGDATA_DIR@".to_string() + "/resources.gresource").unwrap();
    gio::resources_register(&res);
    let app = RelmApp::new("dev.vlinkz.NixSoftwareCenter");
    let application = app.app.clone();
    app.run::<AppModel>(application);
}
