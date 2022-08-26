use nix_software_center::ui::window::AppModel;
use relm4::*;
fn main() {
    pretty_env_logger::init();
    let app = RelmApp::new("dev.vlinkz.NixSoftwareCenter");
    let application = app.app.clone();
    app.run::<AppModel>(application);
}
