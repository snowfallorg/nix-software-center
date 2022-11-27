use std::{path::Path, collections::HashSet};
use crate::APPINFO;

use super::window::*;
use adw::prelude::*;
use relm4::{factory::*, *, gtk::pango};
use log::*;

#[tracker::track]
#[derive(Debug)]
pub struct SearchPageModel {
    #[tracker::no_eq]
    searchitems: FactoryVecDeque<SearchItemModel>,
    searchitemtracker: u8,
}

#[derive(Debug)]
pub enum SearchPageMsg {
    Search(Vec<SearchItem>),
    UpdateInstalled(HashSet<String>, HashSet<String>),
    OpenRow(gtk::ListBoxRow)
}

#[relm4::component(pub)]
impl SimpleComponent for SearchPageModel {
    type Init = ();
    type Input = SearchPageMsg;
    type Output = AppMsg;

    view! {
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            #[track(model.changed(SearchPageModel::searchitemtracker()))]
            set_vadjustment: gtk::Adjustment::NONE,
            adw::Clamp {
                gtk::Stack {
                    set_margin_all: 20,
                    #[local_ref]
                    searchlist -> gtk::ListBox {
                        set_valign: gtk::Align::Start,
                        add_css_class: "boxed-list",
                        set_selection_mode: gtk::SelectionMode::None,
                        connect_row_activated[sender] => move |_, row| {
                            sender.input(SearchPageMsg::OpenRow(row.clone()));
                        }
                    }
                }
            }
        }
    }

    fn init(
        (): Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = SearchPageModel {
            searchitems: FactoryVecDeque::new(gtk::ListBox::new(), sender.input_sender()),
            searchitemtracker: 0,
            tracker: 0,
        };

        let searchlist = model.searchitems.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.reset();
        match msg {
            SearchPageMsg::Search(items) => {
                let mut searchitem_guard = self.searchitems.guard();
                searchitem_guard.clear();
                for item in items {
                    searchitem_guard.push_back(item);
                }
                searchitem_guard.drop();
                self.update_searchitemtracker(|_| ());
            }
            SearchPageMsg::OpenRow(row) => {
                let searchitem_guard = self.searchitems.guard();
                for (i, child) in searchitem_guard.widget().iter_children().enumerate() {
                    if child == row {
                        if let Some(item) = searchitem_guard.get(i) {
                            let pkg = &item.get_item().pkg;
                            sender.output(AppMsg::OpenPkg(pkg.to_string()));
                        }
                    }
                }
            }
            SearchPageMsg::UpdateInstalled(installeduserpkgs, installedsystempkgs) => {
                let mut searchitem_guard = self.searchitems.guard();
                for i in 0..searchitem_guard.len() {
                    if let Some(item) = searchitem_guard.get_mut(i) {
                        let mut pkgitem = item.get_mut_item();
                        pkgitem.installeduser = installeduserpkgs.contains(&pkgitem.pname.to_string());
                        pkgitem.installedsystem = installedsystempkgs.contains(&pkgitem.pkg.to_string());

                    }
                }
            }
        }
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct SearchItem {
    pub name: String,
    pub pkg: String,
    pub pname: String,
    pub summary: Option<String>,
    pub icon: Option<String>,
    pub installeduser: bool,
    pub installedsystem: bool,
}

#[tracker::track]
#[derive(Default, Debug, PartialEq, Eq)]
pub struct SearchItemModel {
    pub item: SearchItem,
}

#[derive(Debug)]
pub enum SearchItemMsg {}

#[relm4::factory(pub)]
impl FactoryComponent for SearchItemModel {
    type CommandOutput = ();
    type Init = SearchItem;
    type Input = ();
    type Output = SearchItemMsg;
    type ParentWidget = adw::gtk::ListBox;
    type ParentInput = SearchPageMsg;

    view! {
        adw::PreferencesRow {
            #[wrap(Some)]
            set_child = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_hexpand: true,
                set_spacing: 10,
                set_margin_all: 10,
                adw::Bin {
                    set_valign: gtk::Align::Center,
                    #[wrap(Some)]
                    set_child = if self.item.icon.is_some() {
                        gtk::Image {
                            add_css_class: "icon-dropshadow",
                            set_halign: gtk::Align::Start,
                            set_from_file: {
                                if let Some(i) = &self.item.icon {
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
                    }
                },
                gtk::Overlay {
                    add_overlay = &gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_valign: gtk::Align::Start,
                        set_halign: gtk::Align::End,
                        gtk::Image {
                            add_css_class: "accent",
                            set_valign: gtk::Align::Start,
                            set_halign: gtk::Align::End,
                            set_icon_name: Some("emblem-default-symbolic"),
                            #[watch]
                            set_visible: self.item.installeduser,
                        },
                        gtk::Image {
                            add_css_class: "success",
                            set_valign: gtk::Align::Start,
                            set_halign: gtk::Align::End,
                            set_icon_name: Some("emblem-default-symbolic"),
                            #[watch]
                            set_visible: self.item.installedsystem,
                        }
                    },
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_halign: gtk::Align::Fill,
                        set_valign: gtk::Align::Center,
                        set_hexpand: true,
                        set_spacing: 2,
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_label: self.item.name.as_str(),
                            set_ellipsize: pango::EllipsizeMode::End,
                            set_lines: 1,
                            set_wrap: true,
                            set_max_width_chars: 0,
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            add_css_class: "dim-label",
                            add_css_class: "caption",
                            set_label: self.item.pkg.as_str(),
                            set_ellipsize: pango::EllipsizeMode::End,
                            set_lines: 1,
                            set_wrap: true,
                            set_max_width_chars: 0,
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_label: self.item.summary.as_deref().unwrap_or(""),
                            set_visible: self.item.summary.is_some(),
                            set_ellipsize: pango::EllipsizeMode::End,
                            set_lines: 1,
                            set_wrap: true,
                            set_max_width_chars: 0,
                        },
                    }
                }
            }
        }
    }

    fn init_model(
        parent: Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        let sum = if let Some(s) = parent.summary {
            let mut sum = s.trim().to_string();
            while sum.contains('\n') {
                sum = sum.replace('\n', " ");
            }
            while sum.contains("  ") {
                sum = sum.replace("  ", " ");
            }
            Some(sum)
        } else {
            None
        };

        let item = SearchItem {
            name: parent.name,
            pkg: parent.pkg,
            pname: parent.pname,
            summary: sum,
            icon: parent.icon,
            installeduser: parent.installeduser,
            installedsystem: parent.installedsystem,
        };

        Self { item, tracker: 0 }
    }
}
