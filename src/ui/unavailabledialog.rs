use std::path::Path;

use gtk::pango;
use log::*;
use relm4::{*, prelude::*, factory::*};
use adw::prelude::*;
use crate::{APPINFO, ui::{window::REBUILD_BROKER, rebuild::RebuildMsg}};

use super::updatepage::{UpdatePageMsg, UpdateType};

#[derive(Debug)]
pub struct UnavailableDialogModel {
    hidden: bool,
    unavailableuseritems: FactoryVecDeque<UnavailableItemModel>,
    unavailablesysitems: FactoryVecDeque<UnavailableItemModel>,
    updatetype: UpdateType,
}

#[derive(Debug)]
pub enum UnavailableDialogMsg {
    Show(Vec<UnavailableItemModel>, Vec<UnavailableItemModel>, UpdateType),
    Close,
    Continue,
}

#[relm4::component(pub)]
impl SimpleComponent for UnavailableDialogModel {
    type Init = gtk::Window;
    type Input = UnavailableDialogMsg;
    type Output = UpdatePageMsg;

    view! {
        dialog = adw::MessageDialog {
            #[watch]
            set_visible: !model.hidden,
            set_transient_for: Some(&parent_window),
            set_modal: true,
            set_heading: Some("Some packages are unavailable!"),
            set_body: "If you continue this update, some packages will be removed",
            #[wrap(Some)]
            set_extra_child = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 20,
                adw::PreferencesGroup {
                    #[watch]
                    set_visible: !model.unavailableuseritems.is_empty(),
                    set_title: "User Packages",
                    #[local_ref]
                    unavailableuserlist -> gtk::ListBox {
                        add_css_class: "boxed-list",
                        set_selection_mode: gtk::SelectionMode::None,
                    },
                },
                adw::PreferencesGroup {
                    #[watch]
                    set_visible: !model.unavailablesysitems.is_empty(),
                    set_title: "System Packages",
                    #[local_ref]
                    unavailablesyslist -> gtk::ListBox {
                        add_css_class: "boxed-list",
                        set_selection_mode: gtk::SelectionMode::None
                    },
                }
            },
            add_response: ("cancel", "Cancel"),
            add_response: ("continue", "Continue"),
            set_response_appearance: ("continue", adw::ResponseAppearance::Destructive),
            connect_close_request => |_| {
                gtk::Inhibit(true)
            }
        }
    }

    fn init(
        parent_window: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {

        let model = UnavailableDialogModel {
            unavailableuseritems: FactoryVecDeque::new(gtk::ListBox::new(), sender.input_sender()),
            unavailablesysitems: FactoryVecDeque::new(gtk::ListBox::new(), sender.input_sender()),
            updatetype: UpdateType::All,
            hidden: true,
        };

        let unavailableuserlist = model.unavailableuseritems.widget();
        let unavailablesyslist = model.unavailablesysitems.widget();

        let widgets = view_output!();

        widgets.dialog.connect_response(None, move |_, resp| {
            match resp {
                "cancel" => {
                    REBUILD_BROKER.send(RebuildMsg::Close);
                    debug!("Response: cancel")
                },
                "continue" => {
                    sender.input(UnavailableDialogMsg::Continue);
                    debug!("Response: continue")
                },
                _ => unreachable!(),
            }
        });
        ComponentParts { model, widgets }

    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            UnavailableDialogMsg::Show(useritems, sysitems, updatetype) => {
                self.updatetype = updatetype;
                let mut unavailableuseritems_guard = self.unavailableuseritems.guard();
                unavailableuseritems_guard.clear();
                for item in useritems {
                    unavailableuseritems_guard.push_back(item);
                }
                let mut unavailablesysitems_guard = self.unavailablesysitems.guard();
                for item in sysitems {
                    unavailablesysitems_guard.push_back(item);
                }
                self.hidden = false;
            }
            UnavailableDialogMsg::Close => {
                info!("UpdateDialogMsg::Close");
                let mut unavailableuseritems_guard = self.unavailableuseritems.guard();
                let mut unavailablesysitems_guard = self.unavailablesysitems.guard();
                unavailableuseritems_guard.clear();
                unavailablesysitems_guard.clear();
                self.hidden = true;
            }
            UnavailableDialogMsg::Continue => {
                match self.updatetype {
                    UpdateType::User => {
                        sender.output(UpdatePageMsg::UpdateAllUserRm(self.unavailableuseritems.iter().map(|x| x.pkg.to_string()).collect()));
                    }
                    UpdateType::System => {
                        sender.output(UpdatePageMsg::UpdateSystemRm(self.unavailablesysitems.iter().map(|x| x.pkg.to_string()).collect()));
                    }
                    UpdateType::All => {
                        sender.output(UpdatePageMsg::UpdateAllRm(self.unavailableuseritems.iter().map(|x| x.pkg.to_string()).collect(), self.unavailablesysitems.iter().map(|x| x.pkg.to_string()).collect()));
                    }
                }
                sender.input(UnavailableDialogMsg::Close)
            }
        }
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct UnavailableItemModel {
    pub name: String,
    pub pkg: String,
    pub pname: String,
    pub icon: Option<String>,
    pub message: String,
}

#[derive(Debug)]
pub enum UnavailableItemMsg {}

#[relm4::factory(pub)]
impl FactoryComponent for UnavailableItemModel {
    type CommandOutput = ();
    type Init = UnavailableItemModel;
    type Input = ();
    type Output = UnavailableItemMsg;
    type ParentWidget = adw::gtk::ListBox;
    type ParentInput = UnavailableDialogMsg;

    view! {
        adw::PreferencesRow {
            set_activatable: false,
            #[wrap(Some)]
            set_child = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_hexpand: true,
                set_spacing: 10,
                set_margin_all: 10,
                adw::Bin {
                    set_valign: gtk::Align::Center,
                    #[wrap(Some)]
                    set_child = if self.icon.is_some() {
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
                    }
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,
                    set_spacing: 20,
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_halign: gtk::Align::Fill,
                        set_valign: gtk::Align::Center,
                        set_spacing: 2,
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_label: self.name.as_str(),
                            set_ellipsize: pango::EllipsizeMode::End,
                            set_lines: 1,
                            set_wrap: true,
                            set_max_width_chars: 0,
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            add_css_class: "dim-label",
                            add_css_class: "caption",
                            set_label: self.pkg.as_str(),
                            set_ellipsize: pango::EllipsizeMode::End,
                            set_lines: 1,
                            set_wrap: true,
                            set_max_width_chars: 0,
                        },
                    },
                    gtk::Label {
                        set_halign: gtk::Align::Center,
                        set_hexpand: true,
                        set_label: self.message.as_str(),
                        set_wrap: true,
                    }
                }
                
            }
        }
    }

    fn init_model(
        init: Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        init
    }
}
