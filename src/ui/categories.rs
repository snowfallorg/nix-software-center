use relm4::adw::prelude::*;
use relm4::gtk::pango;
use relm4::{factory::*, *};
use strum_macros::{EnumIter, Display};

use super::window::AppMsg;

#[derive(Debug)]
pub struct PkgGroup {
    pub category: PkgCategory,
}

#[derive(Debug, Display, Hash, EnumIter, Eq, PartialEq, Clone)]
pub enum PkgCategory {
    Audio,
    Development,
    Games,
    Graphics,
    Network,
    Video,
}

#[derive(Debug)]
pub enum PkgCategoryMsg {
    Open(PkgCategory),
}

#[relm4::factory(pub)]
impl FactoryComponent for PkgGroup {
    type CommandOutput = ();
    type Init = PkgCategory;
    type Input = ();
    type Output = PkgCategoryMsg;
    type Widgets = PkgGroupWidgets;
    type ParentWidget = gtk::FlowBox;
    type ParentInput = AppMsg;

    view! {
        gtk::FlowBoxChild {
            set_width_request: 210,
            set_height_request: 70,
            gtk::Button {
                add_css_class: "card",
                gtk::Box {
                    set_margin_start: 15,
                    set_margin_end: 15,
                    set_margin_top: 10,
                    set_margin_bottom: 10,
                    set_spacing: 10,
                    set_halign: gtk::Align::Center,
                    gtk::Image {
                        add_css_class: "icon-dropshadow",
                        set_icon_name: match self.category {
                            PkgCategory::Audio => Some("audio-x-generic"),
                            PkgCategory::Development => Some("computer"),
                            PkgCategory::Games => Some("input-gaming"),
                            PkgCategory::Graphics => Some("image-x-generic"),
                            PkgCategory::Network => Some("network-server"),
                            PkgCategory::Video => Some("video-x-generic"),
                        },
                        set_pixel_size: 32,
                    },
                    gtk::Label {
                        add_css_class: "title-2",
                        set_valign: gtk::Align::Center,
                        set_hexpand: true,
                        set_label: match self.category {
                            PkgCategory::Audio => "Audio",
                            PkgCategory::Development => "Development",
                            PkgCategory::Games => "Games",
                            PkgCategory::Graphics => "Graphics",
                            PkgCategory::Network => "Network",
                            PkgCategory::Video => "Video",
                        },
                        set_ellipsize: pango::EllipsizeMode::End,
                        set_lines: 1,
                        set_wrap: true,
                        set_max_width_chars: 0,
                    }
                },
                connect_clicked[sender, category = self.category.clone()] => move |_| {
                    sender.output(PkgCategoryMsg::Open(category.clone()));
                }
            }
        }
    }

    fn init_model(
        parent: Self::Init,
        _index: &DynamicIndex,
        _sender: FactoryComponentSender<Self>,
    ) -> Self {
        Self {
            category: parent,
        }
    }

    fn output_to_parent_input(output: Self::Output) -> Option<AppMsg> {
        Some(match output {
            PkgCategoryMsg::Open(x) => AppMsg::OpenCategoryPage(x),
        })
    }

}
