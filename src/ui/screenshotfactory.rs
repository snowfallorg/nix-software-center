use relm4::adw::prelude::*;
use relm4::{factory::*, *};

use super::pkgpage::PkgMsg;

#[derive(Default, Debug, PartialEq, Eq)]
pub struct ScreenshotItem {
    pub path: Option<String>,
    pub error: bool,
}

#[derive(Debug)]
pub enum ScreenshotItemMsg {}

#[relm4::factory(pub)]
impl FactoryComponent for ScreenshotItem {
    type CommandOutput = ();
    type Init = ();
    type Input = ();
    type Output = ScreenshotItemMsg;
    type ParentWidget = adw::Carousel;
    type ParentInput = PkgMsg;

    view! {
        gtk::Box {
            set_margin_all: 15,
            set_halign: gtk::Align::Center,
            set_valign: gtk::Align::Fill,
            set_vexpand: true,
            gtk::Picture {
                #[watch]
                set_visible: self.path.is_some() && !self.error,
                #[watch]
                set_filename: self.path.as_ref(),
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
                set_hexpand: true,
                set_vexpand: true,
            },
            gtk::Spinner {
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
                set_hexpand: true,
                set_vexpand: true,
                #[watch]
                set_visible: self.path.is_none() && !self.error,
                set_spinning: true,
                set_height_request: 80,
                set_width_request: 80,
                set_margin_all: 30,
            },
            gtk::Image {
                add_css_class: "error",
                set_pixel_size: 64,
                set_icon_name: Some("dialog-error-symbolic"),
                #[watch]
                set_visible: self.error,
            }
        }
    }

    fn init_model(
        _parent: Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        Self {
            path: None,
            error: false,
        }
    }
}
