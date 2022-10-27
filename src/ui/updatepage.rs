use crate::APPINFO;

use super::{pkgpage::InstallType, window::*, updatedialog::{UpdateDialogModel, UpdateDialogMsg}, updateworker::{UpdateAsyncHandler, UpdateAsyncHandlerMsg, UpdateAsyncHandlerInit}};
use adw::prelude::*;
use nix_data::config::configfile::NixDataConfig;
use relm4::{factory::*, gtk::pango, *};
use std::{path::Path, convert::identity};
use log::*;

#[tracker::track]
#[derive(Debug)]
pub struct UpdatePageModel {
    #[tracker::no_eq]
    updateuserlist: FactoryVecDeque<UpdateItemModel>,
    #[tracker::no_eq]
    updatesystemlist: FactoryVecDeque<UpdateItemModel>,
    channelupdate: Option<(String, String)>,
    #[tracker::no_eq]
    updatedialog: Controller<UpdateDialogModel>,
    #[tracker::no_eq]
    updateworker: WorkerController<UpdateAsyncHandler>,
    config: NixDataConfig,
    systype: SystemPkgs,
    usertype: UserPkgs,
    updatetracker: u8,
}

#[derive(Debug)]
pub enum UpdatePageMsg {
    UpdateConfig(NixDataConfig),
    UpdatePkgTypes(SystemPkgs, UserPkgs),
    Update(Vec<UpdateItem>, Vec<UpdateItem>),
    OpenRow(usize, InstallType),
    UpdateSystem,
    UpdateAllUser,
    UpdateUser(String),
    UpdateChannels,
    UpdateSystemAndChannels,
    UpdateAll,
    DoneWorking,
    DoneLoading,
    FailedWorking,
}

pub struct UpdatePageInit {
    pub window: gtk::Window,
    pub systype: SystemPkgs,
    pub usertype: UserPkgs,
    pub config: NixDataConfig,
}

#[relm4::component(pub)]
impl SimpleComponent for UpdatePageModel {
    type Init = UpdatePageInit;
    type Input = UpdatePageMsg;
    type Output = AppMsg;
    type Widgets = UpdatePageWidgets;

