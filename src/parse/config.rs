use std::{error::Error, env, path::Path, fs::{self, File}, io::Write};

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct NscConfig {
    pub systemconfig: String,
    pub flake: Option<String>,
}

pub fn getconfig() -> NscConfig {
    if let Ok(c) = getconfigval() {
        c
    } else {
        NscConfig {
            systemconfig: String::from("/etc/nixos/configuration.nix"),
            flake: None,
        }
    }
}

fn getconfigval() -> Result<NscConfig, Box<dyn Error>> {
    let configfile = checkconfig()?;
    let config: NscConfig = serde_json::from_reader(File::open(format!("{}/config.json", configfile))?)?;
    Ok(config)
}

fn checkconfig() -> Result<String, Box<dyn Error>> {
    let cfgdir = format!("{}/.config/nix-software-center", env::var("HOME")?);
    if !Path::is_file(Path::new(&format!("{}/config.json", &cfgdir))) {
        if !Path::is_file(Path::new("/etc/nix-software-center/config.json")) {
            createdefaultconfig()?;
            Ok(cfgdir)
        } else {
            Ok("/etc/nix-software-center/".to_string())
        }
    } else {
        Ok(cfgdir)
    }
}

// fn configexists() -> Result<bool, Box<dyn Error>> {
//     let cfgdir = format!("{}/.config/nix-software-center", env::var("HOME")?);
//     if !Path::is_file(Path::new(&format!("{}/config.json", &cfgdir))) {
//         if !Path::is_file(Path::new("/etc/nix-software-center/config.json")) {
//             Ok(false)
//         } else {
//             Ok(true)
//         }
//     } else {
//         Ok(true)
//     }
// }

pub fn editconfig(config: NscConfig) -> Result<(), Box<dyn Error>> {
    let cfgdir = format!("{}/.config/nix-software-center", env::var("HOME")?);
    fs::create_dir_all(&cfgdir)?;
    let json = serde_json::to_string_pretty(&config)?;
    let mut file = File::create(format!("{}/config.json", cfgdir))?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

fn createdefaultconfig() -> Result<(), Box<dyn Error>> {
    let cfgdir = format!("{}/.config/nix-software-center", env::var("HOME")?);
    fs::create_dir_all(&cfgdir)?;
    let config = NscConfig {
        systemconfig: "/etc/nixos/configuration.nix".to_string(),
        flake: None,
    };
    let json = serde_json::to_string_pretty(&config)?;
    let mut file = File::create(format!("{}/config.json", cfgdir))?;
    file.write_all(json.as_bytes())?;
    Ok(())
}


// pub fn readconfig(cfg: String) -> Result<(String, Option<String>), Box<dyn Error>> {
//     let file = fs::read_to_string(cfg)?;
//     let config: NscConfig = match serde_json::from_str(&file) {
//         Ok(x) => x,
//         Err(_) => {
//             createdefaultconfig()?;
//             return Ok((
//                 "/etc/nixos/configuration.nix".to_string(),
//                 None,
//             ));
//         }
//     };
//     if Path::is_file(Path::new(&config.systemconfig)) {
//         Ok((config.systemconfig, config.flake))
//     } else {
//         Ok((
//             "/etc/nixos/configuration.nix".to_string(),
//             None,
//         ))
//     }
// }
