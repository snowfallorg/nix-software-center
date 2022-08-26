use ijson::IString;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    error::Error,
    fs::{self, File},
    io::{BufReader, Read, Write},
    path::Path,
    process::Command,
};

#[derive(Serialize, Deserialize, Debug)]
struct NewPackageBase {
    packages: HashMap<String, NewPackage>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct NewPackage {
    version: IString,
}

pub fn checkcache() -> Result<(), Box<dyn Error>> {
    setuppkgscache()?;
    setupupdatecache()?;
    setupnewestver()?;
    Ok(())
}

pub fn uptodate() -> Result<Option<(String, String)>, Box<dyn Error>> {
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
        println!("OLD {:?} != NEW {:?}", oldversion, newversion);
        Ok(Some((oldversion, newversion)))
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

fn setuppkgscache() -> Result<(), Box<dyn Error>> {
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
    let pkgbase: NewPackageBase = simd_json::serde::from_reader(reader).unwrap();
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
