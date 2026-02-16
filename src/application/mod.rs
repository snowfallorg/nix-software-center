mod imp;

use gettextrs::gettext;
use tracing::{info, warn};

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, gio, glib};
use libappstream::prelude::*;

use crate::config::{APP_ID, PKGDATADIR, PROFILE, VERSION};
use crate::runtime::runtime;
use crate::window::NscWindow;

glib::wrapper! {
    pub struct NscApplication(ObjectSubclass<imp::NscApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl NscApplication {
    fn main_window(&self) -> NscWindow {
        self.imp().window.get().unwrap().upgrade().unwrap()
    }

    fn setup_gactions(&self) {
        let action_quit = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| {
                app.main_window().close();
                app.quit();
            })
            .build();

        let action_about = gio::ActionEntry::builder("about")
            .activate(|app: &Self, _, _| {
                app.show_about_dialog();
            })
            .build();
        self.add_action_entries([action_quit, action_about]);
    }

    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<Control>q"]);
        self.set_accels_for_action("window.close", &["<Control>w"]);
    }

    fn setup_css(&self) {
        let provider = gtk::CssProvider::new();
        provider.load_from_resource("/org/snowflakeos/NixSoftwareCenter/style.css");
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    fn authors() -> Vec<&'static str> {
        // Authors are defined in Cargo.toml
        env!("CARGO_PKG_AUTHORS").split(":").collect()
    }

    fn show_about_dialog(&self) {
        let dialog = adw::AboutDialog::builder()
            .application_icon(*APP_ID)
            .version(*VERSION)
            .translator_credits(gettext("translator-credits"))
            .developers(Self::authors())
            .designers(vec!["Victor Fuentes"])
            .build();

        dialog.present(Some(&self.main_window()));
    }

    fn load_metadata(&self) {
        let (sender, receiver) = async_channel::bounded(1);

        runtime().spawn(async move {
            let result = libsnow::metadata::Metadata::connect().await;
            sender
                .send(result)
                .await
                .expect("The channel needs to be open.");
        });

        let app = self.clone();
        glib::spawn_future_local(async move {
            while let Ok(result) = receiver.recv().await {
                match result {
                    Ok(metadata) => {
                        info!("Metadata loaded successfully");
                        app.imp().metadata.replace(Some(metadata));
                    }
                    Err(err) => {
                        warn!("Failed to load metadata: {}", err);
                    }
                }
            }
        });
    }

    fn load_appstream(&self) {
        let data_path = match std::env::var("NSC_APPSTREAM_DATA") {
            Ok(path) => path,
            Err(_) => {
                warn!("NSC_APPSTREAM_DATA not set, skipping AppStream loading");
                return;
            }
        };

        let pool = libappstream::Pool::new();
        pool.set_load_std_data_locations(false);
        pool.add_extra_data_location(&data_path, libappstream::FormatStyle::Catalog);

        let app = self.clone();
        glib::spawn_future_local(async move {
            match pool.load_future().await {
                Ok(()) => {
                    let count = pool.components().map(|cbox| cbox.size()).unwrap_or(0);
                    info!("AppStream pool loaded: {} components", count);
                    app.imp().appstream_pool.replace(Some(pool));
                }
                Err(err) => {
                    warn!("Failed to load AppStream pool: {}", err);
                }
            }
        });
    }

    pub fn run(&self) -> glib::ExitCode {
        info!("Nix Software Center ({})", *APP_ID);
        info!("Version: {} ({})", *VERSION, *PROFILE);
        info!("Datadir: {}", *PKGDATADIR);

        ApplicationExtManual::run(self)
    }
}

impl Default for NscApplication {
    fn default() -> Self {
        glib::Object::builder()
            .property("application-id", *APP_ID)
            .property("resource-base-path", "/org/snowflakeos/NixSoftwareCenter/")
            .build()
    }
}
