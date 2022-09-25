use adw::prelude::*;
use relm4::*;

use crate::config;

use super::window::AppMsg;

#[derive(Debug)]
pub struct AboutPageModel {
    hidden: bool,
}

#[derive(Debug)]
pub enum AboutPageMsg {
    Show,
}

#[relm4::component(pub)]
impl SimpleComponent for AboutPageModel {
    type InitParams = gtk::Window;
    type Input = AboutPageMsg;
    type Output = AppMsg;
    type Widgets = AboutPageWidgets;

    view! {
        adw::AboutWindow {
            #[watch]
            set_visible: !model.hidden,
            set_transient_for: Some(&parent_window),
            set_modal: true,
            set_application_name: "Nix Software Center",
            set_application_icon: config::APP_ID,
            set_developer_name: "Victor Fuentes",
            set_version: env!("CARGO_PKG_VERSION"),
            set_issue_url: "https://github.com/vlinkz/nix-software-center/issues",
            set_license_type: gtk::License::Gpl30Only,
            set_website: "https://github.com/vlinkz/nix-software-center",
            set_developers: &["Victor Fuentes https://github.com/vlinkz"],
        }
    }

    fn init(
        parent_window: Self::InitParams,
        root: &Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AboutPageModel {
            hidden: true,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AboutPageMsg::Show => {
                self.hidden = false;
            }
        }
    }

}