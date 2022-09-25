use std::{
    env,
    error::Error,
    fs::{self, File},
    io::Write,
    path::Path,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct NscConfig {
    pub systemconfig: Option<String>,
    pub flake: Option<String>,
    pub flakearg: Option<String>,
}

pub fn getconfig() -> Option<NscConfig> {
    if let Ok(c) = getconfigval() {
        Some(c)
    } else {
        None
    }
}

fn getconfigval() -> Result<NscConfig, Box<dyn Error>> {
    let configfile = checkconfig()?;
    let config: NscConfig =
        serde_json::from_reader(File::open(format!("{}/config.json", configfile))?)?;
    Ok(config)
}

fn checkconfig() -> Result<String, Box<dyn Error>> {
    let cfgdir = format!("{}/.config/nix-software-center", env::var("HOME")?);
    if !Path::is_file(Path::new(&format!("{}/config.json", &cfgdir))) {
        if !Path::is_file(Path::new("/etc/nix-software-center/config.json")) {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "No config file found",
            )))
        } else {
            Ok("/etc/nix-software-center/".to_string())
        }
    } else {
        Ok(cfgdir)
    }
}

pub fn editconfig(config: NscConfig) -> Result<(), Box<dyn Error>> {
    let cfgdir = format!("{}/.config/nix-software-center", env::var("HOME")?);
    fs::create_dir_all(&cfgdir)?;
    let json = serde_json::to_string_pretty(&config)?;
    let mut file = File::create(format!("{}/config.json", cfgdir))?;
    file.write_all(json.as_bytes())?;
    Ok(())
}