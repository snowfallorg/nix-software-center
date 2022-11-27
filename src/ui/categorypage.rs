use super::{categories::PkgCategory, categorytile::CategoryTile, window::*};
use adw::prelude::*;
use log::*;
use relm4::{factory::*, *};

#[tracker::track]
#[derive(Debug)]
pub struct CategoryPageModel {
    category: PkgCategory,
    #[tracker::no_eq]
    recommendedapps: FactoryVecDeque<CategoryTile>,
    #[tracker::no_eq]
    apps: FactoryVecDeque<CategoryTile>,
    busy: bool,
}

#[derive(Debug)]
pub enum CategoryPageMsg {
    Close,
    OpenPkg(String),
    Open(PkgCategory, Vec<CategoryTile>, Vec<CategoryTile>),
    Loading(PkgCategory),
    UpdateInstalled(Vec<String>, Vec<String>),
}

#[derive(Debug)]
pub enum CategoryPageAsyncMsg {
    PushRec(CategoryTile),
    Push(CategoryTile),
}

#[relm4::component(pub)]
impl Component for CategoryPageModel {
    type Init = ();
    type Input = CategoryPageMsg;
    type Output = AppMsg;
    type CommandOutput = CategoryPageAsyncMsg;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            adw::HeaderBar {
                pack_start = &gtk::Button {
                    add_css_class: "flat",
                    gtk::Image {
                        set_icon_name: Some("go-previous-symbolic"),
                    },
                    connect_clicked[sender] => move |_| {
                        sender.input(CategoryPageMsg::Close)
                    },
                },
                #[wrap(Some)]
                set_title_widget = &gtk::Label {
                    #[watch]
                    set_label: match model.category {
                        PkgCategory::Audio => "Audio",
                        PkgCategory::Development => "Development",
                        PkgCategory::Games => "Games",
                        PkgCategory::Graphics => "Graphics",
                        PkgCategory::Web => "Web",
                        PkgCategory::Video => "Video",
                    },
                },
            },
            gtk::ScrolledWindow {
                set_vexpand: true,
                set_hexpand: true,
                set_hscrollbar_policy: gtk::PolicyType::Never,
                set_vscrollbar_policy: gtk::PolicyType::Automatic,
                #[track(model.changed(CategoryPageModel::category()))]
                set_vadjustment: gtk::Adjustment::NONE,
                adw::Clamp {
                    set_maximum_size: 1000,
                    set_tightening_threshold: 750,
                    if model.busy {
                        #[name(spinner)]
                        gtk::Spinner {
                            set_hexpand: true,
                            set_vexpand: true,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_spinning: true,
                            set_size_request: (64, 64),
                        }
                    } else {
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_valign: gtk::Align::Start,
                            set_margin_all: 15,
                            set_spacing: 15,
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: "title-4",
                                set_label: "Recommended",
                            },
                            #[local_ref]
                            recbox -> gtk::FlowBox {
                                set_halign: gtk::Align::Fill,
                                set_hexpand: true,
                                set_valign: gtk::Align::Center,
                                set_orientation: gtk::Orientation::Horizontal,
                                set_selection_mode: gtk::SelectionMode::None,
                                set_homogeneous: true,
                                set_max_children_per_line: 3,
                                set_min_children_per_line: 1,
                                set_column_spacing: 14,
                                set_row_spacing: 14,
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: "title-4",
                                set_label: "Other",
                            },
                            #[local_ref]
                            allbox -> gtk::FlowBox {
                                set_halign: gtk::Align::Fill,
                                set_hexpand: true,
                                set_valign: gtk::Align::Center,
                                set_orientation: gtk::Orientation::Horizontal,
                                set_selection_mode: gtk::SelectionMode::None,
                                set_homogeneous: true,
                                set_max_children_per_line: 3,
                                set_min_children_per_line: 1,
                                set_column_spacing: 14,
                                set_row_spacing: 14,
                            }
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
        let model = CategoryPageModel {
            category: PkgCategory::Audio,
            recommendedapps: FactoryVecDeque::new(gtk::FlowBox::new(), sender.input_sender()),
            apps: FactoryVecDeque::new(gtk::FlowBox::new(), sender.input_sender()),
            busy: true,
            tracker: 0,
        };

        let recbox = model.recommendedapps.widget();
        let allbox = model.apps.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.reset();
        match msg {
            CategoryPageMsg::Close => {
                sender.output(AppMsg::FrontFrontPage);
            },
            CategoryPageMsg::OpenPkg(pkg) => {
                sender.output(AppMsg::OpenPkg(pkg));
            },
            CategoryPageMsg::Open(category, catrec, catall) => {
                info!("CategoryPageMsg::Open");
                self.set_category(category);
                let mut recapps_guard = self.recommendedapps.guard();
                recapps_guard.clear();
                recapps_guard.drop();
                let mut apps_guard = self.apps.guard();
                apps_guard.clear();
                apps_guard.drop();

                sender.command(|out, shutdown| {
                    shutdown
                        .register(async move {
                            for app in catrec {
                                out.send(CategoryPageAsyncMsg::PushRec(app));
                                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
                            }
                        })
                        .drop_on_shutdown()
                });

                sender.command(|out, shutdown| {
                    shutdown
                        .register(async move {
                            for app in catall {
                                out.send(CategoryPageAsyncMsg::Push(app));
                                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
                            }
                        })
                        .drop_on_shutdown()
                });

                self.busy = false;
                info!("DONE CategoryPageMsg::Open");
            }
            CategoryPageMsg::Loading(category) => {
                info!("CategoryPageMsg::Loading");
                self.set_category(category);
                self.busy = true;
            }
            CategoryPageMsg::UpdateInstalled(installeduserpkgs, installedsystempkgs) => {
                let mut recapps_guard = self.recommendedapps.guard();
                for i in 0..recapps_guard.len() {
                    let app = recapps_guard.get_mut(i).unwrap();
                    if installeduserpkgs.contains(&app.pname) {
                        app.installeduser = true;
                    } else {
                        app.installeduser = false;
                    }
                    if installedsystempkgs.contains(&app.pkg) {
                        app.installedsystem = true;
                    } else {
                        app.installedsystem = false;
                    }
                }
                let mut apps_guard = self.apps.guard();
                for i in 0..apps_guard.len() {
                    let app = apps_guard.get_mut(i).unwrap();
                    if installeduserpkgs.contains(&app.pname) {
                        app.installeduser = true;
                    } else {
                        app.installeduser = false;
                    }
                    if installedsystempkgs.contains(&app.pkg) {
                        app.installedsystem = true;
                    } else {
                        app.installedsystem = false;
                    }
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            CategoryPageAsyncMsg::PushRec(tile) => {
                let mut recapps_guard = self.recommendedapps.guard();
                recapps_guard.push_back(tile);
                recapps_guard.drop();
            }
            CategoryPageAsyncMsg::Push(tile) => {
                let mut apps_guard = self.apps.guard();
                apps_guard.push_back(tile);
                apps_guard.drop();
            }
        }
    }
}
