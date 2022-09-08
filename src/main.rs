use adw::gio;
use nix_software_center::ui::window::AppModel;
use relm4::*;
use nix_software_center::config::PKGDATADIR;
fn main() {
    pretty_env_logger::init();
    if let Ok(res) = gio::Resource::load(PKGDATADIR.to_string() + "/resources.gresource") {
        gio::resources_register(&res);
    }
    let app = RelmApp::new(nix_software_center::config::APP_ID);
    let application = app.app.clone();
    app.run::<AppModel>(application);
}
