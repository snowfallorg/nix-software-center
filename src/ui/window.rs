use crate::{
    config,
    parse::{
        config::{editconfig, getconfig},
        packages::{AppData, LicenseEnum, PkgMaintainer, Platform},
        util,
    },
    ui::{
        installedpage::InstalledItem, pkgpage::PkgPageInit, rebuild::RebuildMsg,
        unavailabledialog::UnavailableDialogMsg, updatepage::UNAVAILABLE_BROKER,
        welcome::WelcomeMsg,
    },
    APPINFO,
};
use adw::prelude::*;
use log::*;
use nix_data::config::configfile::NixDataConfig;
use relm4::{
    self,
    actions::{RelmAction, RelmActionGroup},
    factory::FactoryVecDeque,
    Component, ComponentController, ComponentParts, ComponentSender, Controller, MessageBroker,
    RelmWidgetExt, WorkerController,
};
use spdx::Expression;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use std::{
    collections::{HashMap, HashSet},
    convert::identity,
    fs,
    path::Path,
};

use super::{
    about::{AboutPageModel, AboutPageMsg},
    categories::{PkgCategory, PkgGroup},
    categorypage::{CategoryPageModel, CategoryPageMsg},
    categorytile::CategoryTile,
    installedpage::{InstalledPageModel, InstalledPageMsg},
    pkgpage::{self, InstallType, PkgInitModel, PkgModel, PkgMsg, WorkPkg},
    pkgtile::PkgTile,
    preferencespage::{PreferencesPageModel, PreferencesPageMsg},
    rebuild::RebuildModel,
    searchpage::{SearchItem, SearchPageModel, SearchPageMsg},
    unavailabledialog::UnavailableItemModel,
    updatepage::{UpdateItem, UpdatePageInit, UpdatePageModel, UpdatePageMsg, UpdateType},
    welcome::WelcomeModel,
    windowloading::{LoadErrorModel, LoadErrorMsg, WindowAsyncHandler, WindowAsyncHandlerMsg},
};

pub static REBUILD_BROKER: MessageBroker<RebuildMsg> = MessageBroker::new();

#[derive(PartialEq)]
enum Page {
    FrontPage,
    PkgPage,
}

#[derive(PartialEq)]
enum MainPage {
    FrontPage,
    CategoryPage,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SystemPkgs {
    Legacy,
    Flake,
    None,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum UserPkgs {
    Env,
    Profile,
}

#[tracker::track]
pub struct AppModel {
    mainwindow: adw::ApplicationWindow,
    config: NixDataConfig,
    #[tracker::no_eq]
    windowloading: WorkerController<WindowAsyncHandler>,
    #[tracker::no_eq]
    loaderrordialog: Controller<LoadErrorModel>,
    busy: bool,
    page: Page,
    mainpage: MainPage,
    // #[tracker::no_eq]
    // pkgs: HashMap<String, Package>,
    // syspkgs: HashMap<String, String>,
    // profilepkgs: Option<HashMap<String, String>>,
    // pkgitems: HashMap<String, PkgItem>,
    #[tracker::no_eq]
    pkgdb: String,
    #[tracker::no_eq]
    nixpkgsdb: Option<String>,
    #[tracker::no_eq]
    systemdb: Option<String>,
    appdata: HashMap<String, AppData>,
    installeduserpkgs: HashMap<String, String>,
    installedsystempkgs: HashSet<String>,
    syspkgtype: SystemPkgs,
    userpkgtype: UserPkgs,
    categoryrec: HashMap<PkgCategory, Vec<String>>,
    categoryall: HashMap<PkgCategory, Vec<String>>,
    #[tracker::no_eq]
    recommendedapps: FactoryVecDeque<PkgTile>,
    #[tracker::no_eq]
    categories: FactoryVecDeque<PkgGroup>,
    #[tracker::no_eq]
    pkgpage: Controller<PkgModel>,
    #[tracker::no_eq]
    searchpage: Controller<SearchPageModel>,
    #[tracker::no_eq]
    categorypage: Controller<CategoryPageModel>,
    searching: bool,
    searchquery: String,
    vschild: String,
    showvsbar: bool,
    #[tracker::no_eq]
    aboutpage: Controller<AboutPageModel>,
    #[tracker::no_eq]
    preferencespage: Controller<PreferencesPageModel>,
    #[tracker::no_eq]
    installedpage: Controller<InstalledPageModel>,
    #[tracker::no_eq]
    updatepage: Controller<UpdatePageModel>,
    viewstack: adw::ViewStack,
    installedpagebusy: Vec<(String, InstallType)>,
    #[tracker::no_eq]
    rebuild: Controller<RebuildModel>,
    #[tracker::no_eq]
    welcomepage: Controller<WelcomeModel>,
    online: bool,
}

#[derive(Debug)]
pub enum AppMsg {
    UpdateSysconfig(Option<String>),
    UpdateFlake(Option<String>, Option<String>),
    TryLoad,
    UpdateDB,
    LoadConfig(NixDataConfig),
    Close,
    LoadError(String, String),
    Initialize(
        String,
        Option<String>,
        Option<String>,
        HashMap<String, AppData>,
        Vec<String>,
        HashMap<PkgCategory, Vec<String>>,
        HashMap<PkgCategory, Vec<String>>,
    ),
    OpenPkg(String),
    FrontPage,
    FrontFrontPage,
    // UpdatePkgs(Option<Vec<String>>),
    UpdateInstalledPkgs,
    UpdateInstalledPage,
    // UpdateUpdatePkgs,
    UpdateCategoryPkgs,
    SetSearch(bool),
    SetVsBar(bool),
    SetVsChild(String),
    Search(String),
    AddInstalledToWorkQueue(WorkPkg),
    RemoveInstalledBusy(WorkPkg),
    OpenCategoryPage(PkgCategory),
    LoadCategory(PkgCategory),
    UpdateRecPkgs(Vec<String>),
    SetDarkMode(bool),
    GetUnavailableItems(HashMap<String, String>, HashMap<String, String>, UpdateType),
    CheckNetwork,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkgItem {
    pkg: String,
    pname: String,
    name: String,
    version: String,
    summary: Option<String>,
    icon: Option<String>,
}

#[derive(Debug)]
pub enum AppAsyncMsg {
    Search(String, Vec<SearchItem>),
    UpdateRecPkgs(Vec<PkgTile>),
    UpdateInstalledPkgs(HashSet<String>, HashMap<String, String>),
    LoadCategory(PkgCategory, Vec<CategoryTile>, Vec<CategoryTile>),
    SetNetwork(bool),
}

#[relm4::component(pub)]
impl Component for AppModel {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = AppAsyncMsg;

    view! {
        #[name(main_window)]
        adw::ApplicationWindow {
            set_default_width: 1150,
            set_default_height: 800,
            #[name(main_stack)]
            if model.busy {
                gtk::Box {
                    set_vexpand: true,
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Fill,
                    set_orientation: gtk::Orientation::Vertical,
                    adw::HeaderBar {
                        add_css_class: "flat",
                        #[wrap(Some)]
                        set_title_widget = &gtk::Label {
                            set_label: "Nix Software Center"
                        }
                    },
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_hexpand: true,
                        set_vexpand: true,
                        set_spacing: 15,
                        gtk::Image {
                            set_icon_name: Some(config::APP_ID),
                            set_pixel_size: 192,
                        },
                        gtk::Label {
                            add_css_class: "title-1",
                            set_label: "Loading...",
                        },
                    }
                }
            } else {
                #[name(main_leaf)]
                adw::Leaflet {
                    set_can_unfold: false,
                    set_homogeneous: false,
                    set_transition_type: adw::LeafletTransitionType::Over,
                    set_can_navigate_back: true,
                    #[name(front_leaf)]
                    append = &adw::Leaflet {
                        set_can_unfold: false,
                        set_homogeneous: false,
                        set_transition_type: adw::LeafletTransitionType::Over,
                        set_can_navigate_back: true,
                        #[name(main_box)]
                        append = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            adw::HeaderBar {
                                set_centering_policy: adw::CenteringPolicy::Strict,
                                pack_start: searchbtn = &gtk::ToggleButton {
                                    add_css_class: "flat",
                                    set_icon_name: "system-search-symbolic",
                                    #[watch]
                                    #[block_signal(searchtoggle)]
                                    set_active: model.searching,
                                    connect_toggled[sender] => move |x| {
                                        sender.input(AppMsg::SetSearch(x.is_active()))
                                    } @searchtoggle

                                },
                                #[name(viewswitchertitle)]
                                #[wrap(Some)]
                                set_title_widget = &adw::ViewSwitcherTitle {
                                    set_title: "Nix Software Center",
                                    set_stack: Some(viewstack),
                                    connect_title_visible_notify[sender] => move |x| {
                                        sender.input(AppMsg::SetVsBar(x.is_title_visible()))
                                    },
                                },
                                pack_end: menu = &gtk::MenuButton {
                                    add_css_class: "flat",
                                    set_icon_name: "open-menu-symbolic",
                                    #[wrap(Some)]
                                    set_popover = &gtk::PopoverMenu::from_model(Some(&mainmenu)) {
                                        add_css_class: "menu"
                                    }
                                }
                            },
                            gtk::SearchBar {
                                #[watch]
                                set_search_mode: model.searching,
                                #[wrap(Some)]
                                set_child = &adw::Clamp {
                                    set_hexpand: true,
                                    gtk::SearchEntry {
                                        #[track(model.changed(AppModel::searching()) && model.searching)]
                                        grab_focus: (),
                                        #[track(model.changed(AppModel::searching()) && !model.searching)]
                                        set_text: "",
                                        connect_search_changed[sender] => move |x| {
                                            if x.text().len() > 1 {
                                                sender.input(AppMsg::Search(x.text().to_string()))
                                            }
                                        }
                                    }
                                }
                            },
                            #[local_ref]
                            viewstack -> adw::ViewStack {
                                connect_visible_child_notify[sender] => move |x| {
                                    if let Some(c) = x.visible_child_name() {
                                        sender.input(AppMsg::SetVsChild(c.to_string()))
                                    }
                                },
                                #[name(frontpage)]
                                add = &gtk::ScrolledWindow {
                                    set_vexpand: true,
                                    set_hexpand: true,
                                    set_hscrollbar_policy: gtk::PolicyType::Never,
                                    adw::Clamp {
                                        set_maximum_size: 1000,
                                        set_tightening_threshold: 750,
                                        gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            set_valign: gtk::Align::Start,
                                            set_margin_all: 15,
                                            set_spacing: 15,
                                            gtk::Label {
                                                set_halign: gtk::Align::Start,
                                                add_css_class: "title-4",
                                                set_label: "Categories",
                                            },
                                            #[local_ref]
                                            categorybox -> gtk::FlowBox {
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
                                            }
                                        }
                                    }
                                },
                                add: model.installedpage.widget(),
                                add: model.searchpage.widget(),
                                add: model.updatepage.widget(),
                            },
                            adw::ViewSwitcherBar {
                                set_stack: Some(viewstack),
                                #[track(model.changed(AppModel::showvsbar()))]
                                set_reveal: model.showvsbar,
                            }
                        },
                        append: model.categorypage.widget(),
                    },
                    append: model.pkgpage.widget()
                }
            }
        }
    }

