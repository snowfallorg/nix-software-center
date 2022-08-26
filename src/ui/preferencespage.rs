use std::path::PathBuf;

use super::window::AppMsg;
use adw::prelude::*;
use relm4::*;
use relm4_components::open_dialog::*;

#[derive(Debug)]
pub struct PreferencesPageModel {
    hidden: bool,
    configpath: PathBuf,
    flake: Option<(PathBuf, String)>,
    open_dialog: Controller<OpenDialog>,
    flake_file_dialog: Controller<OpenDialog>,
}

#[derive(Debug)]
pub enum PreferencesPageMsg {
    Show(PathBuf, Option<(PathBuf, String)>),
    Open,
    OpenFlake,
    SetConfigPath(PathBuf),
    SetFlake(Option<(PathBuf, String)>),
    SetFlakePath(PathBuf),
    SetFlakeArg(String),
    ModifyFlake,
    Ignore,
}

#[relm4::component(pub)]
impl SimpleComponent for PreferencesPageModel {
    type InitParams = gtk::Window;
    type Input = PreferencesPageMsg;
    type Output = AppMsg;
    type Widgets = PreferencesPageWidgets;

    view! {
        adw::PreferencesWindow {
            #[watch]
            set_visible: !model.hidden,
            set_transient_for: Some(&parent_window),
            set_modal: true,
            set_search_enabled: false,
            add = &adw::PreferencesPage {
                add = &adw::PreferencesGroup {
                    // set_title: "Preferences",
                    add = &adw::ActionRow {
                        set_title: "Configuration file",
                        add_suffix = &gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_halign: gtk::Align::End,
                            set_valign: gtk::Align::Center,
                            set_spacing: 10,
                            gtk::Button {
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 5,
                                    gtk::Image {
                                        set_icon_name: Some("document-open-symbolic"),
                                    },
                                    gtk::Label {
                                        #[watch]
                                        set_label: {
                                            let x = model.configpath.file_name().unwrap_or_default().to_str().unwrap_or_default();
                                            if x.is_empty() {
                                                "(None)"
                                            } else {
                                                x
                                            }
                                        }
                                    }
                                },
                                connect_clicked[sender] => move |_| {
                                    sender.input(PreferencesPageMsg::Open);
                                }
                            },
                            gtk::Button {
                                add_css_class: "flat",
                                set_icon_name: "view-refresh-symbolic",
                                connect_clicked[sender] => move |_| {
                                    sender.input(PreferencesPageMsg::SetConfigPath(PathBuf::from("/etc/nixos/configuration.nix")));
                                }
                            }
                        }
                    },
                    add = &adw::ActionRow {
                        set_title: "Use nix flakes",
                        add_suffix = &gtk::Switch {
                            set_valign: gtk::Align::Center,
                            connect_state_set[sender] => move |_, b| {
                                if b {
                                    sender.input(PreferencesPageMsg::SetFlake(Some((PathBuf::new(), String::default()))));
                                } else {
                                    sender.input(PreferencesPageMsg::SetFlake(None));
                                }
                                gtk::Inhibit(false)
                            }
                        }
                    },
                    add = &adw::ActionRow {
                        set_title: "Flake file",
                        #[watch]
                        set_visible: model.flake.is_some(),
                        add_suffix = &gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_halign: gtk::Align::End,
                            set_valign: gtk::Align::Center,
                            set_spacing: 10,
                            gtk::Button {
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 5,
                                    gtk::Image {
                                        set_icon_name: Some("document-open-symbolic"),
                                    },
                                    gtk::Label {
                                        #[watch]
                                        set_label: {
                                            let x = if let Some((f, _)) = &model.flake {
                                                f.file_name().unwrap_or_default().to_str().unwrap_or_default()
                                            } else {
                                                ""
                                            };
                                            if x.is_empty() {
                                                "(None)"
                                            } else {
                                                x
                                            }
                                        }
                                    }
                                },
                                connect_clicked[sender] => move |_| {
                                    sender.input(PreferencesPageMsg::OpenFlake);
                                }
                            },
                            gtk::Button {
                                add_css_class: "flat",
                                set_icon_name: "user-trash-symbolic",
                                connect_clicked[sender] => move |_| {
                                    sender.input(PreferencesPageMsg::SetFlakePath(PathBuf::new()));
                                }
                            }
                        }
                    },
                    add = &adw::EntryRow {
                        #[watch]
                        set_visible: model.flake.is_some(),
                        set_title: "Flake arguments (--flake path/to/flake.nix#<THIS ENTRY>)",
                        connect_changed[sender] => move |x| {
                            sender.input(PreferencesPageMsg::SetFlakeArg(x.text().to_string()));
                        }
                    }

                }
            }
        }
    }

    fn init(
        parent_window: Self::InitParams,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let open_dialog = OpenDialog::builder()
            .transient_for_native(root)
            .launch(OpenDialogSettings::default())
            .forward(&sender.input, |response| match response {
                OpenDialogResponse::Accept(path) => PreferencesPageMsg::SetConfigPath(path),
                OpenDialogResponse::Cancel => PreferencesPageMsg::Ignore,
            });
        let flake_file_dialog = OpenDialog::builder()
            .transient_for_native(root)
            .launch(OpenDialogSettings::default())
            .forward(&sender.input, |response| match response {
                OpenDialogResponse::Accept(path) => PreferencesPageMsg::SetFlakePath(path),
                OpenDialogResponse::Cancel => PreferencesPageMsg::Ignore,
            });
        let model = PreferencesPageModel {
            hidden: true,
            configpath: PathBuf::new(),
            flake: None,
            open_dialog,
            flake_file_dialog,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PreferencesPageMsg::Show(path, flake) => {
                self.configpath = path;
                self.flake = flake;
                self.hidden = false;
                println!("FLAKE {:?}", self.flake);
            }
            PreferencesPageMsg::Open => self.open_dialog.emit(OpenDialogMsg::Open),
            PreferencesPageMsg::OpenFlake => self.flake_file_dialog.emit(OpenDialogMsg::Open),
            PreferencesPageMsg::SetConfigPath(path) => {
                self.configpath = path.clone();
                sender.output(AppMsg::UpdateSysconfig(path.to_string_lossy().to_string()));
            }
            PreferencesPageMsg::SetFlake(flake) => {
                self.flake = flake;
                println!("FLAKE {:?}", self.flake);
                sender.input(PreferencesPageMsg::ModifyFlake)
            }
            PreferencesPageMsg::SetFlakePath(path) => {
                self.flake = Some((
                    path,
                    self.flake.as_ref().map(|x| x.1.clone()).unwrap_or_default(),
                ));
                println!("FLAKE {:?}", self.flake);
                sender.input(PreferencesPageMsg::ModifyFlake)
            }
            PreferencesPageMsg::SetFlakeArg(arg) => {
                self.flake = Some((
                    self.flake.as_ref().map(|x| x.0.clone()).unwrap_or_default(),
                    arg,
                ));
                println!("FLAKE {:?}", self.flake);
                sender.input(PreferencesPageMsg::ModifyFlake)
            }
            PreferencesPageMsg::ModifyFlake => {
                let out = self
                    .flake
                    .as_ref()
                    .map(|(path, arg)| format!("{}#{}", path.to_string_lossy(), arg));
                sender.output(AppMsg::UpdateFlake(out.map(|x| x.strip_suffix('#').unwrap_or(&x).to_string())));
            }
            _ => {}
        }
    }
}
