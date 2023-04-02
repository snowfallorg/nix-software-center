use super::window::AppMsg;
use super::window::SystemPkgs;
use crate::parse::packages::appsteamdata;
use crate::parse::packages::AppData;
use crate::ui::categories::PkgCategory;
use crate::ui::window::UserPkgs;
use log::*;
use nix_data::config::configfile::NixDataConfig;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use relm4::adw::prelude::*;
use relm4::*;
use sqlx::SqlitePool;
use std::path::Path;
use std::{collections::HashMap, env};

pub struct WindowAsyncHandler;

#[derive(Debug)]
pub enum WindowAsyncHandlerMsg {
    CheckCache(SystemPkgs, UserPkgs, NixDataConfig),
    UpdateDB(SystemPkgs, UserPkgs),
}

impl Worker for WindowAsyncHandler {
    type Init = ();
    type Input = WindowAsyncHandlerMsg;
    type Output = AppMsg;

    fn init(_params: Self::Init, _sender: relm4::ComponentSender<Self>) -> Self {
        Self
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            WindowAsyncHandlerMsg::CheckCache(syspkgs, userpkgs, _config) => {
                info!("WindowAsyncHandlerMsg::CheckCache");
                relm4::spawn(async move {
                    let mut recpicks = vec![];
                    let mut catpicks: HashMap<PkgCategory, Vec<String>> = HashMap::new();
                    let mut catpkgs: HashMap<PkgCategory, Vec<String>> = HashMap::new();

                    let nixos = Path::new("/etc/NIXOS").exists();

                    let pkgdb = if nixos {
                        match nix_data::cache::nixos::nixospkgs().await {
                            Ok(p) => p,
                            Err(e) => {
                                error!("Error getting NixOS pkgs: {}", e);
                                let _ = sender.output(AppMsg::LoadError(
                                    String::from("Error retrieving NixOS package database"),
                                    e.to_string(),
                                ));
                                return;
                            }
                        }
                    } else {
                        match nix_data::cache::nonnixos::nixpkgs().await {
                            Ok(p) => p,
                            Err(e) => {
                                error!("Error getting nixpkgs: {}", e);
                                let _ = sender.output(AppMsg::LoadError(
                                    String::from("Error retrieving nixpkgs package database"),
                                    e.to_string(),
                                ));
                                return;
                            }
                        }
                    };

                    let pool = match SqlitePool::connect(&format!("sqlite://{}", pkgdb)).await {
                        Ok(p) => p,
                        Err(e) => {
                            error!("Error connecting to pkgdb: {}", e);
                            let _ = sender.output(AppMsg::LoadError(
                                String::from("Error connecting to package database"),
                                e.to_string(),
                            ));
                            return;
                        }
                    };

                    let nixpkgsdb = match userpkgs {
                        UserPkgs::Profile => {
                            if let Ok(x) = nix_data::cache::profile::nixpkgslatest().await {
                                Some(x)
                            } else {
                                None
                            }
                        }
                        UserPkgs::Env => None,
                    };

                    let systemdb = match syspkgs {
                        SystemPkgs::None => None,
                        SystemPkgs::Legacy => {
                            if let Ok(x) = nix_data::cache::channel::legacypkgs().await {
                                Some(x)
                            } else {
                                None
                            }
                        }
                        SystemPkgs::Flake => {
                            if let Ok(x) = nix_data::cache::flakes::flakespkgs().await {
                                Some(x)
                            } else {
                                None
                            }
                        }
                    };

                    let pkglist: Vec<(String,)> = match sqlx::query_as("SELECT attribute FROM pkgs")
                        .fetch_all(&pool)
                        .await
                    {
                        Ok(x) => x,
                        Err(e) => {
                            error!("Error getting pkglist: {}", e);
                            let _ = sender.output(AppMsg::LoadError(
                                String::from("Malformed package database"),
                                e.to_string(),
                            ));
                            return;
                        }
                    };

                    let pkglist = pkglist.iter().map(|x| x.0.clone()).collect::<Vec<String>>();

                    let posvec: Vec<(String, String)> =
                        match sqlx::query_as("SELECT attribute, position FROM meta")
                            .fetch_all(&pool)
                            .await
                        {
                            Ok(x) => x,
                            Err(e) => {
                                error!("Error getting package metadata: {}", e);
                                let _ = sender.output(AppMsg::LoadError(
                                    String::from("Malformed package database"),
                                    e.to_string(),
                                ));
                                return;
                            }
                        };
                    let appdata = match appsteamdata() {
                        Ok(x) => x,
                        Err(e) => {
                            error!("Error getting appdata: {}", e);
                            let _ = sender.output(AppMsg::LoadError(
                                String::from("Error retrieving appstream data"),
                                e.to_string(),
                            ));
                            return;
                        }
                    };
                    let desktopenv = env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();

                    let mut recpkgs = pkglist
                        .iter()
                        .filter(|x| {
                            if let Some(data) = appdata.get(&x.to_string()) {
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
                                    && (!x.starts_with("cinnamon.") || desktopenv == "X-Cinnamon")
                                    && (!x.starts_with("libsForQt5") || desktopenv == "KDE")
                                    && (!x.starts_with("pantheon.") || desktopenv == "Pantheon")
                            } else {
                                false
                            }
                        })
                        .collect::<Vec<_>>();

                    let mut rng = thread_rng();
                    recpkgs.shuffle(&mut rng);

                    let mut desktoppicks = recpkgs
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

                    let pospkgs = posvec
                        .into_iter()
                        .map(|(x, y)| (x, if y.is_empty() { None } else { Some(y) }))
                        .collect::<HashMap<String, Option<String>>>();

                    for category in vec![
                        PkgCategory::Audio,
                        PkgCategory::Development,
                        PkgCategory::Games,
                        PkgCategory::Graphics,
                        PkgCategory::Web,
                        PkgCategory::Video,
                    ] {
                        desktoppicks.shuffle(&mut rng);
                        let mut cvec = vec![];
                        let mut allvec = vec![];
                        let mut rpkgs = recpkgs.clone();
                        fn checkpkgs(
                            pkg: String,
                            pospkgs: &HashMap<String, Option<String>>,
                            appdata: &HashMap<String, AppData>,
                            category: PkgCategory,
                        ) -> bool {
                            match category {
                                PkgCategory::Audio => {
                                    // Audio:
                                    // - pkgs/applications/audio
                                    if let Some(Some(pos)) = pospkgs.get(&pkg) {
                                        if pos.starts_with("pkgs/applications/audio") {
                                            return true;
                                        }
                                        if let Some(data) = appdata.get(&pkg) {
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
                                    if let Some(Some(pos)) = pospkgs.get(&pkg) {
                                        if pos.starts_with("pkgs/development")
                                            || pos
                                                .starts_with("pkgs/applications/terminal-emulators")
                                        {
                                            return true;
                                        }
                                        if let Some(data) = appdata.get(&pkg) {
                                            if let Some(categories) = &data.categories {
                                                if categories.contains(&String::from("Development"))
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
                                    if let Some(Some(pos)) = pospkgs.get(&pkg) {
                                        if pos.starts_with("pkgs/games")
                                            || pos.starts_with("pkgs/applications/emulators")
                                            || pos.starts_with("pkgs/tools/games")
                                        {
                                            return true;
                                        }
                                        if let Some(data) = &appdata.get(&pkg) {
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
                                    if let Some(Some(pos)) = pospkgs.get(&pkg) {
                                        if pos.starts_with("pkgs/applications/graphics")
                                            || pos.starts_with("xdg:Graphics")
                                        {
                                            return true;
                                        }
                                        if let Some(data) = &appdata.get(&pkg) {
                                            if let Some(categories) = &data.categories {
                                                if categories.contains(&String::from("Graphics")) {
                                                    return true;
                                                }
                                            }
                                        }
                                    }
                                    false
                                }
                                PkgCategory::Web => {
                                    // Web:
                                    // - pkgs/applications/networking
                                    // - xdg: Network
                                    if let Some(Some(pos)) = pospkgs.get(&pkg) {
                                        if pos.starts_with("pkgs/applications/networking")
                                            || pos.starts_with("xdg:Network")
                                        {
                                            return true;
                                        }
                                        if let Some(data) = &appdata.get(&pkg) {
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
                                    if let Some(Some(pos)) = pospkgs.get(&pkg) {
                                        if pos.starts_with("pkgs/applications/video")
                                            || pos.starts_with("xdg:Video")
                                        {
                                            return true;
                                        }
                                        if let Some(data) = &appdata.get(&pkg) {
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
                            if checkpkgs(pkg.to_string(), &pospkgs, &appdata, category.clone()) {
                                cvec.push(pkg.to_string());
                            }
                        }

                        while cvec.len() < 12 {
                            if let Some(pkg) = rpkgs.pop() {
                                if !cvec.contains(&pkg.to_string())
                                    && checkpkgs(
                                        pkg.to_string(),
                                        &pospkgs,
                                        &appdata,
                                        category.clone(),
                                    )
                                {
                                    cvec.push(pkg.to_string());
                                }
                            } else {
                                break;
                            }
                        }

                        let catagortypkgs = pkglist
                            .iter()
                            .filter(|x| {
                                if appdata.get(*x).is_some() {
                                    if let Some(Some(position)) = &pospkgs.get(*x) {
                                        (position.starts_with("pkgs/applications/audio")
                                            && category == PkgCategory::Audio)
                                            || (position.starts_with(
                                                "pkgs/applications/terminal-emulators",
                                            ) && category == PkgCategory::Development)
                                            || (position.starts_with("pkgs/applications/emulators")
                                                && category == PkgCategory::Games)
                                            || (position.starts_with("pkgs/applications/graphics")
                                                && category == PkgCategory::Graphics)
                                            || (position
                                                .starts_with("pkgs/applications/networking")
                                                && category == PkgCategory::Web)
                                            || (position.starts_with("pkgs/applications/video")
                                                && category == PkgCategory::Video)
                                            || (position.starts_with("pkgs/tools/games")
                                                && category == PkgCategory::Games)
                                            || (position.starts_with("pkgs/games")
                                                && category == PkgCategory::Games)
                                            || (position.starts_with("pkgs/development")
                                                && category == PkgCategory::Development)
                                            || recpkgs.contains(x)
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            })
                            .collect::<Vec<_>>();

                        for pkg in catagortypkgs {
                            if checkpkgs(pkg.to_string(), &pospkgs, &appdata, category.clone()) {
                                allvec.push(pkg.to_string());
                            }
                        }

                        cvec.shuffle(&mut rng);
                        allvec.sort_by_key(|x| x.to_lowercase());
                        catpicks.insert(category.clone(), cvec);
                        catpkgs.insert(category.clone(), allvec);
                    }

                    while recpicks.len() < 12 {
                        if let Some(p) = recpkgs.pop() {
                            if !recpicks.contains(&p.to_string()) {
                                recpicks.push(p.to_string());
                            }
                        } else {
                            break;
                        }
                    }
                    recpicks.shuffle(&mut rng);

                    sender.output(AppMsg::Initialize(
                        pkgdb, nixpkgsdb, systemdb, appdata, recpicks, catpicks, catpkgs,
                    ));
                });
            }
            WindowAsyncHandlerMsg::UpdateDB(syspkgs, userpkgs) => {
                relm4::spawn(async move {
                    let nixos = Path::new("/etc/NIXOS").exists();

                    let _pkgdb = if nixos {
                        match nix_data::cache::nixos::nixospkgs().await {
                            Ok(p) => p,
                            Err(e) => {
                                error!("Error getting NixOS pkgs: {}", e);
                                sender.output(AppMsg::LoadError(
                                    String::from("Error retrieving NixOS package database"),
                                    e.to_string(),
                                ));
                                return;
                            }
                        }
                    } else {
                        match nix_data::cache::nonnixos::nixpkgs().await {
                            Ok(p) => p,
                            Err(e) => {
                                error!("Error getting nixpkgs: {}", e);
                                sender.output(AppMsg::LoadError(
                                    String::from("Error retrieving nixpkgs package database"),
                                    e.to_string(),
                                ));
                                return;
                            }
                        }
                    };

                    let _nixpkgsdb = match userpkgs {
                        UserPkgs::Profile => {
                            if let Ok(x) = nix_data::cache::profile::nixpkgslatest().await {
                                Some(x)
                            } else {
                                None
                            }
                        }
                        UserPkgs::Env => None,
                    };

                    let _systemdb = match syspkgs {
                        SystemPkgs::None => None,
                        SystemPkgs::Legacy => {
                            if let Ok(x) = nix_data::cache::channel::legacypkgs().await {
                                Some(x)
                            } else {
                                None
                            }
                        }
                        SystemPkgs::Flake => {
                            if let Ok(x) = nix_data::cache::flakes::flakespkgs().await {
                                Some(x)
                            } else {
                                None
                            }
                        }
                    };
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
}

#[relm4::component(pub)]
impl SimpleComponent for LoadErrorModel {
    type Init = gtk::Window;
    type Input = LoadErrorMsg;
    type Output = AppMsg;

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
        parent_window: Self::Init,
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
                sender.output(AppMsg::TryLoad);
            }
            LoadErrorMsg::Close => {
                sender.output(AppMsg::Close);
            } // LoadErrorMsg::Preferences => sender.output(AppMsg::ShowPrefMenu),
        }
    }
}
