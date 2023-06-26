use std::path::Path;
use crate::APPINFO;

use super::{window::*, pkgpage::{InstallType, WorkPkg, PkgAction, NotifyPage}};
use adw::prelude::*;
use relm4::{factory::*, *, gtk::pango};

#[tracker::track]
#[derive(Debug)]
pub struct InstalledPageModel {
    #[tracker::no_eq]
    installeduserlist: FactoryVecDeque<InstalledItemModel>,
    #[tracker::no_eq]
    installedsystemlist: FactoryVecDeque<InstalledItemModel>,
    userpkgtype: UserPkgs,
    systempkgtype: SystemPkgs,
    updatetracker: u8,
}

#[derive(Debug)]
pub enum InstalledPageMsg {
    Update(Vec<InstalledItem>, Vec<InstalledItem>),
    UpdatePkgTypes(SystemPkgs, UserPkgs),
    OpenRow(usize, InstallType),
    Remove(InstalledItem),
    UnsetBusy(WorkPkg),
}

#[relm4::component(pub)]
impl SimpleComponent for InstalledPageModel {
    type Init = (SystemPkgs, UserPkgs);
    type Input = InstalledPageMsg;
    type Output = AppMsg;
    type Widgets = InstalledPageWidgets;

    view! {
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            #[track(model.changed(InstalledPageModel::updatetracker()))]
            set_vadjustment: gtk::Adjustment::NONE,
            adw::Clamp {
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_valign: gtk::Align::Start,
                    set_margin_all: 15,
                    set_spacing: 15,
                    gtk::Label {
                        #[watch]
                        set_visible: !model.installeduserlist.is_empty(),
                        set_halign: gtk::Align::Start,
                        add_css_class: "title-4",
                        set_label: match model.userpkgtype {
                            UserPkgs::Env => "User (nix-env)",
                            UserPkgs::Profile => "User (nix profile)",
                        },
                    },
                    #[local_ref]
                    installeduserlist -> gtk::ListBox {
                        #[watch]
                        set_visible: !model.installeduserlist.is_empty(),
                        set_valign: gtk::Align::Start,
                        add_css_class: "boxed-list",
                        set_selection_mode: gtk::SelectionMode::None,
                        connect_row_activated[sender] => move |listbox, row| {
                            if let Some(i) = listbox.index_of_child(row) {
                                sender.input(InstalledPageMsg::OpenRow(i as usize, InstallType::User))
                            }
                        }
                    },
                    gtk::Label {
                        #[watch]
                        set_visible: !model.installedsystemlist.is_empty(),
                        set_halign: gtk::Align::Start,
                        add_css_class: "title-4",
                        set_label: "System (configuration.nix)",
                    },
                    #[local_ref]
                    installedsystemlist -> gtk::ListBox {
                        #[watch]
                        set_visible: !model.installedsystemlist.is_empty(),
                        set_valign: gtk::Align::Start,
                        add_css_class: "boxed-list",
                        set_selection_mode: gtk::SelectionMode::None,
                        connect_row_activated[sender] => move |listbox, row| {
                            if let Some(i) = listbox.index_of_child(row) {
                                sender.input(InstalledPageMsg::OpenRow(i as usize, InstallType::System))
                            }
                        }
                    }
                }
            }
        }
    }

    fn init(
        (systempkgtype, userpkgtype): Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = InstalledPageModel {
            installeduserlist: FactoryVecDeque::new(gtk::ListBox::new(), sender.input_sender()),
            installedsystemlist: FactoryVecDeque::new(gtk::ListBox::new(), sender.input_sender()),
            updatetracker: 0,
            userpkgtype,
            systempkgtype,
            tracker: 0
        };

        let installeduserlist = model.installeduserlist.widget();
        let installedsystemlist = model.installedsystemlist.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.reset();
        match msg {
            InstalledPageMsg::Update(installeduserlist, installedsystemlist) => {
                self.update_updatetracker(|_| ());
                let mut installeduserlist_guard = self.installeduserlist.guard();
                installeduserlist_guard.clear();
                for installeduser in installeduserlist {
                    installeduserlist_guard.push_back(installeduser);
                }
                let mut installedsystemlist_guard = self.installedsystemlist.guard();
                installedsystemlist_guard.clear();
                for installedsystem in installedsystemlist {
                    installedsystemlist_guard.push_back(installedsystem);
                }
            }
            InstalledPageMsg::UpdatePkgTypes(systempkgtype, userpkgtype) => {
                self.systempkgtype = systempkgtype;
                self.userpkgtype = userpkgtype;
            }
            InstalledPageMsg::OpenRow(row, pkgtype) => {
                match pkgtype {
                    InstallType::User => {
                        let installeduserlist_guard = self.installeduserlist.guard();
                        if let Some(item) = installeduserlist_guard.get(row) {
                            if let Some(pkg) = &item.item.pkg {
                                sender.output(AppMsg::OpenPkg(pkg.to_string()));
                            }
                        }
                    }
                    InstallType::System => {
                        let installedsystemlist_guard = self.installedsystemlist.guard();
                        if let Some(item) = installedsystemlist_guard.get(row) {
                            if let Some(pkg) = &item.item.pkg {
                                sender.output(AppMsg::OpenPkg(pkg.to_string()));
                            }
                        }
                    }
                }
            }
            InstalledPageMsg::Remove(item) => {
                let work = WorkPkg {
                    pkg: item.pkg.unwrap_or_default(),
                    pname: item.pname,
                    pkgtype: item.pkgtype,
                    action: PkgAction::Remove,
                    block: false,
                    notify: Some(NotifyPage::Installed)
                };
                sender.output(AppMsg::AddInstalledToWorkQueue(work));
            }
            InstalledPageMsg::UnsetBusy(work) => {
                match work.pkgtype {
                    InstallType::User => {
                        let mut installeduserlist_guard = self.installeduserlist.guard();
                        for i in 0..installeduserlist_guard.len() {
                            if let Some(item) = installeduserlist_guard.get_mut(i) {
                                if item.item.pname == work.pname && item.item.pkgtype == work.pkgtype {
                                    item.item.busy = false;
                                }
                            }
                        }
                    }
                    InstallType::System => {
                        let mut installedsystemlist_guard = self.installedsystemlist.guard();
                        for i in 0..installedsystemlist_guard.len() {
                            if let Some(item) = installedsystemlist_guard.get_mut(i) {
                                if item.item.pkg == Some(work.pkg.clone()) && item.item.pkgtype == work.pkgtype {
                                    item.item.busy = false;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}




#[derive(Debug, PartialEq, Eq, Clone)]
pub struct InstalledItem {
    pub name: String,
    pub pkg: Option<String>,
    pub pname: String,
    pub summary: Option<String>,
    pub icon: Option<String>,
    pub pkgtype: InstallType,
    pub busy: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct InstalledItemModel {
    pub item: InstalledItem,
}

#[derive(Debug)]
pub enum InstalledItemMsg {
    Delete(InstalledItem),
}

#[derive(Debug)]
pub enum InstalledItemInputMsg {
    Busy(bool),
}

#[relm4::factory(pub)]
impl FactoryComponent for InstalledItemModel {
    type CommandOutput = ();
    type Init = InstalledItem;
    type Input = InstalledItemInputMsg;
    type Output = InstalledItemMsg;
    type ParentWidget = adw::gtk::ListBox;
    type ParentInput = InstalledPageMsg;

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
                        set_label: if let Some(p) = &self.item.pkg { p } else { &self.item.pname },
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
                if self.item.busy {
                    gtk::Spinner {
                        set_spinning: true,
                    }
                } else {
                    gtk::Button {
                        add_css_class: "destructive-action",
                        set_valign: gtk::Align::Center,
                        set_halign: gtk::Align::End,
                        set_icon_name: "user-trash-symbolic",
                        set_can_focus: false,
                        connect_clicked[sender, item = self.item.clone()] => move |_| {
                            sender.input(InstalledItemInputMsg::Busy(true));
                            sender.output(InstalledItemMsg::Delete(item.clone()))
                        }
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

        let item = InstalledItem {
            name: parent.name,
            pkg: parent.pkg,
            pname: parent.pname,
            summary: sum,
            icon: parent.icon,
            pkgtype: parent.pkgtype,
            busy: parent.busy,
        };

        Self {
            item,
        }
    }

    fn forward_to_parent(output: Self::Output) -> Option<InstalledPageMsg> {
        Some(match output {
            InstalledItemMsg::Delete(item) => InstalledPageMsg::Remove(item),
        })
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            InstalledItemInputMsg::Busy(b) => self.item.busy = b,
        }
    }

}