    menu! {
        mainmenu: {
            "Preferences" => PreferencesAction,
            "About" => AboutAction,
        }
    }

    fn pre_view() {
        match model.page {
            Page::FrontPage => {
                main_leaf.set_visible_child(front_leaf);
            }
            Page::PkgPage => {
                main_leaf.set_visible_child(model.pkgpage.widget());
            }
        }
        match model.mainpage {
            MainPage::FrontPage => {
                front_leaf.set_visible_child(main_box);
            }
            MainPage::CategoryPage => {
                front_leaf.set_visible_child(model.categorypage.widget());
            }
        }
    }

    #[tokio::main]
    async fn init(
        _application: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let (config, welcome) = if let Some(config) = getconfig() {
            debug!("Got config: {:?}", config);
            let mut out = false;
            if let Some(configpath) = &config.systemconfig {
                if !Path::new(configpath).exists() {
                    warn!("Invalid system config path: {}", configpath);
                    out = true
                }
            }
            if let Some(flakepath) = &config.flake {
                if !Path::new(&flakepath).exists() {
                    warn!("Invalid flake path: {}", flakepath);
                    out = true
                }
            }
            (config, out)
        } else {
            // Show welcome page
            debug!("No config found");
            (
                NixDataConfig {
                    systemconfig: None,
                    flake: None,
                    flakearg: None,
                    generations: None,
                },
                true,
            )
        };

        let userpkgtype = if let Ok(h) = std::env::var("HOME") {
            if Path::new(&format!("{}/.nix-profile/manifest.json", h)).exists()
                || !Path::new("/nix/var/nix/profiles/per-user/root/channels/nixos").exists()
                || !Path::new(&format!("{}/.nix-profile/manifest.nix", h)).exists()
                || if let Ok(m) = fs::read_to_string(&format!("{}/.nix-profile/manifest.nix", h)) {
                    m == "[ ]"
                } else {
                    false
                }
            {
                UserPkgs::Profile
            } else {
                UserPkgs::Env
            }
        } else {
            UserPkgs::Env
        };
        let nixos = Path::new("/etc/NIXOS").exists();
        let syspkgtype = if config.systemconfig.is_none() || !nixos {
            SystemPkgs::None
        } else {
            match fs::read_to_string("/run/current-system/nixos-version") {
                Ok(s) => {
                    if !Path::new("/nix/var/nix/profiles/per-user/root/channels/nixos").exists()
                        || config.flake.is_some()
                    {
                        SystemPkgs::Flake
                    } else if let Some(last) = s.split('.').last() {
                        if last.len() == 7 || last == "dirty" || last == "git" {
                            SystemPkgs::Flake
                        } else {
                            SystemPkgs::Legacy
                        }
                    } else {
                        SystemPkgs::Legacy
                    }
                }
                Err(_) => SystemPkgs::None,
            }
        };

        debug!("userpkgtype: {:?}", userpkgtype);
        debug!("syspkgtype: {:?}", syspkgtype);

        let online = util::checkonline();

        let windowloading = WindowAsyncHandler::builder()
            .detach_worker(())
            .forward(sender.input_sender(), identity);
        let loaderrordialog = LoadErrorModel::builder()
            .launch(root.clone().upcast())
            .forward(sender.input_sender(), identity);
        let pkgpage = PkgModel::builder()
            .launch(PkgPageInit {
                userpkgs: userpkgtype.clone(),
                syspkgs: syspkgtype.clone(),
                config: config.clone(),
                online,
            })
            .forward(sender.input_sender(), identity);
        let searchpage = SearchPageModel::builder()
            .launch(())
            .forward(sender.input_sender(), identity);
        let categorypage = CategoryPageModel::builder()
            .launch(())
            .forward(sender.input_sender(), identity);
        let installedpage = InstalledPageModel::builder()
            .launch((syspkgtype.clone(), userpkgtype.clone()))
            .forward(sender.input_sender(), identity);
        let updatepage = UpdatePageModel::builder()
            .launch(UpdatePageInit {
                window: root.clone().upcast(),
                systype: syspkgtype.clone(),
                usertype: userpkgtype.clone(),
                config: config.clone(),
                online,
            })
            .forward(sender.input_sender(), identity);
        let rebuild = RebuildModel::builder()
            .launch_with_broker(root.clone().upcast(), &REBUILD_BROKER)
            .forward(sender.input_sender(), identity);
        let viewstack = adw::ViewStack::new();
        let welcomepage = WelcomeModel::builder()
            .launch(root.clone().upcast())
            .forward(sender.input_sender(), identity);
        let aboutpage = AboutPageModel::builder()
            .launch(root.clone().upcast())
            .detach();
        let preferencespage = PreferencesPageModel::builder()
            .launch(root.clone().upcast())
            .forward(sender.input_sender(), identity);

        let model = AppModel {
            mainwindow: root.clone(),
            config,
            windowloading,
            loaderrordialog,
            busy: true,
            page: Page::FrontPage,
            mainpage: MainPage::FrontPage,
            pkgdb: String::new(),
            nixpkgsdb: None,
            systemdb: None,
            appdata: HashMap::new(),
            installeduserpkgs: HashMap::new(),
            installedsystempkgs: HashSet::new(),
            syspkgtype,
            userpkgtype,
            categoryrec: HashMap::new(),
            categoryall: HashMap::new(),
            recommendedapps: FactoryVecDeque::new(gtk::FlowBox::new(), sender.input_sender()),
            categories: FactoryVecDeque::new(gtk::FlowBox::new(), sender.input_sender()),
            pkgpage,
            searchpage,
            categorypage,
            searching: false,
            searchquery: String::default(),
            vschild: String::default(),
            showvsbar: false,
            installedpage,
            updatepage,
            viewstack,
            installedpagebusy: vec![],
            rebuild,
            welcomepage,
            aboutpage,
            preferencespage,
            online,
            tracker: 0,
        };

        {
            let sender = sender.clone();
            adw::StyleManager::default()
                .connect_dark_notify(move |x| sender.input(AppMsg::SetDarkMode(x.is_dark())));
        }

        sender.input(AppMsg::SetDarkMode(adw::StyleManager::default().is_dark()));

        if welcome && nixos {
            model.welcomepage.emit(WelcomeMsg::Show);
        } else {
            model.windowloading.emit(WindowAsyncHandlerMsg::CheckCache(
                model.syspkgtype.clone(),
                model.userpkgtype.clone(),
                model.config.clone(),
            ));
        }
        let recbox = model.recommendedapps.widget();
        let categorybox = model.categories.widget();
        let viewstack = &model.viewstack;

        let widgets = view_output!();

        let mut group = RelmActionGroup::<MenuActionGroup>::new();
        let aboutpage: RelmAction<AboutAction> = {
            let sender = model.aboutpage.sender().clone();
            RelmAction::new_stateless(move |_| {
                sender.send(()).unwrap();
            })
        };

        let prefernecespage: RelmAction<PreferencesAction> = {
            let sender = model.preferencespage.sender().clone();
            let preferencespage = model.preferencespage.widget().clone();
            let config = model.config.clone();
            RelmAction::new_stateless(move |_| {
                sender.send(PreferencesPageMsg::Show(config.clone())).unwrap();
                preferencespage.present();
            })
        };

        group.add_action(aboutpage);
        group.add_action(prefernecespage);
        let actions = group.into_action_group();
        widgets
            .main_window
            .insert_action_group("menu", Some(&actions));

        widgets.main_stack.set_vhomogeneous(false);
        widgets.main_stack.set_hhomogeneous(false);
        let frontvs = widgets.viewstack.page(&widgets.frontpage);
        let installedvs = widgets.viewstack.page(model.installedpage.widget());
        let updatesvs = widgets.viewstack.page(model.updatepage.widget());
        let searchvs = widgets.viewstack.page(model.searchpage.widget());
        frontvs.set_title(Some("Explore"));
        installedvs.set_title(Some("Installed"));
        updatesvs.set_title(Some("Updates"));
        frontvs.set_name(Some("explore"));
        installedvs.set_name(Some("installed"));
        searchvs.set_name(Some("search"));
        updatesvs.set_name(Some("updates"));
        frontvs.set_icon_name(Some("nsc-home-symbolic"));
        installedvs.set_icon_name(Some("nsc-installed-symbolic"));
        updatesvs.set_icon_name(Some("nsc-update-symbolic"));

        ComponentParts { model, widgets }
    }

