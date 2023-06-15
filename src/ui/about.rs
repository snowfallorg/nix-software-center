use adw::prelude::*;
use relm4::*;

use crate::config;

#[derive(Debug)]
pub struct AboutPageModel {}

#[derive(Debug)]
pub enum AboutPageMsg {
    Show,
    Hide,
}

pub struct Widgets {
    parent_window: gtk::Window,
}

impl SimpleComponent for AboutPageModel {
    type Init = gtk::Window;
    type Widgets = Widgets;
    type Input = ();
    type Output = ();
    type Root = ();

    fn init_root() -> Self::Root {}

    fn init(
        parent_window: Self::Init,
        _root: &Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {};

        let widgets = Widgets { parent_window };

        ComponentParts { model, widgets }
    }

    fn update_view(&self, dialog: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        let dialog = adw::AboutWindow::builder()
            .application_icon(config::APP_ID)
            .application_name("Nix Software Center")
            .developer_name("Victor Fuentes")
            .developers(vec!["Victor Fuentes https://github.com/vlinkz"])
            .issue_url("https://github.com/vlinkz/nix-software-center/issues")
            .license_type(gtk::License::Gpl30)
            .modal(true)
            .transient_for(&dialog.parent_window)
            .version(config::VERSION)
            .website("https://github.com/vlinkz/nix-software-center")
            .build();
        dialog.present();
    }
}
