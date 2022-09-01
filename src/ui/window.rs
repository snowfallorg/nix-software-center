use std::{collections::{HashMap, HashSet}, convert::identity, error::Error, process::Command, fs, io, path::{PathBuf, Path}, sync::Arc};
use ijson::IValue;
use relm4::{actions::*, factory::*, *};
use adw::prelude::*;
use edit_distance;
use serde_json::Value;
use spdx::Expression;
use crate::{parse::{packages::{Package, LicenseEnum, Platform}, cache::{uptodatelegacy, uptodateflake}, config::{NscConfig, getconfig, editconfig}}, ui::{installedpage::InstalledItem, pkgpage::PkgPageTypes}, APPINFO};

use super::{
    categories::{PkgGroup, PkgCategory},
    pkgtile::PkgTile,
    pkgpage::{PkgModel, PkgMsg, PkgInitModel, self, InstallType, WorkPkg},
    windowloading::{LoadErrorModel, LoadErrorMsg, WindowAsyncHandler, WindowAsyncHandlerMsg, CacheReturn}, searchpage::{SearchPageModel, SearchPageMsg, SearchItem}, installedpage::{InstalledPageModel, InstalledPageMsg}, updatepage::{UpdatePageModel, UpdatePageMsg, UpdateItem, UpdatePageInit}, about::{AboutPageModel, AboutPageMsg}, preferencespage::{PreferencesPageModel, PreferencesPageMsg}, categorypage::{CategoryPageModel, CategoryPageMsg}, categorytile::CategoryTile,
};

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

#[derive(Debug, PartialEq, Clone)]
pub enum SystemPkgs {
    Legacy,
    Flake,
}

#[derive(Debug, PartialEq, Clone)]
pub enum UserPkgs {
    Env,
    Profile,
}


#[tracker::track]
pub struct AppModel {
    application: adw::Application,
    mainwindow: adw::ApplicationWindow,
    config: NscConfig,
    #[tracker::no_eq]
    windowloading: WorkerController<WindowAsyncHandler>,
    #[tracker::no_eq]
    loaderrordialog: Controller<LoadErrorModel>,
    busy: bool,
    page: Page,
    mainpage: MainPage,
    #[tracker::no_eq]
    pkgs: HashMap<String, Package>,
    syspkgs: HashMap<String, String>,
    profilepkgs: Option<HashMap<String, String>>,
    // pkgset: HashSet<String>,
    pkgitems: HashMap<String, PkgItem>,
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
    installedpage: Controller<InstalledPageModel>,
    #[tracker::no_eq]
    updatepage: Controller<UpdatePageModel>,
    viewstack: adw::ViewStack,
    installedpagebusy: Vec<(String, InstallType)>,
}

#[derive(Debug)]
pub enum AppMsg {
    UpdateSysconfig(String),
    UpdateFlake(Option<String>),
    TryLoad,
    ReloadUpdate,
    Close,
    LoadError(String, String),
    Initialize(HashMap<String, Package>, Vec<String>, HashMap<String, String>, HashMap<PkgCategory, Vec<String>>, HashMap<PkgCategory, Vec<String>>, Option<HashMap<String, String>> /* profile pkgs */),
    ReloadUpdateItems(HashMap<String, Package>, HashMap<String, String>),
    OpenPkg(String),
    FrontPage,
    FrontFrontPage,
    UpdatePkgs(Option<Vec<String>>),
    UpdateInstalledPkgs,
    UpdateUpdatePkgs,
    UpdateCategoryPkgs,
    // AddUserPkg(String),
    // RemoveUserPkg(String),
    // AddSystemPkg(String),
    // RemoveSystemPkg(String),
    SetSearch(bool),
    SetVsBar(bool),
    SetVsChild(String),
    Search(String),
    OpenAboutPage,
    OpenPreferencesPage,
    AddInstalledToWorkQueue(WorkPkg),
    RemoveInstalledBusy(WorkPkg),
    OpenCategoryPage(PkgCategory),
    LoadCategory(PkgCategory)
    // OpenWithScrnshots(String, Option<Vec<String>>),
}

#[derive(Debug, Clone, PartialEq)]
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
    Search(String, Vec<SearchItem>)
}

