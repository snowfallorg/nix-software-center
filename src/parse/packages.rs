use flate2::bufread::GzDecoder;
use serde::{Deserialize, Serialize};
use std::{self, fs::File, collections::HashMap, io::{BufReader, Read}};
use log::*;
use anyhow::Result;

use crate::APPINFO;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(untagged)]
pub enum StrOrVec {
    Single(String),
    List(Vec<String>),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(untagged)]
pub enum Platform {
    Single(String),
    List(Vec<String>),
    ListList(Vec<Vec<String>>),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(untagged)]
pub enum LicenseEnum {
    Single(License),
    List(Vec<License>),
    SingleStr(String),
    VecStr(Vec<String>),
    Mixed(Vec<LicenseEnum>)
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct License {
    pub free: Option<bool>,
    #[serde(rename = "fullName")]
    pub fullname: Option<String>,
    #[serde(rename = "spdxId")]
    pub spdxid: Option<String>,
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct PkgMaintainer {
    pub email: Option<String>,
    pub github: Option<String>,
    pub matrix: Option<String>,
    pub name: Option<String>
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct AppData {
    #[serde(rename = "Type")]
    pub metatype: String,
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AppUrl {
    pub homepage: Option<String>,
    pub bugtracker: Option<String>,
    pub help: Option<String>,
    pub donation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AppIconList {
    pub cached: Option<Vec<AppIcon>>,
    pub stock: Option<String>,
    // TODO: add support for other icon types
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct AppIcon {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AppLaunchable {
    #[serde(rename = "desktop-id")]
    pub desktopid: Vec<String>
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AppProvides {
    pub binaries: Option<Vec<String>>,
    pub ids: Option<Vec<String>>,
    pub mediatypes: Option<Vec<String>>,
    pub libraries: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AppScreenshot {
    pub default: Option<bool>,
    pub thumbnails: Option<Vec<String>>,
    #[serde(rename = "source-image")]
    pub sourceimage: Option<AppScreenshotImage>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AppScreenshotImage {
    pub url: String,
}

pub fn appsteamdata() ->  Result<HashMap<String, AppData>> {
    let appdata = File::open(&format!("{}/xmls/nixos_x86_64_linux.yml.gz", APPINFO))?;
    let appreader = BufReader::new(appdata);
    let mut d = GzDecoder::new(appreader);
    let mut s = String::new();
    d.read_to_string(&mut s)?;
    let mut files = s.split("\n---\n").collect::<Vec<_>>();
    files.remove(0);

    let mut out = HashMap::new();

    for f in files {
        if let Ok(appstream) = serde_yaml::from_str::<AppData>(f) {
            out.insert(appstream.package.to_string(), appstream);
        } else {
            warn!("Failed to parse some appstream data");
        }
    }
    Ok(out)
}
