use relm4::adw::prelude::*;
use relm4::gtk::pango;
use relm4::{factory::*, *};

use super::window::AppMsg;

#[derive(Debug)]
pub struct PkgGroup {
    pub category: PkgCategory,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum PkgCategory {
    Audio,
    Development,
    Games,
    Graphics,
    Web,
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
                            PkgCategory::Audio => Some("nsc-audio"),
                            PkgCategory::Development => Some("nsc-development"),
                            PkgCategory::Games => Some("nsc-gaming"),
                            PkgCategory::Graphics => Some("nsc-graphics"),
                            PkgCategory::Web => Some("nsc-web"),
                            PkgCategory::Video => Some("nsc-video"),
                        },
                        set_pixel_size: 40,
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
                            PkgCategory::Web => "Web",
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
        _sender: FactorySender<Self>,
    ) -> Self {
        Self {
            category: parent,
        }
    }

    fn forward_to_parent(output: Self::Output) -> Option<AppMsg> {
        Some(match output {
            PkgCategoryMsg::Open(x) => AppMsg::OpenCategoryPage(x),
        })
    }

}
