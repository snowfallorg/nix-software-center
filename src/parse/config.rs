use anyhow::Result;
use nix_data::config::configfile::NixDataConfig;

pub fn getconfig() -> Option<NixDataConfig> {
    if let Ok(c) = nix_data::config::configfile::getconfig() {
        Some(c)
    } else {
        None
    }
}

pub fn editconfig(config: NixDataConfig) -> Result<()> {
    nix_data::config::configfile::setuserconfig(config)?;
    Ok(())
}
