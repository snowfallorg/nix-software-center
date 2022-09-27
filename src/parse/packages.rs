use flate2::bufread::GzDecoder;
use ijson::IString;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::{self, fs::File, collections::HashMap, error::Error, env, io::BufReader};
use log::*;

use crate::APPINFO;

#[derive(Serialize, Deserialize, Debug)]
pub struct PackageBase {
    packages: HashMap<String, Package>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Package {
    pub system: IString,
    pub pname: IString,
    pub meta: Meta,
    pub version: IString,
    #[serde(skip_deserializing)]
    pub appdata: Option<AppData>,
}
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Meta {
    pub broken: Option<bool>,
    pub insecure: Option<bool>,
    pub unsupported: Option<bool>,
    pub unfree: Option<bool>,
    pub description: Option<IString>,
    #[serde(rename = "longDescription")]
    pub longdescription: Option<IString>,
    pub homepage: Option<StrOrVec>,
    pub maintainers: Option<Vec<PkgMaintainer>>,
    pub position: Option<IString>,
    pub license: Option<LicenseEnum>,
    pub platforms: Option<Platform>
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(untagged)]
pub enum StrOrVec {
    Single(IString),
    List(Vec<IString>),
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(untagged)]
pub enum Platform {
    Single(IString),
    List(Vec<IString>),
    ListList(Vec<Vec<IString>>),
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(untagged)]
pub enum LicenseEnum {
    Single(License),
    List(Vec<License>),
    SingleStr(IString),
    VecStr(Vec<IString>),
    Mixed(Vec<LicenseEnum>)
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct License {
    pub free: Option<bool>,
    #[serde(rename = "fullName")]
    pub fullname: Option<IString>,
    #[serde(rename = "spdxId")]
    pub spdxid: Option<IString>,
    pub url: Option<IString>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct PkgMaintainer {
    pub email: IString,
    pub github: IString,
    pub matrix: Option<IString>,
    pub name: Option<IString>
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct AppData {
    #[serde(rename = "Type")]
    pub metatype: IString,
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Package")]
    pub package: String,
    #[serde(rename = "Name")]
    pub name: Option<HashMap<String, String>>,
    #[serde(rename = "Description")]
    pub description: Option<HashMap<String, String>>,
    #[serde(rename = "Summary")]
    pub summary: Option<HashMap<String, String>>,
    #[serde(rename = "Url")]
    pub url: Option<AppUrl>,
    #[serde(rename = "Icon")]
    pub icon: Option<AppIconList>,
    #[serde(rename = "Launchable")]
    pub launchable: Option<AppLaunchable>,
    #[serde(rename = "Provides")]
    pub provides: Option<AppProvides>,
    #[serde(rename = "Screenshots")]
    pub screenshots: Option<Vec<AppScreenshot>>,
    #[serde(rename = "Categories")]
    pub categories: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AppUrl {
    pub homepage: Option<String>,
    pub bugtracker: Option<String>,
    pub help: Option<String>,
    pub donation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AppIconList {
    pub cached: Option<Vec<AppIcon>>,
    pub stock: Option<String>,
    // TODO: add support for other icon types
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct AppIcon {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AppLaunchable {
    #[serde(rename = "desktop-id")]
    pub desktopid: Vec<String>
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AppProvides {
    pub binaries: Option<Vec<String>>,
    pub ids: Option<Vec<String>>,
    pub mediatypes: Option<Vec<String>>,
    pub libraries: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AppScreenshot {
    pub default: Option<bool>,
    pub thumbnails: Option<Vec<String>>,
    #[serde(rename = "source-image")]
    pub sourceimage: Option<AppScreenshotImage>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AppScreenshotImage {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FlakeJson {
    pname: IString,
    version: IString,
}

pub async fn readpkgs() -> Result<HashMap<String, Package>,  Box<dyn Error + Send + Sync>> {
    info!("Reading package list");
    let cachedir = format!("{}/.cache/nix-software-center/", env::var("HOME")?);
    let cachefile = format!("{}/packages.json", cachedir);
    let file = File::open(cachefile)?;
    let reader = BufReader::new(file);
    trace!("Reading packages.json");
    let pkgbase: PackageBase = simd_json::serde::from_reader(reader)?;
    trace!("Finished reading packages.json");
    let mut pkgs = pkgbase.packages;
    debug!("APPDATADIR {}", APPINFO);
    let appdata = File::open(&format!("{}/xmls/nixos_x86_64_linux.yml.gz", APPINFO))?;
    let appreader = BufReader::new(appdata);
    let mut d = GzDecoder::new(appreader);
    let mut s = String::new();
    d.read_to_string(&mut s)?;
    let mut files = s.split("\n---\n").collect::<Vec<_>>();
    files.remove(0);
    for f in files {
        if let Ok(appstream) = serde_yaml::from_str::<AppData>(f) {
            if let Some(p) = pkgs.get_mut(&appstream.package.to_string()) {
                p.appdata = Some(appstream);
            }
        } else {
            warn!("Failed to parse some appstream data");
        }
    }
    Ok(pkgs)
}

pub fn readlegacysyspkgs() -> Result<HashMap<String, String>,  Box<dyn Error + Send + Sync>> {
    info!("Reading legacy system package list");
    let cachedir = format!("{}/.cache/nix-software-center/", env::var("HOME")?);
    let cachefile = format!("{}/syspackages.json", cachedir);
    if let Ok(f) = fs::read_to_string(&cachefile) {
        if f.trim().is_empty() {
            return Ok(HashMap::new());
        }
    }
    let file = File::open(cachefile)?;
    let reader = BufReader::new(file);
    let newpkgs: HashMap<String, String> = simd_json::serde::from_reader(reader)?;
    Ok(newpkgs)
}

pub fn readflakesyspkgs() -> Result<HashMap<String, String>,  Box<dyn Error + Send + Sync>> {
    info!("Reading flake system package list");
    let cachedir = format!("{}/.cache/nix-software-center/", env::var("HOME")?);
    let cachefile = format!("{}/syspackages.json", cachedir);
    if let Ok(f) = fs::read_to_string(&cachefile) {
        if f.trim().is_empty() {
            return Ok(HashMap::new());
        }
    }
    let file = File::open(cachefile)?;
    let reader = BufReader::new(file);
    let newpkgs: HashMap<String, FlakeJson> = simd_json::serde::from_reader(reader)?;
    let newpkgs = newpkgs.into_iter().filter_map(|(k, v)| if let Some(pkg) = k.strip_prefix("legacyPackages.x86_64-linux.") { Some((pkg.to_string(), v.version.to_string())) } else { None }).collect::<HashMap<_, _>>();
    Ok(newpkgs)
}

pub fn readprofilepkgs() -> Result<HashMap<String, String>,  Box<dyn Error + Send + Sync>> {
    info!("Reading profile package list");
    let cachedir = format!("{}/.cache/nix-software-center/", env::var("HOME")?);
    let cachefile = format!("{}/profilepackages.json", cachedir);
    if let Ok(f) = fs::read_to_string(&cachefile) {
        if f.trim().is_empty() {
            return Ok(HashMap::new());
        }
    }
    let file = File::open(cachefile)?;
    let reader = BufReader::new(file);
    let profilepkgs: HashMap<String, FlakeJson> = simd_json::serde::from_reader(reader)?;
    let profilepkgs = profilepkgs.into_iter().filter_map(|(k, v)| if let Some(pkg) = k.strip_prefix("legacyPackages.x86_64-linux.") { Some((pkg.to_string(), v.version.to_string())) } else { None }).collect::<HashMap<_, _>>();
    Ok(profilepkgs)
}