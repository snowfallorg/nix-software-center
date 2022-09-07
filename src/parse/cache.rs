use ijson::IString;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    env,
    error::Error,
    fs::{self, File},
    io::{BufReader, Read, Write},
    path::Path,
    process::Command,
};

use crate::ui::window::{SystemPkgs, UserPkgs};

#[derive(Serialize, Deserialize, Debug)]
struct NewPackageBase {
    packages: HashMap<String, NewPackage>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct NewPackage {
    version: IString,
}

pub fn checkcache(syspkgs: SystemPkgs, userpkgs: UserPkgs) -> Result<(), Box<dyn Error>> {
    match syspkgs {
        SystemPkgs::Legacy => {
            setuplegacypkgscache().unwrap();
            setupupdatecache().unwrap();
            setupnewestver().unwrap();
        }
        SystemPkgs::Flake => {
            setupflakepkgscache()?;
        }
    }
    if userpkgs == UserPkgs::Profile {
        setupprofilepkgscache()?;
    }
    Ok(())
}

pub fn uptodatelegacy() -> Result<Option<(String, String)>, Box<dyn Error>> {
    let cachedir = format!("{}/.cache/nix-software-center", env::var("HOME")?);
    let oldversion = fs::read_to_string(format!("{}/sysver.txt", cachedir))?
        .trim()
        .to_string();
    let newversion = fs::read_to_string(format!("{}/chnver.txt", cachedir))?
        .trim()
        .to_string();
    if oldversion == newversion {
        println!("System is up to date");
        Ok(None)
    } else {
        Ok(Some((oldversion, newversion)))
    }
}

pub fn uptodateflake() -> Result<Option<(String, String)>, Box<dyn Error>> {
    let cachedir = format!("{}/.cache/nix-software-center", env::var("HOME")?);
    let oldversion = fs::read_to_string(format!("{}/flakever.txt", cachedir))?
        .trim()
        .to_string();
    let newversion = fs::read_to_string(format!("{}/newver.txt", cachedir))?
        .trim()
        .to_string();
    if oldversion == newversion {
        println!("System is up to date");
        Ok(None)
    } else {
        println!("OLD {:?} != NEW {:?}", oldversion, newversion);
        if let (Some(oldv), Some(newv)) = (oldversion.get(..8), newversion.get(..8)) {
            println!("OLD {:?} != NEW {:?}", oldv, newv);
            Ok(Some((oldv.to_string(), newv.to_string())))
        } else {
            Ok(Some((oldversion, newversion)))
        }
    }
}

pub fn channelver() -> Result<Option<(String, String)>, Box<dyn Error>> {
    let cachedir = format!("{}/.cache/nix-software-center", env::var("HOME")?);
    let oldversion = fs::read_to_string(format!("{}/chnver.txt", cachedir))?
        .trim()
        .to_string();
    let newversion = fs::read_to_string(format!("{}/newver.txt", cachedir))?
        .trim()
        .to_string();
    if oldversion == newversion {
        println!("Channels match");
        Ok(None)
    } else {
        println!("chnver {:?} != newver {:?}", oldversion, newversion);
        Ok(Some((oldversion, newversion)))
    }
}

pub fn flakever() -> Result<Option<(String, String)>, Box<dyn Error>> {
    let cachedir = format!("{}/.cache/nix-software-center", env::var("HOME")?);
    let oldversion = fs::read_to_string(format!("{}/flakever.txt", cachedir))?
        .trim()
        .to_string();
    let newversion = fs::read_to_string(format!("{}/newver.txt", cachedir))?
        .trim()
        .to_string();
    if oldversion == newversion {
        println!("Flake hashes match");
        Ok(None)
    } else {
        println!("flakever {:?} != newver {:?}", oldversion, newversion);
        Ok(Some((oldversion, newversion)))
    }
}

fn setuplegacypkgscache() -> Result<(), Box<dyn Error>> {
    let vout = Command::new("nix-instantiate")
        .arg("-I")
        .arg("nixpkgs=/nix/var/nix/profiles/per-user/root/channels/nixos")
        .arg("<nixpkgs/lib>")
        .arg("-A")
        .arg("version")
        .arg("--eval")
        .arg("--json")
        .output()?;

    let dlver = String::from_utf8_lossy(&vout.stdout)
        .to_string()
        .replace('"', "");

    let mut relver = dlver.split('.').collect::<Vec<&str>>().join(".")[0..5].to_string();

    if dlver.len() >= 8 && &dlver[5..8] == "pre" {
        relver = "unstable".to_string();
    }

    let cachedir = format!("{}/.cache/nix-software-center", env::var("HOME")?);
    fs::create_dir_all(&cachedir).expect("Failed to create cache directory");
    let url = format!(
        "https://releases.nixos.org/nixos/{}/nixos-{}/packages.json.br",
        relver, dlver
    );

    println!("VERSION {}", relver);
    // let response = reqwest::blocking::get(url)?;
    // if let Some(latest) = response.url().to_string().split('/').last() {
    let cachedir = format!("{}/.cache/nix-software-center", env::var("HOME")?);
    if !Path::new(&cachedir).exists() {
        fs::create_dir_all(&cachedir).expect("Failed to create cache directory");
    }

    if !Path::new(&format!("{}/chnver.txt", &cachedir)).exists() {
        let mut sysver = fs::File::create(format!("{}/chnver.txt", &cachedir))?;
        sysver.write_all(dlver.as_bytes())?;
    }

    if Path::new(format!("{}/chnver.txt", &cachedir).as_str()).exists()
        && fs::read_to_string(&Path::new(format!("{}/chnver.txt", &cachedir).as_str()))? == dlver
        && Path::new(format!("{}/packages.json", &cachedir).as_str()).exists()
    {
        return Ok(());
    } else {
        let oldver = fs::read_to_string(&Path::new(format!("{}/chnver.txt", &cachedir).as_str()))?;
        let sysver = &dlver;
        // Change to debug msg
        println!("OLD: {}, != NEW: {}", oldver, sysver);
    }
    if Path::new(format!("{}/chnver.txt", &cachedir).as_str()).exists() {
        fs::remove_file(format!("{}/chnver.txt", &cachedir).as_str())?;
    }
    let mut sysver = fs::File::create(format!("{}/chnver.txt", &cachedir))?;
    sysver.write_all(dlver.as_bytes())?;
    let outfile = format!("{}/packages.json", &cachedir);
    dlfile(&url, &outfile)?;
    // }
    Ok(())
}

fn setupflakepkgscache() -> Result<(), Box<dyn Error>> {
    println!("SETUP FLAKES CACHE");
    let cachedir = format!("{}/.cache/nix-software-center", env::var("HOME")?);

    // First remove legacy files
    if Path::new(format!("{}/chnver.txt", &cachedir).as_str()).exists() {
        fs::remove_file(format!("{}/chnver.txt", &cachedir).as_str())?;
    }

    let vout = Command::new("nixos-version").arg("--json").output()?;

    let versiondata: Value = serde_json::from_str(&String::from_utf8_lossy(&vout.stdout))?;
    let rev = versiondata
        .get("nixpkgsRevision")
        .unwrap()
        .as_str()
        .unwrap();
    let dlver = versiondata.get("nixosVersion").unwrap().as_str().unwrap();

    let mut relver = dlver.split('.').collect::<Vec<&str>>()[0..2].join(".");
    println!("RELVER {}", relver);
    if relver == "22.11" {
        relver = "unstable".to_string();
    }

    fs::create_dir_all(&cachedir).expect("Failed to create cache directory");
    let url = format!(
        "https://channels.nixos.org/nixos-{}/packages.json.br",
        relver
    );

    println!("VERSION {}", relver);
    // let response = reqwest::blocking::get(url)?;
    // if let Some(latest) = response.url().to_string().split('/').last() {
    let cachedir = format!("{}/.cache/nix-software-center", env::var("HOME")?);
    if !Path::new(&cachedir).exists() {
        fs::create_dir_all(&cachedir).expect("Failed to create cache directory");
    }

    fn writesyspkgs(outfile: &str) -> Result<(), Box<dyn Error>> {
        let output = Command::new("nix")
            .arg("search")
            .arg("--inputs-from")
            .arg("/home/victor/nix")
            .arg("nixpkgs")
            .arg("--json")
            .output()?;
        let mut file = fs::File::create(outfile)?;
        file.write_all(&output.stdout)?;
        Ok(())
    }

    if !Path::new(&format!("{}/flakever.txt", &cachedir)).exists() {
        let mut sysver = fs::File::create(format!("{}/flakever.txt", &cachedir))?;
        sysver.write_all(rev.as_bytes())?;
        writesyspkgs(&format!("{}/syspackages.json", &cachedir))?;
    } else {
        let oldver =
            fs::read_to_string(&Path::new(format!("{}/flakever.txt", &cachedir).as_str()))?;
        let sysver = rev;
        if oldver != sysver {
            println!("OLD FLAKEVER: {}, != NEW: {}", oldver, sysver);
            let mut sysver = fs::File::create(format!("{}/flakever.txt", &cachedir))?;
            sysver.write_all(rev.as_bytes())?;
            writesyspkgs(&format!("{}/syspackages.json", &cachedir))?;
        }
    }

    if !Path::new(&format!("{}/syspackages.json", &cachedir)).exists() {
        writesyspkgs(&format!("{}/syspackages.json", &cachedir))?;
    }

    // Check newest nixpkgs version
    let revurl = format!("https://channels.nixos.org/nixos-{}/git-revision", relver);
    let response = reqwest::blocking::get(revurl)?;
    let mut dl = false;
    if response.status().is_success() {
        let newrev = response.text()?;
        println!("NEW REV: {}", newrev);
        if Path::new(&format!("{}/newver.txt", &cachedir)).exists() {
            let oldrev = fs::read_to_string(&format!("{}/newver.txt", &cachedir))?;
            if oldrev != newrev {
                dl = true;
            }
        } else {
            dl = true;
        }
        let mut sysver = fs::File::create(format!("{}/newver.txt", &cachedir))?;
        sysver.write_all(newrev.as_bytes())?;
    }

    let outfile = format!("{}/packages.json", &cachedir);
    if dl {
        dlfile(&url, &outfile)?;
    }
    Ok(())
}

fn setupprofilepkgscache() -> Result<(), Box<dyn Error>> {
    let cachedir = format!("{}/.cache/nix-software-center", env::var("HOME")?);
    fn writeprofilepkgs(outfile: &str) -> Result<(), Box<dyn Error>> {
        let output = Command::new("nix")
            .arg("search")
            .arg("nixpkgs")
            .arg("--json")
            .output()?;
        let mut file = fs::File::create(outfile)?;
        file.write_all(&output.stdout)?;
        Ok(())
    }
    // Check nix profile nixpkgs version
    let client = reqwest::blocking::Client::builder()
        .user_agent("request")
        .build()?;
    let response = client
        .get("https://api.github.com/repos/NixOS/nixpkgs/commits/nixpkgs-unstable")
        .send()?;
    if response.status().is_success() {
        let profilerevjson = response.text()?;
        let profilerevdata: Value = serde_json::from_str(&profilerevjson)?;
        let profilerev = profilerevdata.get("sha").unwrap().as_str().unwrap();
        println!("PROFILE REV {}", profilerev);

        if !Path::new(&format!("{}/profilever.txt", &cachedir)).exists() {
            let mut sysver = fs::File::create(format!("{}/profilever.txt", &cachedir))?;
            sysver.write_all(profilerev.as_bytes())?;
            writeprofilepkgs(&format!("{}/profilepackages.json", &cachedir))?;
        } else {
            let oldver =
                fs::read_to_string(&Path::new(format!("{}/profilever.txt", &cachedir).as_str()))?;
            let sysver = profilerev;
            if oldver != sysver {
                println!("OLD PROFILEVER: {}, != NEW: {}", oldver, sysver);
                let mut sysver = fs::File::create(format!("{}/profilever.txt", &cachedir))?;
                sysver.write_all(profilerev.as_bytes())?;
                writeprofilepkgs(&format!("{}/profilepackages.json", &cachedir))?;
            } else {
                println!("PROFILEVER UP TO DATE");
            }
        }
    }
    if !Path::new(&format!("{}/profilepackages.json", &cachedir)).exists() {
        writeprofilepkgs(&format!("{}/profilepackages.json", &cachedir))?;
    }
    Ok(())
}

// nix-instantiate --eval -E '(builtins.getFlake "/home/victor/nix").inputs.nixpkgs.outPath'
// nix-env -f /nix/store/sjmq1gphj1arbzf4aqqnygd9pf4hkfkf-source -qa --json > packages.json
fn setupupdatecache() -> Result<(), Box<dyn Error>> {
    let dlver = fs::read_to_string("/run/current-system/nixos-version")?;

    let mut relver = dlver.split('.').collect::<Vec<&str>>().join(".")[0..5].to_string();

    if dlver.len() >= 8 && &dlver[5..8] == "pre" {
        relver = "unstable".to_string();
    }

    let cachedir = format!("{}/.cache/nix-software-center", env::var("HOME")?);
    fs::create_dir_all(&cachedir).expect("Failed to create cache directory");
    let url = format!(
        "https://releases.nixos.org/nixos/{}/nixos-{}/packages.json.br",
        relver, dlver
    );

    println!("VERSION {}", relver);
    let cachedir = format!("{}/.cache/nix-software-center", env::var("HOME")?);
    if !Path::new(&cachedir).exists() {
        fs::create_dir_all(&cachedir).expect("Failed to create cache directory");
    }

    if !Path::new(&format!("{}/sysver.txt", &cachedir)).exists() {
        let mut sysver = fs::File::create(format!("{}/sysver.txt", &cachedir))?;
        sysver.write_all(dlver.as_bytes())?;
    }

    if Path::new(format!("{}/sysver.txt", &cachedir).as_str()).exists()
        && fs::read_to_string(&Path::new(format!("{}/sysver.txt", &cachedir).as_str()))? == dlver
        && Path::new(format!("{}/syspackages.json", &cachedir).as_str()).exists()
    {
        return Ok(());
    } else {
        let oldver = fs::read_to_string(&Path::new(format!("{}/sysver.txt", &cachedir).as_str()))?;
        let sysver = &dlver;
        // Change to debug msg
        println!("OLD: {}, != NEW: {}", oldver, sysver);
    }
    if Path::new(format!("{}/sysver.txt", &cachedir).as_str()).exists() {
        fs::remove_file(format!("{}/sysver.txt", &cachedir).as_str())?;
    }
    let mut sysver = fs::File::create(format!("{}/sysver.txt", &cachedir))?;
    sysver.write_all(dlver.as_bytes())?;
    let outfile = format!("{}/syspackages.json", &cachedir);
    dlfile(&url, &outfile)?;
    let file = File::open(&outfile)?;
    let reader = BufReader::new(file);
    let pkgbase: NewPackageBase = simd_json::serde::from_reader(reader)?;
    let mut outbase = HashMap::new();
    for (pkg, ver) in pkgbase.packages {
        outbase.insert(pkg.clone(), ver.version.clone());
    }
    let out = simd_json::serde::to_string(&outbase)?;
    fs::write(&outfile, out)?;
    Ok(())
}

fn setupnewestver() -> Result<(), Box<dyn Error>> {
    let version = fs::read_to_string("/run/current-system/nixos-version")?;

    let mut relver = version.split('.').collect::<Vec<&str>>().join(".")[0..5].to_string();

    if version.len() >= 8 && &version[5..8] == "pre" {
        relver = "unstable".to_string();
    }
    println!("VERSION {}", relver);
    let response = reqwest::blocking::get(format!("https://channels.nixos.org/nixos-{}", relver))?;
    if let Some(latest) = response.url().to_string().split('/').last() {
        let latest = latest.strip_prefix("nixos-").unwrap_or(latest);
        let cachedir = format!("{}/.cache/nix-software-center", env::var("HOME")?);
        if !Path::new(&cachedir).exists() {
            fs::create_dir_all(&cachedir).expect("Failed to create cache directory");
        }

        if !Path::new(format!("{}/newver.txt", &cachedir).as_str()).exists() {
            let mut newver = fs::File::create(format!("{}/newver.txt", &cachedir))?;
            newver.write_all(latest.as_bytes())?;
        }

        if Path::new(format!("{}/newver.txt", &cachedir).as_str()).exists()
            && fs::read_to_string(&Path::new(format!("{}/newver.txt", &cachedir).as_str()))?
                == latest
        {
            return Ok(());
        } else {
            let oldver =
                fs::read_to_string(&Path::new(format!("{}/newver.txt", &cachedir).as_str()))?;
            let newver = latest;
            // Change to debug msg
            println!("OLD: {}, != NEW: {}", oldver, newver);
        }
        if Path::new(format!("{}/newver.txt", &cachedir).as_str()).exists() {
            fs::remove_file(format!("{}/newver.txt", &cachedir).as_str())?;
        }
        let mut newver = fs::File::create(format!("{}/newver.txt", &cachedir))?;
        newver.write_all(latest.as_bytes())?;
    }
    Ok(())
}

fn dlfile(url: &str, path: &str) -> Result<(), Box<dyn Error>> {
    println!("Downloading {}", url);
    let response = reqwest::blocking::get(url)?;
    if response.status().is_success() {
        let cachedir = format!("{}/.cache/nix-software-center", env::var("HOME")?);
        if !Path::new(&cachedir).exists() {
            fs::create_dir_all(&cachedir).expect("Failed to create cache directory");
        }

        let dst: Vec<u8> = response.bytes()?.to_vec();
        {
            let mut file = File::create(path)?;
            let mut reader = brotli::Decompressor::new(
                dst.as_slice(),
                4096, // buffer size
            );
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf[..]) {
                    Err(e) => {
                        if let std::io::ErrorKind::Interrupted = e.kind() {
                            continue;
                        }
                        return Err(Box::new(e));
                    }
                    Ok(size) => {
                        if size == 0 {
                            break;
                        }
                        file.write_all(&buf[..size])?
                    }
                }
            }
        }
    }
    Ok(())
}