#[relm4::component(pub)]
impl Component for AppModel {
    type Init = adw::Application;
    type Input = AppMsg;
    type Output = ();
    type Widgets = AppWidgets;
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
                        gtk::Spinner {
                            set_spinning: true,
                            set_height_request: 80,
                        },
                        gtk::Label {
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
                                        println!("TOGGLED TO {}", x.is_active());
                                        sender.input(AppMsg::SetSearch(x.is_active()))
                                    } @searchtoggle
    
                                },
                                #[name(viewswitchertitle)]
                                #[wrap(Some)]
                                set_title_widget = &adw::ViewSwitcherTitle {
                                    set_title: "Nix Software Center",
                                    set_stack: Some(viewstack),
                                    connect_title_visible_notify[sender] => move |x| {
                                        println!("TITLE NOTIFY: {}", x.is_title_visible());
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
                                    println!("VISIBLE CHILD NOTIFY: {:?}", x.visible_child_name());
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
                                                set_min_children_per_line: 2,
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

    fn init(
        application: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {

        let userpkgtype = if let Ok(h) = std::env::var("HOME") {
            if Path::new(&format!("{}/.nix-profile/manifest.json", h)).exists() {
                UserPkgs::Profile
            } else {
                UserPkgs::Env
            }
        } else {
            UserPkgs::Env
        };

        let syspkgtype = match fs::read_to_string("/run/current-system/nixos-version") {
            Ok(s) => {
                if let Some(last) = s.split('.').last() {
                    if last.len() == 7 {
                        SystemPkgs::Flake
                    } else {
                        SystemPkgs::Legacy
                    }
                } else {
                    SystemPkgs::Legacy
                }
            }
            Err(_) => SystemPkgs::Legacy,
        };
        println!("userpkgtype: {:?}", userpkgtype);
        println!("syspkgtype: {:?}", syspkgtype);


        let windowloading = WindowAsyncHandler::builder()
            .detach_worker(())
            .forward(sender.input_sender(), identity);
        let loaderrordialog = LoadErrorModel::builder()
            .launch(root.clone().upcast())
            .forward(sender.input_sender(), identity);
        let pkgpage = PkgModel::builder()
            .launch(PkgPageTypes {
                userpkgs: userpkgtype.clone(),
                syspkgs: syspkgtype.clone(),
            })
            .forward(sender.input_sender(), identity);
        let searchpage = SearchPageModel::builder()
            .launch(())
            .forward(sender.input_sender(), identity);
        let categorypage = CategoryPageModel::builder()
            .launch(())
            .forward(sender.input_sender(), identity);
        let installedpage = InstalledPageModel::builder()
            .launch(())
            .forward(sender.input_sender(), identity);
        let updatepage = UpdatePageModel::builder()
        // ADD FLAKE DETECTION
            .launch(UpdatePageInit { window: root.clone().upcast(), systype: syspkgtype.clone(), usertype: userpkgtype.clone() })
            .forward(sender.input_sender(), identity);
        let viewstack = adw::ViewStack::new();

        let config = getconfig();

        let model = AppModel {
            application,
            mainwindow: root.clone(),
            config,
            windowloading,
            loaderrordialog,
            busy: true,
            page: Page::FrontPage,
            mainpage: MainPage::FrontPage,
            pkgs: HashMap::new(),
            syspkgs: HashMap::new(),
            pkgitems: HashMap::new(),
            installeduserpkgs: HashMap::new(),
            installedsystempkgs: HashSet::new(),
            profilepkgs: None,
            syspkgtype,
            userpkgtype,
            categoryrec: HashMap::new(),
            categoryall: HashMap::new(),
            recommendedapps: FactoryVecDeque::new(gtk::FlowBox::new(), &sender.input),
            categories: FactoryVecDeque::new(gtk::FlowBox::new(), &sender.input),
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
            tracker: 0,
        };

        model.windowloading.emit(WindowAsyncHandlerMsg::CheckCache(CacheReturn::Init, model.syspkgtype.clone(), model.userpkgtype.clone()));
        let recbox = model.recommendedapps.widget();
        let categorybox = model.categories.widget();
        let viewstack = &model.viewstack;

        let widgets = view_output!();

        let group = RelmActionGroup::<MenuActionGroup>::new();
        let aboutpage: RelmAction<AboutAction> = {
            let sender = sender.clone();
            RelmAction::new_stateless(move |_| {
                sender.input(AppMsg::OpenAboutPage);
            })
        };

        let prefernecespage: RelmAction<PreferencesAction> = {
            let sender = sender;
            RelmAction::new_stateless(move |_| {
                sender.input(AppMsg::OpenPreferencesPage);
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
        frontvs.set_icon_name(Some("compass"));
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.reset();
        match msg {
            AppMsg::TryLoad => {
                self.busy = true;
                self.windowloading.emit(WindowAsyncHandlerMsg::CheckCache(CacheReturn::Init, self.syspkgtype.clone(), self.userpkgtype.clone()));
            }
            AppMsg::ReloadUpdate => {
                self.windowloading.emit(WindowAsyncHandlerMsg::CheckCache(CacheReturn::Update, self.syspkgtype.clone(), self.userpkgtype.clone()));
            }
            AppMsg::Close => {
                self.application.quit();
            }
            AppMsg::LoadError(msg, msg2) => {
                self.busy = false;
                self.loaderrordialog.emit(LoadErrorMsg::Show(msg, msg2));
            }
            AppMsg::UpdateSysconfig(systemconfig) => {
                self.config = NscConfig {
                    systemconfig,
                    flake: self.config.flake.clone(),
                };
                if editconfig(self.config.clone()).is_err() {
                    eprintln!("Failed to update config");
                }
                self.pkgpage.emit(PkgMsg::UpdateConfig(self.config.clone()));
                self.updatepage.emit(UpdatePageMsg::UpdateConfig(self.config.clone()));
                sender.input(AppMsg::UpdatePkgs(None))
            }
            AppMsg::UpdateFlake(flake) => {
                self.config = NscConfig {
                    systemconfig: self.config.systemconfig.clone(),
                    flake,
                };
                if editconfig(self.config.clone()).is_err() {
                    eprintln!("Failed to update config");
                }
                self.pkgpage.emit(PkgMsg::UpdateConfig(self.config.clone()));
                self.updatepage.emit(UpdatePageMsg::UpdateConfig(self.config.clone()));
            }
            AppMsg::Initialize(pkgs, recommendedapps, syspkgs, categoryrec, categoryall, profilepkgs) => {
                self.syspkgs = syspkgs;
                self.profilepkgs  = profilepkgs;
                self.categoryrec = categoryrec;
                self.categoryall = categoryall;
                let mut pkgitems = HashMap::new();
                for (pkg, pkgdata) in &pkgs {
                    // if let Some(pkgdata) = pkgs.get(*pkg) {
                        // println!("GOT DATA FOR {}", pkg);
                        let pname = pkgdata.pname.to_string();
                        let mut name = pkgdata.pname.to_string();
                        let version = pkgdata.version.to_string();
                        let mut icon = None;
                        let mut summary = pkgdata.meta.description.as_ref().map(|x| x.to_string());
                        if let Some(appdata) = &pkgdata.appdata {
                            // println!("GOT APPDATA FOR {}", pkg);
                            if let Some(i) = &appdata.icon {
                                if let Some(mut iconvec) = i.cached.clone() {
                                    iconvec.sort_by(|a, b| a.height.cmp(&b.height));
                                    icon = Some(iconvec[0].name.clone());
                                }
                            }
                            if let Some(s) = &appdata.summary {
                                summary = Some(s.get("C").unwrap_or(&summary.unwrap_or_default()).to_string());
                            }
                            if let Some(n) = &appdata.name {
                                name = n.get("C").unwrap_or(&name).to_string();
                            }
                        }
                        pkgitems.insert(pkg.to_string(), PkgItem {
                            pkg: pkg.to_string(),
                            pname,
                            name,
                            version,
                            icon,
                            summary,
                        });
                    // }
                }
                self.pkgitems = pkgitems;
                self.page = Page::FrontPage;
                self.pkgpage.emit(PkgMsg::UpdateConfig(self.config.clone()));
                self.updatepage.emit(UpdatePageMsg::UpdateConfig(self.config.clone()));
                self.pkgs = pkgs;
                sender.input(AppMsg::UpdatePkgs(Some(recommendedapps)));

                let mut cat_guard = self.categories.guard();
                for c in vec![
                    PkgCategory::Audio,
                    PkgCategory::Development,
                    PkgCategory::Games,
                    PkgCategory::Graphics,
                    PkgCategory::Network,
                    PkgCategory::Video,
                ] {
                    cat_guard.push_back(c);
                }
                cat_guard.drop();
                self.busy = false;
            }
            AppMsg::ReloadUpdateItems(pkgs, syspkgs) => {
                self.syspkgs = syspkgs;
                let mut pkgitems = HashMap::new();
                for (pkg, pkgdata) in &pkgs {
                    // if let Some(pkgdata) = pkgs.get(*pkg) {
                        // println!("GOT DATA FOR {}", pkg);
                        let pname = pkgdata.pname.to_string();
                        let mut name = pkgdata.pname.to_string();
                        let version = pkgdata.version.to_string();
                        let mut icon = None;
                        let mut summary = pkgdata.meta.description.as_ref().map(|x| x.to_string());
                        if let Some(appdata) = &pkgdata.appdata {
                            // println!("GOT APPDATA FOR {}", pkg);
                            if let Some(i) = &appdata.icon {
                                if let Some(mut iconvec) = i.cached.clone() {
                                    iconvec.sort_by(|a, b| a.height.cmp(&b.height));
                                    icon = Some(iconvec[0].name.clone());
                                }
                            }
                            if let Some(s) = &appdata.summary {
                                summary = Some(s.get("C").unwrap_or(&summary.unwrap_or_default()).to_string());
                            }
                            if let Some(n) = &appdata.name {
                                name = n.get("C").unwrap_or(&name).to_string();
                            }
                        }
                        pkgitems.insert(pkg.to_string(), PkgItem {
                            pkg: pkg.to_string(),
                            pname,
                            name,
                            version,
                            icon,
                            summary,
                        });
                    // }
                }
                self.pkgitems = pkgitems;
                sender.input(AppMsg::UpdatePkgs(None));
                self.updatepage.emit(UpdatePageMsg::DoneLoading);
            }
            AppMsg::OpenPkg(pkg) => {
                // if let Some(pkgs) = &self.pkgs {
                    if let Some(input) = self.pkgs.get(&pkg) {
                        let mut name = input.pname.to_string();
                        let mut summary = input.meta.description.as_ref().map(|x| x.to_string());
                        let mut description = input.meta.longdescription.as_ref().map(|x| x.to_string());
                        let mut icon = None;
                        let mut screenshots = vec![];
                        let mut licenses = vec![];
                        let mut platforms = vec![];
                        let mut maintainers = vec![];
                        let mut launchable = None;
    
                        fn addlicense(pkglicense: &LicenseEnum, licenses: &mut Vec<pkgpage::License>) {
                            match pkglicense {
                                LicenseEnum::Single(l) => {
                                    if let Some(n) = &l.fullname {
                                        let parsed = if let Some(id) = &l.spdxid {
                                            if let Ok(Some(license)) = Expression::parse(id).map(|p| p.requirements().map(|er| er.req.license.id()).collect::<Vec<_>>()[0]) {
                                                Some(license)
                                            } else {
                                                None
                                            }
                                        } else if let Ok(Some(license)) = Expression::parse(n).map(|p| p.requirements().map(|er| er.req.license.id()).collect::<Vec<_>>()[0]) {
                                            Some(license)
                                        } else {
                                            None
                                        };
                                        licenses.push(pkgpage::License {
                                            free: if let Some(f) = l.free { Some(f) } else { parsed.map(|p| p.is_osi_approved() || p.is_fsf_free_libre() )},
                                            fullname: n.to_string(),
                                            spdxid: l.spdxid.clone().map(|x| x.to_string()),
                                            url: if let Some(u) = &l.url { Some(u.to_string()) } else { parsed.map(|p| format!("https://spdx.org/licenses/{}.html", p.name))},
                                        })
                                    } else if let Some(s) = &l.spdxid {
                                        if let Ok(Some(license)) = Expression::parse(s).map(|p| p.requirements().map(|er| er.req.license.id()).collect::<Vec<_>>()[0]) {
                                            licenses.push(pkgpage::License {
                                                free: Some(license.is_osi_approved() || license.is_fsf_free_libre() || l.free.unwrap_or(false)),
                                                fullname: license.full_name.to_string(),
                                                spdxid: Some(license.name.to_string()),
                                                url: if l.url.is_some() {
                                                    l.url.clone().map(|x| x.to_string())
                                                } else {
                                                    Some(format!("https://spdx.org/licenses/{}.html", license.name))
                                                },
                                            })
                                        }
                                    }
                                },
                                LicenseEnum::List(lst) => {
                                    for l in lst {
                                        addlicense(&LicenseEnum::Single(l.clone()), licenses);
                                    }
                                },
                                LicenseEnum::SingleStr(s) => {
                                    if let Ok(Some(license)) = Expression::parse(s).map(|p| p.requirements().map(|er| er.req.license.id()).collect::<Vec<_>>()[0]) {
                                        licenses.push(pkgpage::License {
                                            free: Some(license.is_osi_approved() || license.is_fsf_free_libre()),
                                            fullname: license.full_name.to_string(),
                                            spdxid: Some(license.name.to_string()),
                                            url: Some(format!("https://spdx.org/licenses/{}.html", license.name)),
                                        })
                                    }
                                },
                                LicenseEnum::VecStr(lst) => {
                                    for s in lst {
                                        addlicense(&LicenseEnum::SingleStr(s.clone()), licenses);
                                    }
                                },
                                LicenseEnum::Mixed(v) => {
                                    for l in v {
                                        addlicense(l, licenses);
                                    }
                                }
                            }
                        } 
    
                        if let Some(pkglicense) = &input.meta.license {
                            addlicense(pkglicense, &mut licenses);
                        }
    
                        if let Some(data) = &input.appdata {
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
                                            if let Some(index) = screenshots.iter().position(|x| *x == u.url) {
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
    
                        if let Some(p) = &input.meta.platforms {
                            match p {
                                Platform::Single(p) => {
                                    if !platforms.contains(&p.to_string()) && p != &input.system {
                                        platforms.push(p.to_string());
                                    }
                                },
                                Platform::List(v) => {
                                    for p in v {
                                        if !platforms.contains(&p.to_string()) && p != &input.system {
                                            platforms.push(p.to_string());
                                        }
                                    }
                                },
                                Platform::ListList(vv) => {
                                    for v in vv {
                                        for p in v {
                                            if !platforms.contains(&p.to_string()) && p != &input.system {
                                                platforms.push(p.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        platforms.sort();
                        platforms.insert(0, input.system.to_string());
    
                        if let Some(m) = input.meta.maintainers.clone() {
                            for m in m {
                                maintainers.push(m);
                            }
                        }
    
                        let out = PkgInitModel {
                            name,
                            pname: input.pname.to_string(),
                            summary,
                            description,
                            icon,
                            pkg,
                            screenshots,
                            homepage: input.meta.homepage.clone(),
                            platforms,
                            licenses,
                            maintainers,
                            installeduserpkgs: self.installeduserpkgs.keys().cloned().collect(),
                            installedsystempkgs: self.installedsystempkgs.clone(),
                            launchable
                        };
                        self.page = Page::PkgPage;
                        if self.viewstack.visible_child_name() != Some(gtk::glib::GString::from("search")) {
                            self.searching = false;
                        }
                        self.busy = false;
                        self.pkgpage.emit(PkgMsg::Open(Box::new(out)));
                    }    
            }
            AppMsg::FrontPage => {
                self.page = Page::FrontPage;
                // sender.input(AppMsg::UpdatePkgs(None));
            }
            AppMsg::FrontFrontPage => {
                self.page = Page::FrontPage;
                self.mainpage = MainPage::FrontPage;
                // sender.input(AppMsg::UpdatePkgs(None));
            }
            AppMsg::UpdatePkgs(rec) => {
                println!("UPDATE PKGS");
                fn getsystempkgs(config: &str) -> Result<HashSet<String>, Box<dyn Error>> {
                    let f = fs::read_to_string(config)?;
                    match nix_editor::read::getarrvals(&f, "environment.systemPackages") {
                        Ok(x) => Ok(HashSet::from_iter(x.into_iter())),
                        Err(_) => Err(Box::new(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Failed to read value from configuration",
                        )))
                    }
                }
                
                fn getuserenvpkgs() -> Result<HashMap<String, String>, Box<dyn Error>> {
                    let out = Command::new("nix-env").arg("-q").arg("--json").output()?;
                    let data: IValue = serde_json::from_str(&String::from_utf8_lossy(&out.stdout))?;
                    let mut pcurrpkgs = HashMap::new();
                    for (_, pkg) in data.as_object().unwrap() {
                        pcurrpkgs.insert(
                            pkg.as_object().unwrap()["pname"]
                                .as_string()
                                .unwrap()
                                .to_string(),
                            pkg.as_object().unwrap()["version"]
                                .as_string()
                                .unwrap()
                                .to_string()
                        );
                    }
                    Ok(pcurrpkgs)
                }

                fn getuserprofilepkgs() -> Result<HashMap<String, String>, Box<dyn Error>> {
                    let data: IValue = serde_json::from_str(&fs::read_to_string(Path::new(&format!("{}/.nix-profile/manifest.json", std::env::var("HOME")?)))?)?;
                    let mut pcurrpkgs = HashMap::new();
                    for pkg in data.as_object().unwrap()["elements"].as_array().unwrap().iter() {
                        if let Some(p) = pkg.get("attrPath") {
                            if let Some(pkgname) = p.as_string()
                            .unwrap()
                            // Change to current platform
                            .strip_prefix("legacyPackages.x86_64-linux.") {
                                if let Some(sp) = pkg.get("storePaths") {
                                    if let Some(sp) = sp.as_array().unwrap().get(0) {
                                        let storepath = sp.as_string().unwrap().to_string();
                                        let output = Command::new("nix")
                                            .arg("show-derivation")
                                            .arg(&storepath)
                                            .output()?;
                                        let data: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
                                        if let Some(version) = data.as_object().unwrap().values().next().unwrap()["env"].get("version") {
                                            let version = version.as_str().unwrap().to_string();
                                            pcurrpkgs.insert(
                                                pkgname.to_string(),
                                                version,
                                            );
                                        } else {
                                            pcurrpkgs.insert(
                                                pkgname.to_string(),
                                                String::default(),
                                            );
                                        }
                                        
                                    }
                                }
                            }
                        }
                    }
                    println!("CURRENT PROFILE PKGS: {:?}", pcurrpkgs);
                    Ok(pcurrpkgs)
                }

                let systempkgs = match getsystempkgs(&self.config.systemconfig) {
                    Ok(x) => x,
                    Err(_) => {
                        self.installedsystempkgs.clone()
                    }
                };

                let userpkgs = match self.userpkgtype {
                    UserPkgs::Env => {
                        match getuserenvpkgs() {
                            Ok(out) => out,
                            Err(_) => {
                                self.installeduserpkgs.clone()
                            }
                        }
                    },
                    UserPkgs::Profile => {
                        match getuserprofilepkgs() {
                            Ok(out) => out,
                            Err(_) => {
                                self.installeduserpkgs.clone()
                            }
                        }
                    }
                };

                self.installedsystempkgs = systempkgs;
                self.installeduserpkgs = userpkgs;

                if let Some(recommendedapps) = rec {
                    let mut recapps_guard = self.recommendedapps.guard();
                    for app in recommendedapps {
                        if let Some(x) = self.pkgs.get(&app) {
                            if let Some(data) = &x.appdata {
                                if let Some(icon) = &data.icon {
                                    if let Some(summary) = &data.summary {
                                        if let Some(name) = &data.name {
                                            let name = name.get("C").unwrap().to_string();
                                            let mut iconvec = icon.cached.as_ref().unwrap().to_vec();
                                            iconvec.sort_by(|a, b| a.height.cmp(&b.height));
                                            let summary =
                                                summary.get("C").unwrap_or(&String::new()).to_string();
                                            recapps_guard.push_back(PkgTile {
                                                pkg: app,
                                                name,
                                                pname: x.pname.to_string(),
                                                icon: Some(iconvec[0].name.clone()),
                                                summary,
                                                installeduser: self.installeduserpkgs.contains_key(&x.pname.to_string()),
                                                installedsystem: self.installedsystempkgs.contains(&x.pname.to_string()),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    recapps_guard.drop();
                } else {
                    let mut pkgtile_guard = self.recommendedapps.guard();
                    for i in 0..pkgtile_guard.len() {
                        if let Some(pkgtile) = &mut pkgtile_guard.get_mut(i) {
                            pkgtile.installedsystem = self.installedsystempkgs.contains(&pkgtile.pkg);
                            pkgtile.installeduser = self.installeduserpkgs.contains_key(&pkgtile.pname);
                        }
                    }
                    pkgtile_guard.drop();
                }

                sender.input(AppMsg::UpdateInstalledPkgs);
                sender.input(AppMsg::UpdateUpdatePkgs);
                sender.input(AppMsg::UpdateCategoryPkgs);

                println!("UPDATE SEARCH");
                if self.searching {
                    self.update_searching(|_| ());
                    self.searchpage.emit(SearchPageMsg::UpdateInstalled(self.installeduserpkgs.keys().cloned().collect(), self.installedsystempkgs.clone()));
                }
                println!("FINISHED UPDATE SEARCH");
            }
            AppMsg::UpdateInstalledPkgs => {
                let mut installeduseritems = vec![];
                match self.userpkgtype {
                    UserPkgs::Env => {
                        for installedpname in self.installeduserpkgs.keys() {
                            let possibleitems = self.pkgitems.iter().filter(|(_, x)| &x.pname == installedpname);
                            let count = possibleitems.clone().count();
                            match count {
                                1 => {
                                    let (pkg, data) = possibleitems.collect::<Vec<_>>()[0];
                                installeduseritems.push(InstalledItem {
                                    name: data.name.clone(),
                                    pname: data.pname.clone(),
                                    pkg: Some(pkg.clone()),
                                    summary: data.summary.clone(),
                                    icon: data.icon.clone(),
                                    pkgtype: InstallType::User,
                                    busy: self.installedpagebusy.contains(&(data.pname.clone(), InstallType::User)),
                                })
                                }
                                2.. => {
                                    installeduseritems.push(InstalledItem {
                                        name: installedpname.clone(),
                                        pname: installedpname.clone(),
                                        pkg: None,
                                        summary: None, //data.summary.clone(),
                                        icon: None, //data.icon.clone(),
                                        pkgtype: InstallType::User,
                                        busy: self.installedpagebusy.contains(&(installedpname.clone(), InstallType::User)),
                                    })
                                }
                                _ => {}
                            }
                        }
                    }
                    UserPkgs::Profile => {
                        for installedpkg in self.installeduserpkgs.keys() {
                            if let Some(item) = self.pkgitems.get(installedpkg) {
                                installeduseritems.push(InstalledItem {
                                    name: item.name.clone(),
                                    pname: item.pname.clone(),
                                    pkg: Some(item.pkg.clone()),
                                    summary: item.summary.clone(),
                                    icon: item.icon.clone(),
                                    pkgtype: InstallType::User,
                                    busy: self.installedpagebusy.contains(&(item.pkg.clone(), InstallType::User)),
                                })
                            }
                        }
                    }
                }

                installeduseritems.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                let mut installedsystemitems = vec![];
                for installedpkg in &self.installedsystempkgs {
                    if let Some(item) = self.pkgitems.get(installedpkg) {
                        installedsystemitems.push(InstalledItem {
                            name: item.name.clone(),
                            pname: item.pname.clone(),
                            pkg: Some(item.pkg.clone()),
                            summary: item.summary.clone(),
                            icon: item.icon.clone(),
                            pkgtype: InstallType::System,
                            busy: self.installedpagebusy.contains(&(item.pkg.clone(), InstallType::System)),
                        })
                    }
                }
                installedsystemitems.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                self.installedpage.emit(InstalledPageMsg::Update(installeduseritems, installedsystemitems));
            }
            AppMsg::UpdateUpdatePkgs => {
                println!("InstalledUserPkgs: {:?}", self.installeduserpkgs);
                println!("InstalledSystemPkgs: {:?}", self.installedsystempkgs);
                let mut updateuseritems = vec![];
                match self.userpkgtype {
                    UserPkgs::Env => {
                        for (installedpname, version) in self.installeduserpkgs.iter() {
                            let possibleitems = self.pkgitems.iter().filter(|(_, x)| &x.pname == installedpname);
                            let count = possibleitems.clone().count();
                            match count {
                                1 => {
                                    let (pkg, data) = possibleitems.collect::<Vec<_>>()[0];
                                    if &data.version != version {
                                        updateuseritems.push(UpdateItem {
                                            name: data.name.clone(),
                                            pname: data.pname.clone(),
                                            pkg: Some(pkg.clone()),
                                            summary: data.summary.clone(),
                                            icon: data.icon.clone(),
                                            pkgtype: InstallType::User,
                                            verfrom: Some(version.clone()),
                                            verto: Some(data.version.clone()),
                                        })
                                    } else {
                                        println!("Pkg {} is up to date", pkg);
                                    }
                                }
                                2.. => {
                                    let mut update = true;
                                    for (pkg, _) in possibleitems {
                                        if let Some(ver) = self.syspkgs.get(pkg) {
                                            if version == ver {
                                                update = false;
                                            }
        
                                        }
                                    }
                                    if update {
                                        updateuseritems.push(UpdateItem {
                                            name: installedpname.clone(),
                                            pname: installedpname.clone(),
                                            pkg: None,
                                            summary: None, //data.summary.clone(),
                                            icon: None, //data.icon.clone(),
                                            pkgtype: InstallType::User,
                                            verfrom: Some(version.clone()),
                                            verto: None,
                                        })
                                    } else {
                                        println!("Pkg {} is up to date", installedpname);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    UserPkgs::Profile => {
                        for (installedpkg, version) in &self.installeduserpkgs {
                            if let Some(item) = self.pkgitems.get(installedpkg) {
                                if let Some(profilepkgs) = &self.profilepkgs {
                                    if let Some(newver) = profilepkgs.get(installedpkg) {
                                        if version != newver {
                                            updateuseritems.push(UpdateItem {
                                                name: item.name.clone(),
                                                pname: item.pname.clone(),
                                                pkg: Some(item.pkg.clone()),
                                                summary: item.summary.clone(),
                                                icon: item.icon.clone(),
                                                pkgtype: InstallType::User,
                                                verfrom: Some(version.clone()),
                                                verto: Some(newver.clone()),
                                            })
                                        } else {
                                            println!("Pkg {} is up to date. Ver: {}", item.pname, version);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                updateuseritems.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                let mut updatesystemitems = vec![];
                for installedpkg in &self.installedsystempkgs {
                    if let Some(item) = self.pkgitems.get(installedpkg) {
                        if let Some(sysver) = self.syspkgs.get(installedpkg) {
                            if &item.version != sysver {
                                updatesystemitems.push(UpdateItem {
                                    name: item.name.clone(),
                                    pname: item.pname.clone(),
                                    pkg: Some(item.pkg.clone()),
                                    summary: item.summary.clone(),
                                    icon: item.icon.clone(),
                                    pkgtype: InstallType::System,
                                    verfrom: Some(sysver.clone()),
                                    verto: Some(item.version.clone()),

                                })
                            } else {
                                println!("Pkg {} is up to date", item.pkg);
                            }
                        }
                    }
                }
                updatesystemitems.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                match self.syspkgtype {
                    SystemPkgs::Legacy => {
                        if let Ok(Some((old, new))) = uptodatelegacy() {
                            updatesystemitems.insert(0, UpdateItem {
                                name: String::from("NixOS System"),
                                pname: String::new(),
                                pkg: None,
                                summary: Some(String::from("NixOS internal packages and modules")),
                                icon: None,
                                pkgtype: InstallType::System,
                                verfrom: Some(old),
                                verto: Some(new),
                            })
                        }
                    }
                    SystemPkgs::Flake => {
                        if let Ok(Some((old, new))) = uptodateflake() {
                            updatesystemitems.insert(0, UpdateItem {
                                name: String::from("NixOS System"),
                                pname: String::new(),
                                pkg: None,
                                summary: Some(String::from("NixOS internal packages and modules")),
                                icon: None,
                                pkgtype: InstallType::System,
                                verfrom: Some(old),
                                verto: Some(new),
                            })
                        }
                    }
                }
                self.updatepage.emit(UpdatePageMsg::Update(updateuseritems, updatesystemitems));
            }
            AppMsg::UpdateCategoryPkgs => {
                self.categorypage.emit(CategoryPageMsg::UpdateInstalled(self.installeduserpkgs.keys().cloned().collect::<Vec<_>>(), self.installedsystempkgs.iter().cloned().collect::<Vec<_>>()));
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
            }
            AppMsg::SetVsBar(vsbar) => {
                self.set_showvsbar(vsbar);
            }
            AppMsg::Search(search) => {
                println!("SEARCHING FOR: {}", search);
                self.viewstack.set_visible_child_name("search");
                self.searchpage.emit(SearchPageMsg::Open);
                self.set_searchquery(search.to_string());
                // let pkgs = self.pkgs.iter().map(|x| );
                let pkgitems: Vec<PkgItem> = self.pkgitems.values().cloned().collect();
                let installeduserpkgs = self.installeduserpkgs.clone();
                let installedsystempkgs = self.installedsystempkgs.clone();
                sender.command(move |out, shutdown| {
                    let pkgs = pkgitems.clone();
                    let search = search.clone();
                    let installeduserpkgs = installeduserpkgs.clone();
                    let installedsystempkgs = installedsystempkgs;
                    shutdown.register(async move {
                        let searchsplit: Vec<String> = search.split(' ').filter(|x| x.len() > 1).map(|x| x.to_string()).collect();
                        let mut namepkgs = pkgs.iter().filter(|x| searchsplit.iter().any(|s| x.name.to_lowercase().contains(&s.to_lowercase()))).collect::<Vec<&PkgItem>>();
                        let mut pnamepkgs = pkgs.iter().filter(|x| searchsplit.iter().any(|s| x.pname.to_lowercase().contains(&s.to_lowercase())) && !namepkgs.contains(x)).collect::<Vec<&PkgItem>>();
                        let mut pkgpkgs = pkgs.iter().filter(|x| searchsplit.iter().any(|s| x.pkg.to_lowercase().contains(&s.to_lowercase())) && !namepkgs.contains(x) && !pnamepkgs.contains(x)).collect::<Vec<&PkgItem>>();
                        let mut sumpkgs = pkgs.iter().filter(|x| if let Some(sum) = &x.summary { searchsplit.iter().any(|s| sum.to_lowercase().contains(&s.to_lowercase())) } else { false } && !namepkgs.contains(x) && !pnamepkgs.contains(x) && !pkgpkgs.contains(x)).collect::<Vec<&PkgItem>>();
                        println!("FOUND {} PACKAGES", namepkgs.len() + pnamepkgs.len() + pkgpkgs.len() + sumpkgs.len());
                        namepkgs.sort_by(|a, b| edit_distance::edit_distance(&a.name.to_lowercase(), &search.to_lowercase()).cmp(&edit_distance::edit_distance(&b.name.to_lowercase(), &search.to_lowercase())));
                        pnamepkgs.sort_by(|a, b| edit_distance::edit_distance(&a.pname.to_lowercase(), &search.to_lowercase()).cmp(&edit_distance::edit_distance(&b.pname.to_lowercase(), &search.to_lowercase())));
                        pkgpkgs.sort_by(|a, b| edit_distance::edit_distance(&a.pkg.to_lowercase(), &search.to_lowercase()).cmp(&edit_distance::edit_distance(&b.pkg.to_lowercase(), &search.to_lowercase())));
                        sumpkgs.sort_by(|a, b| {
                            let mut x = 0;
                            for s in &searchsplit {
                                x += a.summary.as_ref().unwrap_or(&String::new()).to_lowercase().matches(&s.to_lowercase()).count()
                            }
                            let mut y = 0;
                            for s in &searchsplit {
                                y += b.summary.as_ref().unwrap_or(&String::new()).to_lowercase().matches(&s.to_lowercase()).count()
                            }
                            x.cmp(&y)
                        });
                        
                        
                        let mut combpkgs = namepkgs;
                        combpkgs.append(&mut pnamepkgs);
                        combpkgs.append(&mut pkgpkgs);
                        combpkgs.append(&mut sumpkgs);

                        combpkgs.sort_by(|a, b| {
                            b.icon.is_some().cmp(&a.icon.is_some())
                        });
                        // namepkgs.sort_by(|a, b| a.name.cmp(&b.name));
                        // tokio::time::sleep(Duration::from_secs(1)).await;
                        // out.send(AppMsg::Increment);
                        let mut outpkgs: Vec<SearchItem> = vec![];
                        for (i, p) in combpkgs.iter().enumerate() {
                            outpkgs.push(SearchItem {
                                pkg: p.pkg.to_string(),
                                pname: p.pname.to_string(),
                                name: p.name.to_string(),
                                summary: p.summary.clone(),
                                icon: p.icon.clone(),
                                installeduser: installeduserpkgs.contains_key(&p.pname),
                                installedsystem: installedsystempkgs.contains(&p.pname),
                            });
                            if i > 100 {
                                break;
                            }
                        }
                        out.send(AppAsyncMsg::Search(search.to_string(), outpkgs))
                    }).drop_on_shutdown()
                })
            }
            AppMsg::OpenAboutPage => {
                let aboutpage = AboutPageModel::builder()
                    .launch(self.mainwindow.clone().upcast())
                    .forward(sender.input_sender(), identity);
                aboutpage.emit(AboutPageMsg::Show);
            }
            AppMsg::OpenPreferencesPage => {
                let preferencespage = PreferencesPageModel::builder()
                    .launch(self.mainwindow.clone().upcast())
                    .forward(sender.input_sender(), identity);
                if let Some(flake) = &self.config.flake {
                    let flakeparts = flake.split('#').collect::<Vec<&str>>();
                    if let Some(p) = flakeparts.first() {
                        let path = PathBuf::from(p);
                        let args = flakeparts.get(1).unwrap_or(&"").to_string();
                        preferencespage.emit(PreferencesPageMsg::Show(PathBuf::from(&self.config.systemconfig), Some((path, args))))
                    } else {
                        preferencespage.emit(PreferencesPageMsg::Show(PathBuf::from(&self.config.systemconfig), None))
                    }
                } else {
                    preferencespage.emit(PreferencesPageMsg::Show(PathBuf::from(&self.config.systemconfig), None))
                }
            }
            AppMsg::AddInstalledToWorkQueue(work) => {
                println!("ADDING INSTALLED TO WORK QUEUE {:?}", work);
                let p = match work.pkgtype {
                    InstallType::User => work.pname.to_string(),
                    InstallType::System => work.pkg.to_string(),
                };
                self.installedpagebusy.push((p, work.pkgtype.clone()));
                self.pkgpage.emit(PkgMsg::AddToQueue(work));
                println!("INSTALLEDBUSY: {:?}", self.installedpagebusy);
            }
            AppMsg::RemoveInstalledBusy(work) => {
                println!("REMOVE INSTALLED BUSY {:?}", work);
                let p = match work.pkgtype {
                    InstallType::User => work.pname.to_string(),
                    InstallType::System => work.pkg.to_string(),
                };
                self.installedpagebusy.retain(|(x, y)| x != &p && y != &work.pkgtype);
                self.installedpage.emit(InstalledPageMsg::UnsetBusy(work));
            }
            AppMsg::OpenCategoryPage(category) => {
                println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!! OPEN CATEGORY PAGE {:?}", category);
                self.page = Page::FrontPage;
                self.mainpage = MainPage::CategoryPage;
                // self.categorypage.emit(CategoryPageMsg::Open(category.clone(), self.categoryrec.get(&category).unwrap_or(&vec![]).to_vec(), self.categoryall.get(&category).unwrap_or(&vec![]).to_vec()));
                sender.input(AppMsg::LoadCategory(category));
            }
            AppMsg::LoadCategory(category) => {
                let mut catrec = vec![];
                for app in self.categoryrec.get(&category).unwrap_or(&vec![]) {
                    if let Some(x) = self.pkgs.get(app) {
                        let mut name = x.pname.to_string();
                        let mut icon = None;
                        let mut summary = x.meta.description.clone().map(|x| x.to_string());
                        if let Some(data) = &x.appdata {
                            if let Some(i) = &data.icon {
                                let mut iconvec = i.cached.as_ref().unwrap().to_vec();
                                iconvec.sort_by(|a, b| a.height.cmp(&b.height));
                                icon = Some(iconvec[0].name.clone()); 
                            }
                            if let Some(s) = &data.summary {
                                summary =
                                s.get("C").map(|x| x.to_string());
                                
                            }
                            if let Some(n) = &data.name {
                                name = n.get("C").unwrap().to_string();
                            }
                        }
                        catrec.push(CategoryTile {
                            pkg: app.to_string(),
                            name,
                            pname: x.pname.to_string(),
                            icon,
                            summary,
                            installeduser: self.installeduserpkgs.contains_key(&x.pname.to_string()),
                            installedsystem: self.installedsystempkgs.contains(&x.pname.to_string()),
                        });
                    }
                }

                let mut catall = vec![];
                for app in self.categoryall.get(&category).unwrap_or(&vec![]) {
                    if let Some(x) = self.pkgs.get(app) {
                        let mut name = x.pname.to_string();
                        let mut icon = None;
                        let mut summary = x.meta.description.clone().map(|x| x.to_string());
                        if let Some(data) = &x.appdata {
                            if let Some(i) = &data.icon {
                                let mut iconvec = i.cached.as_ref().unwrap().to_vec();
                                iconvec.sort_by(|a, b| a.height.cmp(&b.height));
                                icon = Some(iconvec[0].name.clone()); 
                            }
                            if let Some(s) = &data.summary {
                                summary =
                                s.get("C").map(|x| x.to_string());
                                
                            }
                            if let Some(n) = &data.name {
                                name = n.get("C").unwrap().to_string();
                            }
                        }
                        catall.push(CategoryTile {
                            pkg: app.to_string(),
                            name,
                            pname: x.pname.to_string(),
                            icon,
                            summary,
                            installeduser: self.installeduserpkgs.contains_key(&x.pname.to_string()),
                            installedsystem: self.installedsystempkgs.contains(&x.pname.to_string()),
                        });
                    }
                }

                self.categorypage.emit(CategoryPageMsg::Open(category, catrec, catall));

            }
        }
    }

    fn update_cmd(&mut self, msg: Self::CommandOutput, _sender: ComponentSender<Self>) {
        match msg {
            AppAsyncMsg::Search(search, pkgitems) => {
                if search == self.searchquery {
                    self.searchpage.emit(SearchPageMsg::Search(pkgitems))
                }
            }
        }
    }
}

relm4::new_action_group!(MenuActionGroup, "menu");
relm4::new_stateless_action!(AboutAction, MenuActionGroup, "about");
relm4::new_stateless_action!(PreferencesAction, MenuActionGroup, "preferences");
