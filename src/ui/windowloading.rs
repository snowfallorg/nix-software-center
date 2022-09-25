use super::window::AppMsg;
use super::window::SystemPkgs;
use crate::parse::cache::checkcache;
use crate::parse::config::NscConfig;
use crate::parse::packages::readflakesyspkgs;
use crate::parse::packages::readpkgs;
use crate::parse::packages::readlegacysyspkgs;
use crate::parse::packages::Package;
use crate::parse::packages::readprofilepkgs;
use crate::ui::categories::PkgCategory;
use crate::ui::window::UserPkgs;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use relm4::adw::prelude::*;
use relm4::*;
use std::{collections::HashMap, env};
use strum::IntoEnumIterator;
use log::*;

pub struct WindowAsyncHandler;

#[derive(Debug)]
pub enum WindowAsyncHandlerMsg {
    CheckCache(CacheReturn, SystemPkgs, UserPkgs, NscConfig),
}

#[derive(Debug, PartialEq)]
pub enum CacheReturn {
    Init,
    Update,
}

impl Worker for WindowAsyncHandler {
    type InitParams = ();
    type Input = WindowAsyncHandlerMsg;
    type Output = AppMsg;

    fn init(_params: Self::InitParams, _sender: relm4::ComponentSender<Self>) -> Self {
        Self
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            WindowAsyncHandlerMsg::CheckCache(cr, syspkgs, userpkgs, config) => {
                info!("WindowAsyncHandlerMsg::CheckCache");
                let syspkgs2 = syspkgs.clone();
                let userpkgs2 = userpkgs.clone();
                let config = config.clone();
                relm4::spawn(async move {
                    match checkcache(syspkgs2, userpkgs2, config) {
                        Ok(_) => {}
                        Err(e) => {
                            warn!("FAILED TO CHECK CACHE");
                            warn!("{}", e);
                            sender.output(AppMsg::LoadError(
                                String::from("Could not load cache"),
                                String::from(
                                    "Try connecting to the internet or launching the application again",
                                ),
                            ));
                            return;
                        }
                    }
                    let pkgs = match readpkgs().await {
                        Ok(pkgs) => pkgs,
                        Err(e) => {
                            warn!("FAILED TO LOAD PKGS");
                            warn!("{}", e);
                            sender.output(AppMsg::LoadError(
                                String::from("Could not load packages"),
                                String::from(
                                    "Try connecting to the internet or launching the application again",
                                ),
                            ));
                            return;
                        }
                    };

                    let newpkgs = match syspkgs {
                        SystemPkgs::Legacy => {
                            match readlegacysyspkgs() {
                                Ok(newpkgs) => newpkgs,
                                Err(e) => {
                                    warn!("FAILED TO LOAD NEW PKGS");
                                    warn!("{}", e);
                                    sender.output(AppMsg::LoadError(
                                        String::from("Could not load new packages"),
                                        String::from(
                                            "Try connecting to the internet or launching the application again",
                                        ),
                                    ));
                                    return;
                                }
                            }
                        }
                        SystemPkgs::Flake => {
                            match readflakesyspkgs() {
                                Ok(newpkgs) => newpkgs,
                                Err(e) => {
                                    warn!("FAILED TO LOAD NEW PKGS");
                                    warn!("{}", e);
                                    sender.output(AppMsg::LoadError(
                                        String::from("Could not load new packages"),
                                        String::from(
                                            "Try connecting to the internet or launching the application again",
                                        ),
                                    ));
                                    return;
                                }
                            }
                        }
                        SystemPkgs::None => {
                            HashMap::new()
                        }
                    };

                    let profilepkgs = match userpkgs {
                        UserPkgs::Env => None,
                        UserPkgs::Profile => if let Ok(r) = readprofilepkgs() { Some(r) } else { None },
                    };

                    let mut recpicks = vec![];
                    let mut catpicks: HashMap<PkgCategory, Vec<String>> = HashMap::new();
                    let mut catpkgs: HashMap<PkgCategory, Vec<String>> = HashMap::new();


                    if cr == CacheReturn::Init {
                        let desktopenv = env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
                        let appdatapkgs = pkgs
                            .iter()
                            .filter(|(x, _)| {
                                if let Some(p) = pkgs.get(*x) {
                                    if let Some(data) = &p.appdata {
                                        (if let Some(i) = &data.icon {
                                            i.cached.is_some()
                                        } else {
                                            false
                                        }) && data.description.is_some()
                                            && data.name.is_some()
                                            && data.launchable.is_some()
                                            && data.screenshots.is_some()
                                            && (!x.starts_with("gnome.") || desktopenv == "GNOME")
                                            && (!x.starts_with("xfce.") || desktopenv == "XFCE")
                                            && (!x.starts_with("mate.") || desktopenv == "MATE")
                                            && (!x.starts_with("cinnamon.")
                                                || desktopenv == "X-Cinnamon")
                                            && (!x.starts_with("libsForQt5") || desktopenv == "KDE")
                                            && (!x.starts_with("pantheon.")
                                                || desktopenv == "Pantheon")
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            })
                            .collect::<HashMap<_, _>>();

                        let mut recommendedpkgs = appdatapkgs
                            .keys()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>();
                        let mut rng = thread_rng();
                        recommendedpkgs.shuffle(&mut rng);

                        let mut desktoppicks = recommendedpkgs
                            .iter()
                            .filter(|x| {
                                if desktopenv == "GNOME" {
                                    x.starts_with("gnome.") || x.starts_with("gnome-")
                                } else if desktopenv == "XFCE" {
                                    x.starts_with("xfce.")
                                } else if desktopenv == "MATE" {
                                    x.starts_with("mate.")
                                } else if desktopenv == "X-Cinnamon" {
                                    x.starts_with("cinnamon.")
                                } else if desktopenv == "KDE" {
                                    x.starts_with("libsForQt5")
                                } else if desktopenv == "Pantheon" {
                                    x.starts_with("pantheon.")
                                } else {
                                    false
                                }
                            })
                            .collect::<Vec<_>>();

                        for p in desktoppicks.iter().take(3) {
                            recpicks.push(p.to_string());
                        }
                        for category in PkgCategory::iter() {
                            desktoppicks.shuffle(&mut rng);
                            let mut cvec = vec![];
                            let mut allvec = vec![];
                            let mut rpkgs = recommendedpkgs.clone();
                            fn checkpkgs(
                                pkg: String,
                                pkgs: &HashMap<&String, &Package>,
                                category: PkgCategory,
                            ) -> bool {
                                match category {
                                    PkgCategory::Audio => {
                                        // Audio:
                                        // - pkgs/applications/audio
                                        if let Some(p) = pkgs.get(&pkg) {
                                            if let Some(pos) = &p.meta.position {
                                                if pos.starts_with("pkgs/applications/audio") {
                                                    return true;
                                                }
                                            }
                                            if let Some(data) = &p.appdata {
                                                if let Some(categories) = &data.categories {
                                                    if categories.contains(&String::from("Audio")) {
                                                        return true;
                                                    }
                                                }
                                            }
                                        }
                                        false
                                    }
                                    PkgCategory::Development => {
                                        // Development:
                                        // - pkgs/development
                                        // - pkgs/applications/terminal-emulators
                                        // - xdg: Development
                                        if let Some(p) = pkgs.get(&pkg) {
                                            if let Some(pos) = &p.meta.position {
                                                if pos.starts_with("pkgs/development")
                                                    || pos.starts_with(
                                                        "pkgs/applications/terminal-emulators",
                                                    )
                                                {
                                                    return true;
                                                }
                                            }
                                            if let Some(data) = &p.appdata {
                                                if let Some(categories) = &data.categories {
                                                    if categories
                                                        .contains(&String::from("Development"))
                                                    {
                                                        return true;
                                                    }
                                                }
                                            }
                                        }
                                        false
                                    }
                                    PkgCategory::Games => {
                                        // Games:
                                        // - pkgs/games
                                        // - pkgs/applications/emulators
                                        // - pkgs/tools/games
                                        // - xdg::Games
                                        if let Some(p) = pkgs.get(&pkg) {
                                            if let Some(pos) = &p.meta.position {
                                                if pos.starts_with("pkgs/games")
                                                    || pos.starts_with(
                                                        "pkgs/applications/emulators",
                                                    )
                                                    || pos.starts_with("pkgs/tools/games")
                                                {
                                                    return true;
                                                }
                                            }
                                            if let Some(data) = &p.appdata {
                                                if let Some(categories) = &data.categories {
                                                    if categories.contains(&String::from("Games")) {
                                                        return true;
                                                    }
                                                }
                                            }
                                        }
                                        false
                                    }
                                    PkgCategory::Graphics => {
                                        // Graphics:
                                        // - pkgs/applications/graphics
                                        // - xdg: Graphics
                                        if let Some(p) = pkgs.get(&pkg) {
                                            if let Some(pos) = &p.meta.position {
                                                if pos.starts_with("pkgs/applications/graphics")
                                                    || pos.starts_with("xdg:Graphics")
                                                {
                                                    return true;
                                                }
                                            }
                                            if let Some(data) = &p.appdata {
                                                if let Some(categories) = &data.categories {
                                                    if categories.contains(&String::from("Graphics")) {
                                                        return true;
                                                    }
                                                }
                                            }
                                        }                                        
                                        false
                                    }
                                    PkgCategory::Network => {
                                        // Network:
                                        // - pkgs/applications/networking
                                        // - xdg: Network
                                        if let Some(p) = pkgs.get(&pkg) {
                                            if let Some(pos) = &p.meta.position {
                                                if pos.starts_with("pkgs/applications/networking")
                                                    || pos.starts_with("xdg:Network")
                                                {
                                                    return true;
                                                }
                                            }
                                            if let Some(data) = &p.appdata {
                                                if let Some(categories) = &data.categories {
                                                    if categories.contains(&String::from("Network")) {
                                                        return true;
                                                    }
                                                }
                                            }
                                        }                                        
                                        false
                                    }
                                    PkgCategory::Video => {
                                        // Video:
                                        // - pkgs/applications/video
                                        // - xdg: Video
                                        if let Some(p) = pkgs.get(&pkg) {
                                            if let Some(pos) = &p.meta.position {
                                                if pos.starts_with("pkgs/applications/video")
                                                    || pos.starts_with("xdg:Video")
                                                {
                                                    return true;
                                                }
                                            }
                                            if let Some(data) = &p.appdata {
                                                if let Some(categories) = &data.categories {
                                                    if categories.contains(&String::from("Video")) {
                                                        return true;
                                                    }
                                                }
                                            }
                                        }
                                        false
                                    }
                                }
                            }

                            for pkg in desktoppicks.iter().take(3) {
                                if checkpkgs(pkg.to_string(), &appdatapkgs, category.clone()) {
                                    cvec.push(pkg.to_string());
                                }
                            }

                            while cvec.len() < 12 {
                                if let Some(pkg) = rpkgs.pop() {
                                    if !cvec.contains(&pkg.to_string())
                                        && checkpkgs(
                                            pkg.to_string(),
                                            &appdatapkgs,
                                            category.clone(),
                                        )
                                    {
                                        cvec.push(pkg.to_string());
                                    }
                                } else {
                                    break;
                                }
                            }

                            let catagortypkgs = pkgs
                            .iter()
                            .filter(|(x, _)| {
                                if let Some(p) = appdatapkgs.get(*x) {
                                    if let Some(position) = &p.meta.position {
                                        (position.starts_with("pkgs/applications/audio") && category == PkgCategory::Audio)
                                        || (position.starts_with("pkgs/applications/terminal-emulators") && category == PkgCategory::Development)
                                        || (position.starts_with("pkgs/applications/emulators") && category == PkgCategory::Games)
                                        || (position.starts_with("pkgs/applications/graphics") && category == PkgCategory::Graphics)
                                        || (position.starts_with("pkgs/applications/networking") && category == PkgCategory::Network)
                                        || (position.starts_with("pkgs/applications/video") && category == PkgCategory::Video)
                                        || (position.starts_with("pkgs/tools/games") && category == PkgCategory::Games)
                                        || (position.starts_with("pkgs/games") && category == PkgCategory::Games)
                                        || (position.starts_with("pkgs/development") && category == PkgCategory::Development)
                                        || appdatapkgs.contains_key(x)
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            })
                            .collect::<HashMap<_, _>>();

                            for pkg in catagortypkgs.keys() {
                                if checkpkgs(pkg.to_string(), &catagortypkgs, category.clone()) {
                                    allvec.push(pkg.to_string());
                                }
                            }

                            cvec.shuffle(&mut rng);
                            allvec.sort_by_key(|x| x.to_lowercase());
                            catpicks.insert(category.clone(), cvec);
                            catpkgs.insert(category.clone(), allvec);
                        }

                        while recpicks.len() < 12 {
                            if let Some(p) = recommendedpkgs.pop() {
                                if !recpicks.contains(&p.to_string()) {
                                    recpicks.push(p);
                                }
                            } else {
                                break;
                            }
                        }
                        recpicks.shuffle(&mut rng);
                    }

                    match cr {
                        CacheReturn::Init => {
                            sender.output(AppMsg::Initialize(pkgs, recpicks, newpkgs, catpicks, catpkgs, profilepkgs));
                        }
                        CacheReturn::Update => {
                            sender.output(AppMsg::ReloadUpdateItems(pkgs, newpkgs));
                        }
                    }
                });
            }
        }
    }
}

pub struct LoadErrorModel {
    hidden: bool,
    msg: String,
    msg2: String,
}

#[derive(Debug)]
pub enum LoadErrorMsg {
    Show(String, String),
    Retry,
    Close,
    // Preferences,
}

#[relm4::component(pub)]
impl SimpleComponent for LoadErrorModel {
    type InitParams = gtk::Window;
    type Input = LoadErrorMsg;
    type Output = AppMsg;
    type Widgets = LoadErrorWidgets;

    view! {
        dialog = gtk::MessageDialog {
            set_transient_for: Some(&parent_window),
            set_modal: true,
            #[watch]
            set_visible: !model.hidden,
            #[watch]
            set_text: Some(&model.msg),
            #[watch]
            set_secondary_text: Some(&model.msg2),
            set_use_markup: true,
            set_secondary_use_markup: true,
            add_button: ("Retry", gtk::ResponseType::Accept),
            // add_button: ("Preferences", gtk::ResponseType::Help),
            add_button: ("Quit", gtk::ResponseType::Close),
            connect_response[sender] => move |_, resp| {
                sender.input(match resp {
                    gtk::ResponseType::Accept => LoadErrorMsg::Retry,
                    gtk::ResponseType::Close => LoadErrorMsg::Close,
                    // gtk::ResponseType::Help => LoadErrorMsg::Preferences,
                    _ => unreachable!(),
                });
            },
        }
    }

    fn init(
        parent_window: Self::InitParams,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = LoadErrorModel {
            hidden: true,
            msg: String::default(),
            msg2: String::default(),
        };
        let widgets = view_output!();
        let accept_widget = widgets
            .dialog
            .widget_for_response(gtk::ResponseType::Accept)
            .expect("No button for accept response set");
        accept_widget.add_css_class("warning");
        // let pref_widget = widgets
        //     .dialog
        //     .widget_for_response(gtk::ResponseType::Help)
        //     .expect("No button for help response set");
        // pref_widget.add_css_class("suggested-action");
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            LoadErrorMsg::Show(s, s2) => {
                self.hidden = false;
                self.msg = s;
                self.msg2 = s2;
            }
            LoadErrorMsg::Retry => {
                self.hidden = true;
                sender.output(AppMsg::TryLoad)
            }
            LoadErrorMsg::Close => sender.output(AppMsg::Close),
            // LoadErrorMsg::Preferences => sender.output(AppMsg::ShowPrefMenu),
        }
    }
}
