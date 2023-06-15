use adw::gio;
use gtk::{prelude::ApplicationExt, glib};
use log::{error, info};
use nix_software_center::{ui::window::AppModel, config::RESOURCES_FILE};
use relm4::*;
fn main() {
    gtk::init().unwrap();
    pretty_env_logger::init();
	glib::set_application_name("Software Center");
    if let Ok(res) = gio::Resource::load(RESOURCES_FILE) {
        info!("Resource loaded: {}", RESOURCES_FILE);
        gio::resources_register(&res);
    } else {
        error!("Failed to load resources");
    }
    gtk::Window::set_default_icon_name(nix_software_center::config::APP_ID);
    let app = adw::Application::new(Some(nix_software_center::config::APP_ID), gio::ApplicationFlags::empty());
    app.set_resource_base_path(Some("/dev/vlinkz/NixSoftwareCenter"));
    let app = RelmApp::from_app(app);
    app.run::<AppModel>(());
}