    #[tokio::main]
    async fn update(
        &mut self,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        self.reset();
        match msg {
            AppMsg::TryLoad => {
                self.busy = true;
                self.windowloading.emit(WindowAsyncHandlerMsg::CheckCache(
                    self.syspkgtype.clone(),
                    self.userpkgtype.clone(),
                    self.config.clone(),
                ));
            }
            AppMsg::UpdateDB => {
                self.windowloading.emit(WindowAsyncHandlerMsg::UpdateDB(
                    self.syspkgtype.clone(),
                    self.userpkgtype.clone(),
                ));
            }
            AppMsg::LoadConfig(config) => {
                info!("AppMsg::LoadConfig");
                self.config = config;
                if let Err(e) = editconfig(self.config.clone()) {
                    warn!("Error editing config: {}", e);
                }
                let nixos = Path::new("/etc/NIXOS").exists();
                self.syspkgtype = if self.config.systemconfig.is_none() || !nixos {
                    SystemPkgs::None
                } else {
                    match fs::read_to_string("/run/current-system/nixos-version") {
                        Ok(s) => {
                            if !Path::new("/nix/var/nix/profiles/per-user/root/channels/nixos")
                                .exists()
                                || self.config.flake.is_some()
                            {
                                SystemPkgs::Flake
                            } else if let Some(last) = s.split('.').last() {
                                if last.len() == 7 || last == "dirty" || last == "git" {
                                    SystemPkgs::Flake
                                } else {
                                    SystemPkgs::Legacy
                                }
                            } else {
                                SystemPkgs::Legacy
                            }
                        }
                        Err(_) => SystemPkgs::None,
                    }
                };
                self.pkgpage.emit(PkgMsg::UpdatePkgTypes(
                    self.syspkgtype.clone(),
                    self.userpkgtype.clone(),
                ));
                self.pkgpage.emit(PkgMsg::UpdateConfig(self.config.clone()));
                self.updatepage.emit(UpdatePageMsg::UpdatePkgTypes(
                    self.syspkgtype.clone(),
                    self.userpkgtype.clone(),
                ));
                self.updatepage
                    .emit(UpdatePageMsg::UpdateConfig(self.config.clone()));
                self.windowloading.emit(WindowAsyncHandlerMsg::CheckCache(
                    self.syspkgtype.clone(),
                    self.userpkgtype.clone(),
                    self.config.clone(),
                ));
            }
            AppMsg::Close => {
                relm4::main_application().quit();
            }
            AppMsg::LoadError(msg, msg2) => {
                self.busy = false;
                self.loaderrordialog.emit(LoadErrorMsg::Show(msg, msg2));
            }
            AppMsg::UpdateSysconfig(systemconfig) => {
                self.config = NixDataConfig {
                    systemconfig: systemconfig.clone(),
                    flake: self.config.flake.clone(),
                    flakearg: self.config.flakearg.clone(),
                    generations: self.config.generations,
                };
                if editconfig(self.config.clone()).is_err() {
                    warn!("Failed to update config");
                }
                let nixos = Path::new("/etc/NIXOS").exists();
                if systemconfig.is_some() && nixos {
                    if self.syspkgtype == SystemPkgs::None {
                        if self.config.flake.is_some() {
                            self.syspkgtype = SystemPkgs::Flake;
                        } else {
                            self.syspkgtype = SystemPkgs::Legacy;
                        }
                    }
                } else {
                    self.syspkgtype = SystemPkgs::None;
                }

                self.pkgpage.emit(PkgMsg::UpdateConfig(self.config.clone()));
                self.updatepage
                    .emit(UpdatePageMsg::UpdateConfig(self.config.clone()));
                self.pkgpage.emit(PkgMsg::UpdatePkgTypes(
                    self.syspkgtype.clone(),
                    self.userpkgtype.clone(),
                ));
                self.updatepage.emit(UpdatePageMsg::UpdatePkgTypes(
                    self.syspkgtype.clone(),
                    self.userpkgtype.clone(),
                ));
                self.installedpage.emit(InstalledPageMsg::UpdatePkgTypes(
                    self.syspkgtype.clone(),
                    self.userpkgtype.clone(),
                ));
            }
            AppMsg::UpdateFlake(flake, flakearg) => {
                self.config = NixDataConfig {
                    systemconfig: self.config.systemconfig.clone(),
                    flake: flake.clone(),
                    flakearg,
                    generations: self.config.generations,
                };
                if editconfig(self.config.clone()).is_err() {
                    warn!("Failed to update config");
                }

                let nixos = Path::new("/etc/NIXOS").exists();
                if nixos {
                    if flake.is_some() {
                        self.syspkgtype = SystemPkgs::Flake;
                    } else {
                        self.syspkgtype = SystemPkgs::Legacy;
                    }
                }

                self.pkgpage.emit(PkgMsg::UpdateConfig(self.config.clone()));
                self.updatepage
                    .emit(UpdatePageMsg::UpdateConfig(self.config.clone()));
                self.pkgpage.emit(PkgMsg::UpdatePkgTypes(
                    self.syspkgtype.clone(),
                    self.userpkgtype.clone(),
                ));
                self.updatepage.emit(UpdatePageMsg::UpdatePkgTypes(
                    self.syspkgtype.clone(),
                    self.userpkgtype.clone(),
                ));
                self.installedpage.emit(InstalledPageMsg::UpdatePkgTypes(
                    self.syspkgtype.clone(),
                    self.userpkgtype.clone(),
                ));
            }
            AppMsg::Initialize(
                pkgdb,
                nixpkgsdb,
                systemdb,
                appdata,
                recommendedapps,
                categoryrec,
                categoryall,
            ) => {
                info!("AppMsg::Initialize");
                self.pkgdb = pkgdb;
                self.nixpkgsdb = nixpkgsdb;
                self.systemdb = systemdb;
                self.appdata = appdata;
                self.categoryrec = categoryrec;
                self.categoryall = categoryall;

                self.pkgpage.emit(PkgMsg::UpdateConfig(self.config.clone()));
                self.updatepage
                    .emit(UpdatePageMsg::UpdateConfig(self.config.clone()));
                sender.input(AppMsg::UpdateRecPkgs(recommendedapps));
                let mut cat_guard = self.categories.guard();
                cat_guard.clear();
                for c in vec![
                    PkgCategory::Audio,
                    PkgCategory::Development,
                    PkgCategory::Games,
                    PkgCategory::Graphics,
                    PkgCategory::Web,
                    PkgCategory::Video,
                ] {
                    cat_guard.push_back(c);
                }
                cat_guard.drop();
                self.busy = false;
            }
            AppMsg::UpdateRecPkgs(pkgs) => {
                info!("AppMsg::UpdateRecPkgs");
                let appdata: HashMap<String, AppData> = self
                    .appdata
                    .iter()
                    .filter_map(|(k, v)| {
                        if pkgs.contains(k) {
                            Some((k.to_string(), v.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();
                let installeduser = self.installeduserpkgs.clone();
                let installedsystem = self.installedsystempkgs.clone();
                let poolref = self.pkgdb.clone();
                let userpkgtype = self.userpkgtype.clone();
                sender.oneshot_command(async move {
                    let mut pkgtiles = vec![];
                    if let Ok(pool) = &SqlitePool::connect(&format!("sqlite://{}", poolref)).await {
                        for pkg in pkgs {
                            if let Some(data) = appdata.get(&pkg) {
                                let pname: (String,) =
                                    sqlx::query_as("SELECT pname FROM pkgs WHERE attribute = $1")
                                        .bind(&pkg)
                                        .fetch_one(pool)
                                        .await
                                        .unwrap();
                                pkgtiles.push(PkgTile {
                                    pkg: pkg.to_string(),
                                    name: if let Some(name) = &data.name {
                                        name.get("C").unwrap_or(&pname.0).to_string()
                                    } else {
                                        pname.0.to_string()
                                    },
                                    pname: pname.0.to_string(),
                                    icon: data
                                        .icon
                                        .as_ref()
                                        .and_then(|x| x.cached.as_ref())
                                        .map(|x| x[0].name.clone()),
                                    summary: data
                                        .summary
                                        .as_ref()
                                        .and_then(|x| x.get("C"))
                                        .map(|x| x.to_string())
                                        .unwrap_or_default(),
                                    installeduser: installeduser.contains_key(&match userpkgtype {
                                        UserPkgs::Env => pname.0,
                                        UserPkgs::Profile => pkg.to_string(),
                                    }),
                                    installedsystem: installedsystem.contains(&pkg),
                                })
                            }
                        }
                    }
                    AppAsyncMsg::UpdateRecPkgs(pkgtiles)
                });
            }
            AppMsg::OpenPkg(pkg) => {
                info!("AppMsg::OpenPkg {}", pkg);
                sender.input(AppMsg::CheckNetwork);
                if let Ok(pool) = &SqlitePool::connect(&format!("sqlite://{}", self.pkgdb)).await {
                    let pkgdata: Result<
                        (
                            String,
                            String,
                            String,
                            String,
                            String,
                            String,
                            String,
                            String,
                            String,
                        ),
                        _,
                    > = sqlx::query_as(
                        r#"
SELECT pname, version, system, description, longdescription, homepage, license, platforms, maintainers
FROM pkgs JOIN meta ON (pkgs.attribute = meta.attribute) WHERE pkgs.attribute = $1
                    "#,
                    )
                    .bind(&pkg)
                    .fetch_one(pool)
                    .await;

                    if let Ok((
                        pname,
                        version,
                        system,
                        description,
                        longdescription,
                        homepage,
                        licensejson,
                        platformsjson,
                        maintainersjson,
                    )) = pkgdata
                    {
                        let mut name = pname.to_string();
                        let mut summary = if description.is_empty() {
                            None
                        } else {
                            Some(description)
                        };
                        let mut description = if longdescription.is_empty() {
                            None
                        } else {
                            Some(longdescription)
                        };
                        let mut icon = None;
                        let mut screenshots = vec![];
                        let mut licenses = vec![];
                        let mut platforms = vec![];
                        let mut maintainers = vec![];
                        let mut launchable = None;

                        if let Some(data) = self.appdata.get(&pkg) {
                            if let Some(n) = &data.name {
                                if let Some(n) = n.get("C") {
                                    name = n.to_string();
                                }
                            }
                            if let Some(s) = &data.summary {
                                if let Some(s) = s.get("C") {
                                    summary = Some(s.to_string());
                                }
                            }
                            if let Some(d) = &data.description {
                                if let Some(d) = d.get("C") {
                                    description = Some(d.to_string());
                                }
                            }
                            if let Some(i) = &data.icon {
                                if let Some(mut i) = i.cached.clone() {
                                    i.sort_by(|x, y| x.height.cmp(&y.height));
                                    if let Some(i) = i.last() {
                                        icon = Some(format!(
                                            "{}/icons/nixos/{}x{}/{}",
                                            APPINFO, i.width, i.height, i.name
                                        ));
                                    }
                                }
                            }
                            if let Some(s) = &data.screenshots {
                                for s in s {
                                    if let Some(u) = &s.sourceimage {
                                        if !screenshots.contains(&u.url) {
                                            if s.default == Some(true) {
                                                screenshots.insert(0, u.url.clone());
                                            } else {
                                                screenshots.push(u.url.clone());
                                            }
                                        } else if s.default == Some(true) {
                                            if let Some(index) =
                                                screenshots.iter().position(|x| *x == u.url)
                                            {
                                                screenshots.remove(index);
                                                screenshots.insert(0, u.url.clone());
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(l) = &data.launchable {
                                if let Some(d) = l.desktopid.get(0) {
                                    launchable = Some(d.to_string());
                                }
                            }
                        }

                        fn addlicense(
                            pkglicense: &LicenseEnum,
                            licenses: &mut Vec<pkgpage::License>,
                        ) {
                            match pkglicense {
                                LicenseEnum::Single(l) => {
                                    if let Some(n) = &l.fullname {
                                        let parsed = if let Some(id) = &l.spdxid {
                                            if let Ok(Some(license)) =
                                                Expression::parse(id).map(|p| {
                                                    p.requirements()
                                                        .map(|er| er.req.license.id())
                                                        .collect::<Vec<_>>()[0]
                                                })
                                            {
                                                Some(license)
                                            } else {
                                                None
                                            }
                                        } else if let Ok(Some(license)) =
                                            Expression::parse(n).map(|p| {
                                                p.requirements()
                                                    .map(|er| er.req.license.id())
                                                    .collect::<Vec<_>>()[0]
                                            })
                                        {
                                            Some(license)
                                        } else {
                                            None
                                        };
                                        licenses.push(pkgpage::License {
                                            free: if let Some(f) = l.free {
                                                Some(f)
                                            } else {
                                                parsed.map(|p| {
                                                    p.is_osi_approved() || p.is_fsf_free_libre()
                                                })
                                            },
                                            fullname: n.to_string(),
                                            spdxid: l.spdxid.clone(),
                                            url: if let Some(u) = &l.url {
                                                Some(u.to_string())
                                            } else {
                                                parsed.map(|p| {
                                                    format!(
                                                        "https://spdx.org/licenses/{}.html",
                                                        p.name
                                                    )
                                                })
                                            },
                                        })
                                    } else if let Some(s) = &l.spdxid {
                                        if let Ok(Some(license)) = Expression::parse(s).map(|p| {
                                            p.requirements()
                                                .map(|er| er.req.license.id())
                                                .collect::<Vec<_>>()[0]
                                        }) {
                                            licenses.push(pkgpage::License {
                                                free: Some(
                                                    license.is_osi_approved()
                                                        || license.is_fsf_free_libre()
                                                        || l.free.unwrap_or(false),
                                                ),
                                                fullname: license.full_name.to_string(),
                                                spdxid: Some(license.name.to_string()),
                                                url: if l.url.is_some() {
                                                    l.url.clone()
                                                } else {
                                                    Some(format!(
                                                        "https://spdx.org/licenses/{}.html",
                                                        license.name
                                                    ))
                                                },
                                            })
                                        }
                                    }
                                }
                                LicenseEnum::List(lst) => {
                                    for l in lst {
                                        addlicense(&LicenseEnum::Single(l.clone()), licenses);
                                    }
                                }
                                LicenseEnum::SingleStr(s) => {
                                    if let Ok(Some(license)) = Expression::parse(s).map(|p| {
                                        p.requirements()
                                            .map(|er| er.req.license.id())
                                            .collect::<Vec<_>>()[0]
                                    }) {
                                        licenses.push(pkgpage::License {
                                            free: Some(
                                                license.is_osi_approved()
                                                    || license.is_fsf_free_libre(),
                                            ),
                                            fullname: license.full_name.to_string(),
                                            spdxid: Some(license.name.to_string()),
                                            url: Some(format!(
                                                "https://spdx.org/licenses/{}.html",
                                                license.name
                                            )),
                                        })
                                    }
                                }
                                LicenseEnum::VecStr(lst) => {
                                    for s in lst {
                                        addlicense(&LicenseEnum::SingleStr(s.clone()), licenses);
                                    }
                                }
                                LicenseEnum::Mixed(v) => {
                                    for l in v {
                                        addlicense(l, licenses);
                                    }
                                }
                            }
                        }

                        if let Ok(pkglicense) = serde_json::from_str::<LicenseEnum>(&licensejson) {
                            addlicense(&pkglicense, &mut licenses);
                        }

                        let platformslst = serde_json::from_str::<Platform>(&platformsjson);
                        if let Ok(p) = platformslst {
                            match p {
                                Platform::Single(p) => {
                                    if !platforms.contains(&p) && p != system {
                                        platforms.push(p);
                                    }
                                }
                                Platform::List(v) => {
                                    for p in v {
                                        if !platforms.contains(&p.to_string()) && p != system {
                                            platforms.push(p.to_string());
                                        }
                                    }
                                }
                                Platform::ListList(vv) => {
                                    for v in vv {
                                        for p in v {
                                            if !platforms.contains(&p.to_string()) && p != system {
                                                platforms.push(p.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        platforms.sort();
                        platforms.insert(0, system);

                        if let Ok(m) = serde_json::from_str::<Vec<PkgMaintainer>>(&maintainersjson)
                        {
                            for m in m {
                                maintainers.push(m);
                            }
                        }

                        let out = PkgInitModel {
                            name,
                            version: if version.is_empty() {
                                None
                            } else {
                                Some(version.to_string())
                            },
                            pname,
                            summary,
                            description,
                            icon,
                            pkg,
                            screenshots,
                            homepage: if homepage.is_empty() {
                                None
                            } else {
                                Some(homepage)
                            },
                            platforms,
                            licenses,
                            maintainers,
                            installeduserpkgs: self.installeduserpkgs.keys().cloned().collect(),
                            installedsystempkgs: self.installedsystempkgs.clone(),
                            launchable,
                        };
                        self.page = Page::PkgPage;
                        if self.viewstack.visible_child_name()
                            != Some(gtk::glib::GString::from("search"))
                        {
                            self.searching = false;
                        }
                        self.busy = false;
                        self.pkgpage.emit(PkgMsg::Open(Box::new(out)));
                    }
                } else {
                    error!("No pkgdb!");
                }
            }
            AppMsg::FrontPage => {
                self.page = Page::FrontPage;
            }
            AppMsg::FrontFrontPage => {
                self.page = Page::FrontPage;
                self.mainpage = MainPage::FrontPage;
            }
            AppMsg::UpdateInstalledPkgs => {
                info!("AppMsg::UpdateInstalledPkgs");
                let systemconfig = self.config.systemconfig.clone();
                let syspkgtype = self.syspkgtype.clone();
                let userpkgtype = self.userpkgtype.clone();
                sender.oneshot_command(async move {
                    let installedsystempkgs = if let Some(config) = &systemconfig {
                        match syspkgtype {
                            SystemPkgs::Flake => {
                                let pkgs = nix_data::cache::flakes::getflakepkgs(&[config]).await;
                                if let Ok(pkgs) = pkgs {
                                    pkgs.keys().cloned().collect::<HashSet<String>>()
                                } else {
                                    HashSet::new()
                                }
                            }
                            SystemPkgs::Legacy => {
                                let pkgs = nix_data::cache::channel::getlegacypkgs(&[config]).await;
                                if let Ok(pkgs) = pkgs {
                                    pkgs.keys().cloned().collect::<HashSet<String>>()
                                } else {
                                    HashSet::new()
                                }
                            }
                            _ => HashSet::new(),
                        }
                    } else {
                        HashSet::new()
                    };

                    let installeduserpkgs = match userpkgtype {
                        UserPkgs::Profile => {
                            let pkgs = nix_data::cache::profile::getprofilepkgs_versioned().await;
                            if let Ok(pkgs) = pkgs {
                                pkgs
                            } else {
                                HashMap::new()
                            }
                        }
                        UserPkgs::Env => {
                            let pkgs = nix_data::cache::channel::getenvpkgs();
                            if let Ok(pkgs) = pkgs {
                                pkgs
                            } else {
                                HashMap::new()
                            }
                        }
                    };
                    AppAsyncMsg::UpdateInstalledPkgs(installedsystempkgs, installeduserpkgs)
                });
            }
            AppMsg::UpdateInstalledPage => {
                info!("AppMsg::UpdateInstalledPage");
                let mut installeduseritems = vec![];
                let mut updateuseritems = vec![];
                // let pool = SqlitePool::connect(&self.pkgdb).await.unwrap();
                debug!("Installed user pkgs: {:?}", self.installeduserpkgs);
                debug!("Installed system pkgs: {:?}", self.installedsystempkgs);
                if let Ok(pool) = &SqlitePool::connect(&format!("sqlite://{}", self.pkgdb)).await {
                    match self.userpkgtype {
                        UserPkgs::Env => {
                            for (installedpname, installedver) in &self.installeduserpkgs {
                                let possibleitems: Vec<(String,)> =
                                    sqlx::query_as("SELECT attribute FROM pkgs WHERE pname = $1")
                                        .bind(installedpname)
                                        .fetch_all(pool)
                                        .await
                                        .unwrap();
                                match possibleitems.len() {
                                    1 => {
                                        let (pkg,) = &possibleitems[0];
                                        let (description,): (String,) = sqlx::query_as(
                                            "SELECT description FROM meta WHERE attribute = $1",
                                        )
                                        .bind(pkg)
                                        .fetch_one(pool)
                                        .await
                                        .unwrap();
                                        let (newver,): (String,) = sqlx::query_as(
                                            "SELECT version FROM pkgs WHERE attribute = $1",
                                        )
                                        .bind(pkg)
                                        .fetch_one(pool)
                                        .await
                                        .unwrap();
                                        let mut name = installedpname.to_string();
                                        let mut summary = if description.is_empty() {
                                            None
                                        } else {
                                            Some(description.to_string())
                                        };
                                        let mut icon = None;
                                        if let Some(data) = self.appdata.get(pkg) {
                                            if let Some(n) = &data.name {
                                                if let Some(n) = n.get("C") {
                                                    name = n.to_string();
                                                }
                                            }
                                            if let Some(s) = &data.summary {
                                                if let Some(s) = s.get("C") {
                                                    summary = Some(s.to_string());
                                                }
                                            }
                                            if let Some(i) = &data.icon {
                                                if let Some(i) = &i.cached {
                                                    icon = Some(i[0].name.clone());
                                                }
                                            }
                                        }
                                        installeduseritems.push(InstalledItem {
                                            name: name.clone(),
                                            pname: installedpname.to_string(),
                                            pkg: Some(pkg.clone()),
                                            summary: summary.clone(),
                                            icon: icon.clone(),
                                            pkgtype: InstallType::User,
                                            busy: self.installedpagebusy.contains(&(
                                                installedpname.to_string(),
                                                InstallType::User,
                                            )),
                                        });
                                        if !installedver.eq(&newver) {
                                            updateuseritems.push(UpdateItem {
                                                name,
                                                pname: installedpname.to_string(),
                                                pkg: Some(pkg.clone()),
                                                summary,
                                                icon,
                                                pkgtype: InstallType::User,
                                                verfrom: Some(installedver.to_string()),
                                                verto: Some(newver),
                                            });
                                        }
                                    }
                                    2.. => {
                                        installeduseritems.push(InstalledItem {
                                            name: installedpname.clone(),
                                            pname: installedpname.clone(),
                                            pkg: None,
                                            summary: None,
                                            icon: None,
                                            pkgtype: InstallType::User,
                                            busy: self.installedpagebusy.contains(&(
                                                installedpname.clone(),
                                                InstallType::User,
                                            )),
                                        });
                                        let possibleversions: Vec<(String,)> = sqlx::query_as(
                                            "SELECT version FROM pkgs WHERE pname =  $1",
                                        )
                                        .bind(installedpname)
                                        .fetch_all(pool)
                                        .await
                                        .unwrap();
                                        if !possibleversions.contains(&(installedver.to_string(),))
                                        {
                                            updateuseritems.push(UpdateItem {
                                                name: installedpname.clone(),
                                                pname: installedpname.clone(),
                                                pkg: None,
                                                summary: None,
                                                icon: None,
                                                pkgtype: InstallType::User,
                                                verfrom: Some(installedver.to_string()),
                                                verto: None,
                                            });
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        UserPkgs::Profile => {
                            for installedpkg in self.installeduserpkgs.keys() {
                                debug!("Checking package {}", installedpkg);
                                let (pname, version): (String, String) = sqlx::query_as(
                                    "SELECT pname, version FROM pkgs WHERE attribute = $1",
                                )
                                .bind(installedpkg)
                                .fetch_one(pool)
                                .await
                                .unwrap();
                                let (description,): (String,) = sqlx::query_as(
                                    "SELECT description FROM meta WHERE attribute = $1",
                                )
                                .bind(installedpkg)
                                .fetch_one(pool)
                                .await
                                .unwrap();
                                let mut name = pname.to_string();
                                let mut summary = if description.is_empty() {
                                    None
                                } else {
                                    Some(description)
                                };
                                let mut icon = None;
                                if let Some(data) = self.appdata.get(installedpkg) {
                                    if let Some(n) = &data.name {
                                        if let Some(n) = n.get("C") {
                                            name = n.to_string();
                                        }
                                    }
                                    if let Some(s) = &data.summary {
                                        if let Some(s) = s.get("C") {
                                            summary = Some(s.to_string());
                                        }
                                    }
                                    if let Some(i) = &data.icon {
                                        if let Some(i) = &i.cached {
                                            icon = Some(i[0].name.clone());
                                        }
                                    }
                                }
                                installeduseritems.push(InstalledItem {
                                    name: name.to_string(),
                                    pname: pname.to_string(),
                                    pkg: Some(installedpkg.clone()),
                                    summary: summary.clone(),
                                    icon: icon.clone(),
                                    pkgtype: InstallType::User,
                                    busy: self
                                        .installedpagebusy
                                        .contains(&(installedpkg.clone(), InstallType::User)),
                                });
                                if let Some(latest) = &self.nixpkgsdb {
                                    if let Ok(latestpool) =
                                        &SqlitePool::connect(&format!("sqlite://{}", latest)).await
                                    {
                                        let (newver,): (String,) = sqlx::query_as(
                                            "SELECT version FROM pkgs WHERE attribute = $1",
                                        )
                                        .bind(installedpkg)
                                        .fetch_one(latestpool)
                                        .await
                                        .unwrap();
                                        debug!("PROFILE: {} {} {}", installedpkg, version, newver);
                                        if version != newver {
                                            updateuseritems.push(UpdateItem {
                                                name,
                                                pname,
                                                pkg: Some(installedpkg.clone()),
                                                summary,
                                                icon,
                                                pkgtype: InstallType::System,
                                                verfrom: Some(version.clone()),
                                                verto: Some(newver.clone()),
                                            })
                                        }
                                    }
                                }
                            }
                        }
                    }

                    installeduseritems
                        .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                    let mut installedsystemitems = vec![];
                    let mut updatesystemitems = vec![];
                    for installedpkg in &self.installedsystempkgs {
                        let versionpname: sqlx::Result<(String, String)> =
                            sqlx::query_as("SELECT pname, version FROM pkgs where attribute = $1")
                                .bind(installedpkg)
                                .fetch_one(pool)
                                .await;
                        if let Ok((pname, version)) = versionpname {
                            let desc: sqlx::Result<(String,)> =
                                sqlx::query_as("SELECT description FROM meta WHERE attribute = $1")
                                    .bind(installedpkg)
                                    .fetch_one(pool)
                                    .await;
                            if let Ok((description,)) = desc {
                                let mut name = pname.to_string();
                                let mut summary = if description.is_empty() {
                                    None
                                } else {
                                    Some(description)
                                };
                                let mut icon = None;
                                if let Some(data) = self.appdata.get(installedpkg) {
                                    if let Some(n) = &data.name {
                                        if let Some(n) = n.get("C") {
                                            name = n.to_string();
                                        }
                                    }
                                    if let Some(s) = &data.summary {
                                        if let Some(s) = s.get("C") {
                                            summary = Some(s.to_string());
                                        }
                                    }
                                    if let Some(i) = &data.icon {
                                        if let Some(i) = &i.cached {
                                            icon = Some(i[0].name.clone());
                                        }
                                    }
                                }
                                installedsystemitems.push(InstalledItem {
                                    name: name.to_string(),
                                    pname: pname.to_string(),
                                    pkg: Some(installedpkg.clone()),
                                    summary: summary.clone(),
                                    icon: icon.clone(),
                                    pkgtype: InstallType::System,
                                    busy: self
                                        .installedpagebusy
                                        .contains(&(installedpkg.clone(), InstallType::System)),
                                });
                                if let Some(current) = &self.systemdb {
                                    if let Ok(currentpool) =
                                        &SqlitePool::connect(&format!("sqlite://{}", current)).await
                                    {
                                        let (currver,): (String,) = sqlx::query_as(
                                            "SELECT version FROM pkgs WHERE attribute = $1",
                                        )
                                        .bind(installedpkg)
                                        .fetch_one(currentpool)
                                        .await
                                        .unwrap();
                                        debug!("SYSTEM: {} {} {}", installedpkg, currver, version);
                                        if version != currver {
                                            updatesystemitems.push(UpdateItem {
                                                name,
                                                pname,
                                                pkg: Some(installedpkg.clone()),
                                                summary,
                                                icon,
                                                pkgtype: InstallType::System,
                                                verfrom: Some(currver.clone()),
                                                verto: Some(version.clone()),
                                            })
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Add NixOS system to update list
                    match self.syspkgtype {
                        SystemPkgs::Legacy => {
                            if let Ok(Some((old, new))) = nix_data::cache::channel::uptodate() {
                                updatesystemitems.insert(
                                    0,
                                    UpdateItem {
                                        name: String::from("NixOS System"),
                                        pname: String::new(),
                                        pkg: None,
                                        summary: Some(String::from(
                                            "NixOS internal packages and modules",
                                        )),
                                        icon: None,
                                        pkgtype: InstallType::System,
                                        verfrom: Some(old),
                                        verto: Some(new),
                                    },
                                )
                            }
                        }
                        SystemPkgs::Flake => {
                            if let Ok(Some((old, new))) = nix_data::cache::flakes::uptodate() {
                                updatesystemitems.insert(
                                    0,
                                    UpdateItem {
                                        name: String::from("NixOS System"),
                                        pname: String::new(),
                                        pkg: None,
                                        summary: Some(String::from(
                                            "NixOS internal packages and modules",
                                        )),
                                        icon: None,
                                        pkgtype: InstallType::System,
                                        verfrom: Some(old),
                                        verto: Some(new),
                                    },
                                )
                            }
                        }
                        SystemPkgs::None => {}
                    }

                    installedsystemitems
                        .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                    self.installedpage.emit(InstalledPageMsg::Update(
                        installeduseritems,
                        installedsystemitems,
                    ));
                    self.updatepage
                        .emit(UpdatePageMsg::Update(updateuseritems, updatesystemitems));
                } else {
                    error!("Could not connect to pkgdb");
                }
            }
            AppMsg::UpdateCategoryPkgs => {
                self.categorypage.emit(CategoryPageMsg::UpdateInstalled(
                    self.installeduserpkgs.keys().cloned().collect::<Vec<_>>(),
                    self.installedsystempkgs.iter().cloned().collect::<Vec<_>>(),
                ));
            }
            AppMsg::SetSearch(show) => {
                self.set_searching(show);
                if !show {
                    if let Some(s) = self.viewstack.visible_child_name() {
                        if s == "search" {
                            self.viewstack.set_visible_child_name("explore");
                        }
                    }
                }
            }
            AppMsg::SetVsChild(name) => {
                if name != self.vschild {
                    self.set_vschild(name.to_string());
                    if name != "search" {
                        sender.input(AppMsg::SetSearch(false))
                    }
                }
                if name == "updates" && self.online {
                    sender.input(AppMsg::CheckNetwork);
                }
            }
            AppMsg::SetVsBar(vsbar) => {
                self.set_showvsbar(vsbar);
            }
            AppMsg::Search(search) => {
                info!("AppMsg::Search");
                debug!("Searching for: {}", search);
                self.viewstack.set_visible_child_name("search");
                self.set_searchquery(search.to_string());
                let installeduserpkgs = self.installeduserpkgs.clone();
                let installedsystempkgs = self.installedsystempkgs.clone();
                let userpkgtype = self.userpkgtype.clone();
                let pkgdb = self.pkgdb.clone();
                let appdata = self.appdata.clone();
                sender.command(move |out, shutdown| {
                    let search = search.clone();
                    let installeduserpkgs = installeduserpkgs.clone();
                    let installedsystempkgs = installedsystempkgs;
                    let userpkgtype = userpkgtype.clone();
                    shutdown.register(async move {
                        let searchsplit: Vec<String> = search.split(' ').filter(|x| x.len() > 1).map(|x| x.to_string()).collect();
                        warn!("Searchsplit: {:?}", searchsplit);
                        if let Ok(pkgpool) = &SqlitePool::connect(&format!("sqlite://{}", pkgdb)).await {
                            let mut queryb: QueryBuilder<Sqlite> = QueryBuilder::new(
                                "SELECT pkgs.attribute, pkgs.pname, description, version FROM pkgs JOIN meta ON (pkgs.attribute = meta.attribute) WHERE (",
                            );
                            for (i, q) in searchsplit.iter().enumerate() {
                                if i == searchsplit.len() - 1 {
                                    queryb
                                        .push(r#"pkgs.attribute LIKE "#)
                                        .push_bind(format!("%{}%", q))
                                        .push(r#" OR description LIKE "#)
                                        .push_bind(format!("%{}%", q))
                                        .push(")");
                                } else {
                                    queryb
                                        .push(r#"pkgs.attribute LIKE "#)
                                        .push_bind(format!("%{}%", q))
                                        .push(r#" OR description LIKE "#)
                                        .push_bind(format!("%{}%", q))
                                        .push(r#") AND ("#);
                                }
                            }
                            queryb.push("ORDER BY LENGTH(pkgs.attribute) ASC");
                            let q: Vec<(String, String, String, String)> =
                                queryb.build_query_as().fetch_all(pkgpool).await.unwrap();
                            let mut outpkgs = Vec::new();
                            for (i, (attr, pname, desc, _version)) in q.into_iter().enumerate() {
                                if let Some(data) = appdata.get(&attr) {
                                    outpkgs.push(SearchItem {
                                        pkg: attr.to_string(),
                                        pname: pname.to_string(),
                                        name: if let Some(name) = &data.name { name.get("C").unwrap_or(&attr).to_string() } else { attr.to_string() },
                                        summary: if desc.is_empty() { None } else { Some(desc) },
                                        icon: data
                                            .icon
                                            .as_ref()
                                            .and_then(|x| x.cached.as_ref())
                                            .map(|x| x[0].name.clone()),
                                        installeduser: match userpkgtype {
                                          UserPkgs::Env => installeduserpkgs.contains_key(&pname),
                                          UserPkgs::Profile => installeduserpkgs.contains_key(&attr)
                                        },
                                        installedsystem: installedsystempkgs.contains(&attr),
                                    })
                                } else {
                                    outpkgs.push(SearchItem {
                                        pkg: attr.to_string(),
                                        pname: pname.to_string(),
                                        name: pname.to_string(),
                                        summary: if desc.is_empty() { None } else { Some(desc) },
                                        icon: None,
                                        installeduser: match userpkgtype {
                                          UserPkgs::Env => installeduserpkgs.contains_key(&pname),
                                          UserPkgs::Profile => installeduserpkgs.contains_key(&attr)
                                        },
                                        installedsystem: installedsystempkgs.contains(&attr),
                                    });
                                }
                                if i >= 200 {
                                    break;
                                }
                            }
                            outpkgs.sort_by(|a, b| {
                                let mut aleft = a.name.to_lowercase() + &a.pkg.to_lowercase();
                                let mut bleft = b.name.to_lowercase() + &b.pkg.to_lowercase();
                                for q in searchsplit.iter() {
                                    let q = &q.to_lowercase();
                                    if aleft.contains(q) {
                                        aleft = aleft.replace(q, "");
                                    } else {
                                        aleft.push_str(q);
                                    }
                                    if bleft.contains(q) {
                                        bleft = bleft.replace(q, "");
                                    } else {
                                        bleft.push_str(q);
                                    }
                                }
                                let mut apoints = aleft.len() + 5;
                                let mut bpoints = bleft.len() + 5;
                                // for q in searchsplit.iter() {
                                //     if a.name.contains(q) {
                                //         apoints -= 1;
                                //     }
                                //     if b.name.contains(q) {
                                //         bpoints -= 1;
                                //     }
                                // }
                                if appdata.get(&a.pkg).is_some() {
                                    apoints -= 5;
                                }
                                if appdata.get(&b.pkg).is_some() {
                                    bpoints -= 5;
                                }
                                apoints.cmp(&bpoints)
                            });
                            out.send(AppAsyncMsg::Search(search.to_string(), outpkgs));
                        }
                    }).drop_on_shutdown()
                })
            }
            AppMsg::AddInstalledToWorkQueue(work) => {
                let p = match work.pkgtype {
                    InstallType::User => work.pname.to_string(),
                    InstallType::System => work.pkg.to_string(),
                };
                self.installedpagebusy.push((p, work.pkgtype.clone()));
                self.pkgpage.emit(PkgMsg::AddToQueue(work));
            }
            AppMsg::RemoveInstalledBusy(work) => {
                let p = match work.pkgtype {
                    InstallType::User => work.pname.to_string(),
                    InstallType::System => work.pkg.to_string(),
                };
                self.installedpagebusy
                    .retain(|(x, y)| x != &p && y != &work.pkgtype);
                self.installedpage.emit(InstalledPageMsg::UnsetBusy(work));
            }
            AppMsg::OpenCategoryPage(category) => {
                info!("AppMsg::OpenCategoryPage({:?})", category);
                self.page = Page::FrontPage;
                self.mainpage = MainPage::CategoryPage;
                self.categorypage
                    .emit(CategoryPageMsg::Loading(category.clone()));
                sender.input(AppMsg::LoadCategory(category));
            }
            AppMsg::LoadCategory(category) => {
                info!("AppMsg::LoadCategory({:?})", category);
                let pkgdb = self.pkgdb.clone();
                let categoryrec = self.categoryrec.get(&category).unwrap_or(&vec![]).to_vec();
                let categoryall = self.categoryall.get(&category).unwrap_or(&vec![]).to_vec();
                let appdata = self.appdata.clone();
                let installeduser = self.installeduserpkgs.clone();
                let installedsystem = self.installedsystempkgs.clone();
                let category = category;
                sender.oneshot_command(async move {
                    let mut catrec = vec![];
                    let mut catall = vec![];
                    if let Ok(pool) = &SqlitePool::connect(&format!("sqlite://{}", pkgdb)).await {
                        for pkg in categoryrec {
                            if let Some(data) = appdata.get(&pkg) {
                                let pname: (String,) =
                                sqlx::query_as("SELECT pname FROM pkgs WHERE attribute = $1")
                                    .bind(&pkg)
                                    .fetch_one(pool)
                                    .await
                                    .unwrap();
                                catrec.push(CategoryTile {
                                    pkg: pkg.to_string(),
                                    name: if let Some(name) = &data.name {
                                        name.get("C").unwrap_or(&pname.0).to_string()
                                    } else {
                                        pname.0.to_string()
                                    },
                                    pname: pname.0,
                                    icon: data
                                        .icon
                                        .as_ref()
                                        .and_then(|x| x.cached.as_ref())
                                        .map(|x| x[0].name.clone()),
                                    summary: data
                                        .summary
                                        .as_ref()
                                        .and_then(|x| x.get("C"))
                                        .map(|x| x.to_string()),
                                    installeduser: installeduser.contains_key(&pkg),
                                    installedsystem: installedsystem.contains(&pkg),
                                })
                            } else {
                                let (pname, description): (String, String) =
                                sqlx::query_as("SELECT pname, description FROM pkgs JOIN meta ON (pkgs.attribute = meta.attribute) WHERE pkgs.attribute = $1")
                                    .bind(&pkg)
                                    .fetch_one(pool)
                                    .await
                                    .unwrap();
                                catrec.push(CategoryTile {
                                    pkg: pkg.to_string(),
                                    name: pname.to_string(),
                                    pname: pname.to_string(),
                                    icon: None,
                                    summary: if description.is_empty() { None } else { Some(description) },
                                    installeduser: installeduser.contains_key(&pkg),
                                    installedsystem: installedsystem.contains(&pkg),
                                })
                            }
                        }
                        for pkg in categoryall {
                            if let Some(data) = appdata.get(&pkg) {
                                let pname: (String,) =
                                sqlx::query_as("SELECT pname FROM pkgs WHERE attribute = $1")
                                    .bind(&pkg)
                                    .fetch_one(pool)
                                    .await
                                    .unwrap();
                                catall.push(CategoryTile {
                                    pkg: pkg.to_string(),
                                    name: if let Some(name) = &data.name {
                                        name.get("C").unwrap_or(&pname.0).to_string()
                                    } else {
                                        pname.0.to_string()
                                    },
                                    pname: pname.0,
                                    icon: data
                                        .icon
                                        .as_ref()
                                        .and_then(|x| x.cached.as_ref())
                                        .map(|x| x[0].name.clone()),
                                    summary: data
                                        .summary
                                        .as_ref()
                                        .and_then(|x| x.get("C"))
                                        .map(|x| x.to_string()),
                                    installeduser: installeduser.contains_key(&pkg),
                                    installedsystem: installedsystem.contains(&pkg),
                                })
                            } else {
                                let (pname, description): (String, String) =
                                sqlx::query_as("SELECT pname, description FROM pkgs JOIN meta ON (pkgs.attribute = meta.attribute) WHERE pkgs.attribute = $1")
                                    .bind(&pkg)
                                    .fetch_one(pool)
                                    .await
                                    .unwrap();
                                catall.push(CategoryTile {
                                    pkg: pkg.to_string(),
                                    name: pname.to_string(),
                                    pname: pname.to_string(),
                                    icon: None,
                                    summary: if description.is_empty() { None } else { Some(description) },
                                    installeduser: installeduser.contains_key(&pkg),
                                    installedsystem: installedsystem.contains(&pkg),
                                })
                            }
                        }
                    } else {
                        error!("Failed to connect to pkgdb")
                    }
                    AppAsyncMsg::LoadCategory(category, catrec, catall)
                });
            }
            AppMsg::SetDarkMode(dark) => {
                info!("AppMsg::SetDarkMode({})", dark);
                let scheme = if dark { "Adwaita-dark" } else { "Adwaita" };
                self.rebuild.emit(RebuildMsg::SetScheme(scheme.to_string()));
            }
            AppMsg::GetUnavailableItems(userpkgs, syspkgs, updatetype) => {
                info!("AppMsg::GetUnavailableItems");
                let appdata: HashMap<String, AppData> = self
                    .appdata
                    .iter()
                    .filter_map(|(k, v)| {
                        if syspkgs.contains_key(k) || userpkgs.contains_key(k) {
                            Some((k.to_string(), v.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();
                let poolref = self.pkgdb.clone();
                relm4::spawn(async move {
                    let mut unavailableuser = vec![];
                    let mut unavailablesys = vec![];
                    if let Ok(pool) = &SqlitePool::connect(&format!("sqlite://{}", poolref)).await {
                        let mut sortuserpkgs = userpkgs.into_iter().collect::<Vec<_>>();
                        sortuserpkgs.sort();
                        for (pkg, msg) in sortuserpkgs {
                            if let Some(data) = appdata.get(&pkg) {
                                let pname: Result<(String,), sqlx::Error> =
                                    sqlx::query_as("SELECT pname FROM pkgs WHERE attribute = $1")
                                        .bind(&pkg)
                                        .fetch_one(pool)
                                        .await;
                                if let Ok(pname) = pname {
                                    unavailableuser.push(UnavailableItemModel {
                                        pkg: pkg.to_string(),
                                        name: if let Some(name) = &data.name {
                                            name.get("C").unwrap_or(&pname.0).to_string()
                                        } else {
                                            pname.0.to_string()
                                        },
                                        pname: pname.0.to_string(),
                                        icon: data
                                            .icon
                                            .as_ref()
                                            .and_then(|x| x.cached.as_ref())
                                            .map(|x| x[0].name.clone()),
                                        message: msg,
                                    })
                                } else {
                                    unavailableuser.push(UnavailableItemModel {
                                        pkg: pkg.to_string(),
                                        name: if let Some(name) = &data.name {
                                            name.get("C").unwrap_or(&pkg).to_string()
                                        } else {
                                            pkg.to_string()
                                        },
                                        pname: String::new(),
                                        icon: data
                                            .icon
                                            .as_ref()
                                            .and_then(|x| x.cached.as_ref())
                                            .map(|x| x[0].name.clone()),
                                        message: msg,
                                    })
                                }
                            } else {
                                unavailableuser.push(UnavailableItemModel {
                                    pkg: pkg.to_string(),
                                    name: pkg.to_string(),
                                    pname: String::new(),
                                    icon: None,
                                    message: msg,
                                })
                            }
                        }
                        let mut sortsyspkgs = syspkgs.into_iter().collect::<Vec<_>>();
                        sortsyspkgs.sort();
                        for (pkg, msg) in sortsyspkgs {
                            if let Some(data) = appdata.get(&pkg) {
                                let pname: Result<(String,), sqlx::Error> =
                                    sqlx::query_as("SELECT pname FROM pkgs WHERE attribute = $1")
                                        .bind(&pkg)
                                        .fetch_one(pool)
                                        .await;
                                if let Ok(pname) = pname {
                                    unavailablesys.push(UnavailableItemModel {
                                        pkg: pkg.to_string(),
                                        name: if let Some(name) = &data.name {
                                            name.get("C").unwrap_or(&pname.0).to_string()
                                        } else {
                                            pname.0.to_string()
                                        },
                                        pname: pname.0.to_string(),
                                        icon: data
                                            .icon
                                            .as_ref()
                                            .and_then(|x| x.cached.as_ref())
                                            .map(|x| x[0].name.clone()),
                                        message: msg,
                                    })
                                } else {
                                    unavailablesys.push(UnavailableItemModel {
                                        pkg: pkg.to_string(),
                                        name: if let Some(name) = &data.name {
                                            name.get("C").unwrap_or(&pkg).to_string()
                                        } else {
                                            pkg.to_string()
                                        },
                                        pname: String::new(),
                                        icon: data
                                            .icon
                                            .as_ref()
                                            .and_then(|x| x.cached.as_ref())
                                            .map(|x| x[0].name.clone()),
                                        message: msg,
                                    })
                                }
                            } else {
                                unavailablesys.push(UnavailableItemModel {
                                    pkg: pkg.to_string(),
                                    name: pkg.to_string(),
                                    pname: String::new(),
                                    icon: None,
                                    message: msg,
                                })
                            }
                        }
                    }
                    UNAVAILABLE_BROKER.send(UnavailableDialogMsg::Show(
                        unavailableuser,
                        unavailablesys,
                        updatetype,
                    ));
                });
            }
            AppMsg::CheckNetwork => {
                let selfonline = self.online;
                let senderclone = sender.clone();
                sender.oneshot_command(async move {
                    info!("AppMsg::CheckNetwork");
                    let online = util::checkonline();
                    if online && !selfonline {
                        senderclone.input(AppMsg::UpdateDB);
                    }
                    AppAsyncMsg::SetNetwork(online)
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AppAsyncMsg::Search(search, pkgitems) => {
                if search == self.searchquery {
                    self.searchpage.emit(SearchPageMsg::Search(pkgitems))
                }
            }
            AppAsyncMsg::UpdateRecPkgs(pkgtiles) => {
                info!("AppAsyncMsg::UpdateRecPkgs");
                let mut recapps_guard = self.recommendedapps.guard();
                recapps_guard.clear();
                for tile in pkgtiles {
                    recapps_guard.push_back(tile);
                }
                recapps_guard.drop();
                sender.input(AppMsg::UpdateInstalledPkgs);
                info!("DONE AppAsyncMsg::UpdateRecPkgs");
            }
            AppAsyncMsg::UpdateInstalledPkgs(installedsystempkgs, installeduserpkgs) => {
                info!("AppAsyncMsg::UpdateInstalledPkgs");
                if installedsystempkgs != self.installedsystempkgs
                    || installeduserpkgs != self.installeduserpkgs
                {
                    warn!("Changes needed!");
                    self.installedsystempkgs = installedsystempkgs;
                    self.installeduserpkgs = installeduserpkgs;
                    sender.input(AppMsg::UpdateInstalledPage);
                    debug!("Getting recommended apps guard");
                    let mut recommendedapps_guard = self.recommendedapps.guard();
                    debug!("Got recommended apps guard");
                    for item in recommendedapps_guard.iter_mut() {
                        debug!("Got item {}", item.pkg);
                        item.installeduser =
                            self.installeduserpkgs.contains_key(match self.userpkgtype {
                                UserPkgs::Env => &item.pname,
                                UserPkgs::Profile => &item.pkg,
                            });
                        item.installedsystem = self.installedsystempkgs.contains(&item.pkg);
                    }
                    if self.searching {
                        self.searchpage.emit(SearchPageMsg::UpdateInstalled(
                            self.installeduserpkgs.keys().cloned().collect(),
                            self.installedsystempkgs.clone(),
                        ));
                    }
                }
                info!("DONE AppAsyncMsg::UpdateInstalledPkgs");
            }
            AppAsyncMsg::LoadCategory(category, catrec, catall) => {
                self.categorypage
                    .emit(CategoryPageMsg::Open(category, catrec, catall));
            }
            AppAsyncMsg::SetNetwork(online) => {
                self.online = online;
                self.updatepage.emit(UpdatePageMsg::UpdateOnline(online));
                self.pkgpage.emit(PkgMsg::UpdateOnline(online));
            }
        }
    }
}

relm4::new_action_group!(MenuActionGroup, "menu");
relm4::new_stateless_action!(AboutAction, MenuActionGroup, "about");
relm4::new_stateless_action!(PreferencesAction, MenuActionGroup, "preferences");