    view! {
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            #[track(model.changed(UpdatePageModel::updatetracker()))]
            set_vadjustment: gtk::Adjustment::NONE,
            adw::Clamp {
                if model.channelupdate.is_some() || !model.updateuserlist.is_empty() || !model.updatesystemlist.is_empty() {
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_valign: gtk::Align::Start,
                        set_margin_all: 15,
                        set_spacing: 15,
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_hexpand: true,
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: "title-2",
                                set_label: "Updates",
                            },
                            gtk::Button {
                                add_css_class: "suggested-action",
                                set_halign: gtk::Align::End,
                                set_valign: gtk::Align::Center,
                                set_hexpand: true,
                                set_label: "Update Everything",
                                connect_clicked[sender] => move |_| {
                                    sender.input(UpdatePageMsg::UpdateAll);
                                }
                            }
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_hexpand: true,
                            #[watch]
                            set_visible: model.channelupdate.is_some(),
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: "title-4",
                                set_label: "Channels",
                            },
                        },
                        gtk::ListBox {
                            set_valign: gtk::Align::Start,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            #[watch]
                            set_visible: model.channelupdate.is_some(),
                            adw::PreferencesRow {
                                set_activatable: false,
                                set_can_focus: false,
                                #[wrap(Some)]
                                set_child = &gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_hexpand: true,
                                    set_spacing: 10,
                                    set_margin_all: 10,
                                    adw::Bin {
                                        set_valign: gtk::Align::Center,
                                        gtk::Image {
                                            add_css_class: "icon-dropshadow",
                                            set_halign: gtk::Align::Start,
                                            set_icon_name: Some("application-x-addon"),
                                            set_pixel_size: 64,
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
                                            set_label: "nixos",
                                            set_ellipsize: pango::EllipsizeMode::End,
                                            set_lines: 1,
                                            set_wrap: true,
                                            set_max_width_chars: 0,
                                        },
                                        gtk::Label {
                                            set_halign: gtk::Align::Start,
                                            add_css_class: "dim-label",
                                            add_css_class: "caption",
                                            set_label: {
                                                &(if let Some((old, new)) = &model.channelupdate {
                                                    format!("{} → {}", old, new)
                                                } else {
                                                    String::default()
                                                })
                                            },
                                            set_visible: model.channelupdate.is_some(),
                                            set_ellipsize: pango::EllipsizeMode::End,
                                            set_lines: 1,
                                            set_wrap: true,
                                            set_max_width_chars: 0,
                                        },
                                    },
                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_spacing: 5,
                                        set_halign: gtk::Align::End,
                                        set_valign: gtk::Align::Center,
                                        gtk::Button {
                                            add_css_class: "suggested-action",
                                            set_valign: gtk::Align::Center,
                                            set_halign: gtk::Align::End,
                                            set_label: "Update channel and system",
                                            set_can_focus: false,
                                            connect_clicked[sender] => move |_| {
                                                sender.input(UpdatePageMsg::UpdateSystemAndChannels);
                                            }
                                        },
                                        gtk::Button {
                                            set_valign: gtk::Align::Center,
                                            set_halign: gtk::Align::End,
                                            set_label: "Update channel only",
                                            set_can_focus: false,
                                            connect_clicked[sender] => move |_| {
                                                sender.input(UpdatePageMsg::UpdateChannels);
                                            }
                                        },
                                    }
                                }
                            }
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_hexpand: true,
                            #[watch]
                            set_visible: !model.updateuserlist.is_empty(),
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: "title-4",
                                set_label: match model.usertype {
                                    UserPkgs::Env => "User (nix-env)",
                                    UserPkgs::Profile => "User (nix profile)",
                                }
                            },
                            gtk::Button {
                                add_css_class: "suggested-action",
                                set_halign: gtk::Align::End,
                                set_valign: gtk::Align::Center,
                                set_hexpand: true,
                                set_label: "Update All",
                                connect_clicked[sender] => move |_| {
                                    sender.input(UpdatePageMsg::UpdateAllUser);
                                }
                            }
                        },
                        #[local_ref]
                        updateuserlist -> gtk::ListBox {
                            set_valign: gtk::Align::Start,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            connect_row_activated[sender] => move |listbox, row| {
                                if let Some(i) = listbox.index_of_child(row) {
                                    sender.input(UpdatePageMsg::OpenRow(i as usize, InstallType::User));
                                }
                            },
                            #[watch]
                            set_visible: !model.updateuserlist.is_empty(),
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_hexpand: true,
                            #[watch]
                            set_visible: !model.updatesystemlist.is_empty(),
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: "title-4",
                                set_label: "System (configuration.nix)",
                            },
                            gtk::Button {
                                add_css_class: "suggested-action",
                                set_halign: gtk::Align::End,
                                set_hexpand: true,
                                set_valign: gtk::Align::Center,
                                set_label: "Update All",
                                connect_clicked[sender] => move |_|{
                                    sender.input(UpdatePageMsg::UpdateSystem);
                                },
                            }
                        },
                        #[local_ref]
                        updatesystemlist -> gtk::ListBox {
                            set_valign: gtk::Align::Start,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            connect_row_activated[sender] => move |listbox, row| {
                                if let Some(i) = listbox.index_of_child(row) {
                                    sender.input(UpdatePageMsg::OpenRow(i as usize, InstallType::System));
                                }
                            },
                            #[watch]
                            set_visible: !model.updatesystemlist.is_empty(),
                        }
                    }
                } else {
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_valign: gtk::Align::Center,
                        set_halign: gtk::Align::Center,
                        set_hexpand: true,
                        set_vexpand: true,
                        set_spacing: 10,
                        gtk::Image {
                            add_css_class: "success",
                            set_icon_name: Some("emblem-ok-symbolic"),
                            set_pixel_size: 256,
                        },
                        gtk::Label {
                            add_css_class: "title-1",
                            set_label: "Everything is up to date!"
                        }
                    }
                }
            }
        }
    }

    fn init(
        initparams: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let updatedialog = UpdateDialogModel::builder()
            .launch(initparams.window.upcast())
            .forward(sender.input_sender(), identity);
        let updateworker = UpdateAsyncHandler::builder()
            .detach_worker(UpdateAsyncHandlerInit { syspkgs: initparams.systype.clone(), userpkgs: initparams.usertype.clone() })
            .forward(sender.input_sender(), identity);

        let config = initparams.config;
        updateworker.emit(UpdateAsyncHandlerMsg::UpdateConfig(config.clone()));

        let model = UpdatePageModel {
            updateuserlist: FactoryVecDeque::new(gtk::ListBox::new(), sender.input_sender()),
            updatesystemlist: FactoryVecDeque::new(gtk::ListBox::new(), sender.input_sender()),
            channelupdate: None,
            updatetracker: 0,
            updatedialog,
            updateworker,
            config,
            systype: initparams.systype,
            usertype: initparams.usertype,
            tracker: 0,
        };

        let updateuserlist = model.updateuserlist.widget();
        let updatesystemlist = model.updatesystemlist.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.reset();
        match msg {
            UpdatePageMsg::UpdateConfig(config) => {
                self.config = config;
                self.updateworker.emit(UpdateAsyncHandlerMsg::UpdateConfig(self.config.clone()));
            }
            UpdatePageMsg::UpdatePkgTypes(systype, usertype) => {
                self.systype = systype;
                self.usertype = usertype;
                self.updateworker.emit(UpdateAsyncHandlerMsg::UpdatePkgTypes(self.systype.clone(), self.usertype.clone()));
            }
            UpdatePageMsg::Update(updateuserlist, updatesystemlist) => {
                info!("UpdatePageMsg::Update");
                debug!("UPDATEUSERLIST: {:?}", updateuserlist);
                debug!("UPDATESYSTEMLIST: {:?}", updatesystemlist);
                self.channelupdate = match nix_data::cache::channel::uptodate() {
                    Ok(x) => {
                        x
                    },
                    Err(_) => None,
                };
                debug!("CHANNELUPDATE: {:?}", self.channelupdate);
                self.update_updatetracker(|_| ());
                let mut updateuserlist_guard = self.updateuserlist.guard();
                updateuserlist_guard.clear();
                for updateuser in updateuserlist {
                    updateuserlist_guard.push_back(updateuser);
                }
                let mut updatesystemlist_guard = self.updatesystemlist.guard();
                updatesystemlist_guard.clear();
                for updatesystem in updatesystemlist {
                    updatesystemlist_guard.push_back(updatesystem);
                }
            }
            UpdatePageMsg::OpenRow(row, pkgtype) => match pkgtype {
                InstallType::User => {
                    let updateuserlist_guard = self.updateuserlist.guard();
                    if let Some(item) = updateuserlist_guard.get(row) {
                        if let Some(pkg) = &item.item.pkg {
                            sender.output(AppMsg::OpenPkg(pkg.to_string()));
                        }
                    }
                }
                InstallType::System => {
                    let updatesystemlist_guard = self.updatesystemlist.guard();
                    if let Some(item) = updatesystemlist_guard.get(row) {
                        if let Some(pkg) = &item.item.pkg {
                            sender.output(AppMsg::OpenPkg(pkg.to_string()));
                        }
                    }
                }
            },
            UpdatePageMsg::UpdateChannels => {
                self.updatedialog.emit(UpdateDialogMsg::Show(String::from("Updating channels...")));
                self.updateworker.emit(UpdateAsyncHandlerMsg::UpdateChannels);
            }
            UpdatePageMsg::UpdateSystemAndChannels => {
                self.updatedialog.emit(UpdateDialogMsg::Show(String::from("Updating system and channels...")));
                self.updateworker.emit(UpdateAsyncHandlerMsg::UpdateChannelsAndSystem);
            }
            UpdatePageMsg::UpdateSystem => {
                self.updatedialog.emit(UpdateDialogMsg::Show(String::from("Updating system...")));
                self.updateworker.emit(UpdateAsyncHandlerMsg::RebuildSystem);
            }
            UpdatePageMsg::UpdateUser(pkg) => {
                info!("UPDATE USER PKG: {}", pkg);
                warn!("unimplemented");
            }
            UpdatePageMsg::UpdateAllUser => {
                self.updatedialog.emit(UpdateDialogMsg::Show(String::from("Updating all user packages...")));
                self.updateworker.emit(UpdateAsyncHandlerMsg::UpdateUserPkgs);
            }
            UpdatePageMsg::UpdateAll => {
                self.updatedialog.emit(UpdateDialogMsg::Show(String::from("Updating everything...")));
                self.updateworker.emit(UpdateAsyncHandlerMsg::UpdateAll);
            }
            UpdatePageMsg::DoneWorking => {
                sender.output(AppMsg::UpdateInstalledPkgs);
            }
            UpdatePageMsg::DoneLoading => {
                self.updatedialog.emit(UpdateDialogMsg::Done);
            }
            UpdatePageMsg::FailedWorking => {
                self.updatedialog.emit(UpdateDialogMsg::Failed);
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct UpdateItem {
    pub name: String,
    pub pkg: Option<String>,
    pub pname: String,
    pub summary: Option<String>,
    pub icon: Option<String>,
    pub pkgtype: InstallType,
    pub verfrom: Option<String>,
    pub verto: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct UpdateItemModel {
    item: UpdateItem,
}

#[derive(Debug)]
pub enum UpdateItemMsg {}

#[relm4::factory(pub)]
impl FactoryComponent for UpdateItemModel {
    type CommandOutput = ();
    type Init = UpdateItem;
    type Input = ();
    type Output = UpdateItemMsg;
    type Widgets = UpdateItemWidgets;
    type ParentWidget = adw::gtk::ListBox;
    type ParentInput = UpdatePageMsg;

    view! {
        adw::PreferencesRow {
            set_activatable: self.item.pkg.is_some(),
            set_can_focus: false,
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
                        set_label: {
                            &(if let Some(old) = &self.item.verfrom {
                                if let Some(new) = &self.item.verto {
                                    format!("{} → {}", old, new)
                                } else {
                                    String::default()
                                }
                            } else {
                                String::default()
                            })
                        },
                        set_visible: self.item.verfrom.is_some() && self.item.verto.is_some(),
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
                },
                // gtk::Button {
                //     set_visible: self.item.pkgtype == InstallType::User,
                //     set_valign: gtk::Align::Center,
                //     set_halign: gtk::Align::End,
                //     set_label: "Update",
                //     set_can_focus: false,
                // }
            }
        }
    }

    fn init_model(
        parent: Self::Init,
        _index: &DynamicIndex,
        _sender: FactoryComponentSender<Self>,
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

        let item = UpdateItem {
            name: parent.name,
            pkg: parent.pkg,
            pname: parent.pname,
            summary: sum,
            icon: parent.icon,
            pkgtype: parent.pkgtype,
            verfrom: parent.verfrom,
            verto: parent.verto,
        };

        Self { item }
    }
}
