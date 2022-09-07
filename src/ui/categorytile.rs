use std::path::Path;

use crate::APPINFO;

use super::categorypage::CategoryPageMsg;
use relm4::adw::prelude::*;
use relm4::gtk::pango;
use relm4::{factory::*, *};

#[derive(Default, Debug, PartialEq, Clone)]
pub struct CategoryTile {
    pub name: String,
    pub pkg: String,
    pub pname: String,
    pub summary: Option<String>,
    pub icon: Option<String>,
    pub installeduser: bool,
    pub installedsystem: bool,
}

#[derive(Debug)]
pub enum CategoryTileMsg {
    Open(String),
}

#[relm4::factory(pub)]
impl FactoryComponent for CategoryTile {
    type CommandOutput = ();
    type Init = CategoryTile;
    type Input = ();
    type Output = CategoryTileMsg;
    type Widgets = CategoryTileWidgets;
    type ParentWidget = gtk::FlowBox;
    type ParentMsg = CategoryPageMsg;

    view! {
        gtk::FlowBoxChild {
            set_width_request: 270,
            gtk::Overlay {
                add_overlay = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_valign: gtk::Align::Start,
                    set_halign: gtk::Align::End,
                    gtk::Image {
                        add_css_class: "accent",
                        set_valign: gtk::Align::Start,
                        set_halign: gtk::Align::End,
                        set_pixel_size: 16,
                        set_margin_top: 8,
                        set_margin_end: 8,
                        set_icon_name: Some("emblem-default-symbolic"),
                        #[watch]
                        set_visible: self.installeduser,
                    },
                    gtk::Image {
                        add_css_class: "success",
                        set_valign: gtk::Align::Start,
                        set_halign: gtk::Align::End,
                        set_pixel_size: 16,
                        set_margin_top: 8,
                        set_margin_end: 8,
                        set_icon_name: Some("emblem-default-symbolic"),
                        #[watch]
                        set_visible: self.installedsystem,
                    }
                },
                gtk::Button {
                    add_css_class: "card",
                    connect_clicked[sender, pkg = self.pkg.clone()] => move |_| {
                        println!("CLICKED");
                        sender.output(CategoryTileMsg::Open(pkg.to_string()))
                    },
                    set_can_focus: false,
                    gtk::Box {
                        set_margin_start: 15,
                        set_margin_end: 15,
                        set_margin_top: 10,
                        set_margin_bottom: 10,
                        set_spacing: 20,
                        append = if self.icon.is_some() {
                            gtk::Image {
                                add_css_class: "icon-dropshadow",
                                set_halign: gtk::Align::Start,
                                set_from_file: {
                                    if let Some(i) = &self.icon {
                                        let iconpath = format!("{}/icons/nixos/128x128/{}", APPINFO, i);
                                        let iconpath64 = format!("{}/icons/nixos/64x64/{}", APPINFO, i);
                                        if Path::new(&iconpath).is_file() {
                                            Some(iconpath)
                                        } else if Path::new(&iconpath64).is_file() {
                                            Some(iconpath64)
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                },
                                set_pixel_size: 64,
                            }
                        } else {
                            gtk::Image {
                                add_css_class: "icon-dropshadow",
                                set_halign: gtk::Align::Start,
                                set_icon_name: Some("package-x-generic"),
                                set_pixel_size: 64,
                            }
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_halign: gtk::Align::Fill,
                            set_valign: gtk::Align::Center,
                            set_hexpand: true,
                            set_spacing: 3,
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: "heading",
                                set_label: &self.name,
                                set_ellipsize: pango::EllipsizeMode::End,
                                set_lines: 1,
                                set_wrap: true,
                                set_max_width_chars: 0,
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: "dim-label",
                                add_css_class: "caption",
                                set_label: &self.pkg,
                                set_ellipsize: pango::EllipsizeMode::End,
                                set_lines: 1,
                                set_wrap: true,
                                set_max_width_chars: 0,
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                // add_css_class: "dim-label",
                                #[watch]
                                set_visible: self.summary.is_some(),
                                set_label: &(if let Some(s) = &self.summary { s.to_string() } else { String::default() }),
                                set_ellipsize: pango::EllipsizeMode::End,
                                set_lines: 2,
                                set_wrap: true,
                                set_max_width_chars: 0,
                            }
                        }
                    }
                }
            }
        }
    }

    fn init_model(
        parent: Self::Init,
        _index: &DynamicIndex,
        _sender: FactoryComponentSender<Self>,
    ) -> Self {
        let mut sum = parent.summary;
        sum = sum.map(|mut s| {
            s.trim().to_string();
            while s.contains('\n') {
                s = s.replace('\n', " ");
            }
            while s.contains("  ") {
                s = s.replace("  ", " ");
            }
            s
        });

        Self {
            name: parent.name,
            pkg: parent.pkg,
            pname: parent.pname,
            summary: sum,
            icon: parent.icon,
            installeduser: parent.installeduser,
            installedsystem: parent.installedsystem,
        }
    }

    fn output_to_parent_msg(output: Self::Output) -> Option<CategoryPageMsg> {
        Some(match output {
            CategoryTileMsg::Open(x) => CategoryPageMsg::OpenPkg(x),
        })
    }
}