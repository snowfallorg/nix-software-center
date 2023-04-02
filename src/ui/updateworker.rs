use anyhow::{anyhow, Result};
use log::*;
use nix_data::config::configfile::NixDataConfig;
use relm4::*;
use std::{fs, path::Path, process::Stdio};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::ui::{rebuild::RebuildMsg, window::REBUILD_BROKER};

use super::{
    updatepage::UpdatePageMsg,
    window::{SystemPkgs, UserPkgs},
};

#[tracker::track]
#[derive(Debug)]
pub struct UpdateAsyncHandler {
    #[tracker::no_eq]
    process: Option<JoinHandle<()>>,
    config: NixDataConfig,
    syspkgs: SystemPkgs,
    userpkgs: UserPkgs,
}

#[derive(Debug)]
pub enum UpdateAsyncHandlerMsg {
    UpdateConfig(NixDataConfig),
    UpdatePkgTypes(SystemPkgs, UserPkgs),

    // UpdateChannels,
    // UpdateChannelsAndSystem,
    UpdateSystem,
    UpdateSystemRemove(Vec<String>),

    RebuildSystem,
    UpdateUserPkgs,
    UpdateUserPkgsRemove(Vec<String>),

    UpdateAll,
    UpdateAllRemove(Vec<String>, Vec<String>),
}

enum NscCmd {
    Rebuild,
    Channel,
    All,
}

pub struct UpdateAsyncHandlerInit {
    pub syspkgs: SystemPkgs,
    pub userpkgs: UserPkgs,
}

impl Worker for UpdateAsyncHandler {
    type Init = UpdateAsyncHandlerInit;
    type Input = UpdateAsyncHandlerMsg;
    type Output = UpdatePageMsg;

    fn init(params: Self::Init, _sender: relm4::ComponentSender<Self>) -> Self {
        Self {
            process: None,
            config: NixDataConfig {
                systemconfig: None,
                flake: None,
                flakearg: None,
                generations: None
            },
            syspkgs: params.syspkgs,
            userpkgs: params.userpkgs,
            tracker: 0,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            UpdateAsyncHandlerMsg::UpdateConfig(config) => {
                self.config = config;
            }
            UpdateAsyncHandlerMsg::UpdatePkgTypes(syspkgs, userpkgs) => {
                self.syspkgs = syspkgs;
                self.userpkgs = userpkgs;
            }
            UpdateAsyncHandlerMsg::UpdateSystem => {
                let config = self.config.clone();
                let syspkgs = self.syspkgs.clone();
                relm4::spawn(async move {
                    let result = runcmd(NscCmd::All, config, syspkgs, None).await;
                    match result {
                        Ok(true) => {
                            sender.output(UpdatePageMsg::DoneWorking);
                        }
                        _ => {
                            warn!("UPDATE SYSTEM FAILED");
                            sender.output(UpdatePageMsg::FailedWorking);
                        }
                    }
                });
            }
            UpdateAsyncHandlerMsg::UpdateSystemRemove(pkgs) => {
                let config = self.config.clone();
                let syspkgs = self.syspkgs.clone();
                relm4::spawn(async move {
                    let result =
                        runcmd(NscCmd::All, config, syspkgs, Some(pkgs)).await;
                    match result {
                        Ok(true) => {
                            sender.output(UpdatePageMsg::DoneWorking);
                        }
                        _ => {
                            warn!("UPDATE SYSTEM FAILED");
                            sender.output(UpdatePageMsg::FailedWorking);
                        }
                    }
                });
            }
            UpdateAsyncHandlerMsg::RebuildSystem => {
                let config = self.config.clone();
                let syspkgs = self.syspkgs.clone();
                relm4::spawn(async move {
                    let result = match syspkgs {
                        SystemPkgs::Legacy => {
                            runcmd(NscCmd::Rebuild, config, syspkgs, None).await
                        }
                        SystemPkgs::Flake => {
                            runcmd(NscCmd::All, config, syspkgs, None).await
                        }
                        SystemPkgs::None => Ok(true),
                    };
                    match result {
                        Ok(true) => {
                            sender.output(UpdatePageMsg::DoneWorking);
                        }
                        _ => {
                            warn!("REBUILD FAILED");
                            sender.output(UpdatePageMsg::FailedWorking);
                        }
                    }
                });
            }
            UpdateAsyncHandlerMsg::UpdateUserPkgs => {
                let userpkgs = self.userpkgs.clone();
                relm4::spawn(async move {
                    let result = match userpkgs {
                        UserPkgs::Env => updateenv().await,
                        UserPkgs::Profile => updateprofile(None).await,
                    };
                    match result {
                        Ok(true) => {
                            sender.output(UpdatePageMsg::DoneWorking);
                        }
                        _ => {
                            warn!("UPDATE USER FAILED");
                            sender.output(UpdatePageMsg::FailedWorking);
                        }
                    }
                });
            }
            UpdateAsyncHandlerMsg::UpdateUserPkgsRemove(pkgs) => {
                let userpkgs = self.userpkgs.clone();
                relm4::spawn(async move {
                    let result = match userpkgs {
                        UserPkgs::Env => updateenv().await,
                        UserPkgs::Profile => updateprofile(Some(pkgs)).await,
                    };
                    match result {
                        Ok(true) => {
                            sender.output(UpdatePageMsg::DoneWorking);
                        }
                        _ => {
                            warn!("UPDATE USER FAILED");
                            sender.output(UpdatePageMsg::FailedWorking);
                        }
                    }
                });
            }
            UpdateAsyncHandlerMsg::UpdateAll => {
                let config = self.config.clone();
                let syspkgs = self.syspkgs.clone();
                let userpkgs = self.userpkgs.clone();
                relm4::spawn(async move {
                    let result = runcmd(NscCmd::All, config, syspkgs, None).await;
                    match result {
                        Ok(true) => {
                            match match userpkgs {
                                UserPkgs::Env => updateenv().await,
                                UserPkgs::Profile => updateprofile(None).await,
                            } {
                                Ok(true) => {
                                    sender.output(UpdatePageMsg::DoneWorking);
                                }
                                _ => {
                                    warn!("UPDATE ALL FAILED");
                                    sender.output(UpdatePageMsg::FailedWorking);
                                }
                            }
                        }
                        _ => {
                            warn!("UPDATE ALL FAILED");
                            sender.output(UpdatePageMsg::FailedWorking);
                        }
                    }
                });
            }
            UpdateAsyncHandlerMsg::UpdateAllRemove(userrmpkgs, sysrmpkgs) => {
                let config = self.config.clone();
                let syspkgs = self.syspkgs.clone();
                let userpkgs = self.userpkgs.clone();
                relm4::spawn(async move {
                    let result = runcmd(
                        NscCmd::All,
                        config,
                        syspkgs,
                        Some(sysrmpkgs),
                    )
                    .await;
                    match result {
                        Ok(true) => {
                            match match userpkgs {
                                UserPkgs::Env => updateenv().await,
                                UserPkgs::Profile => updateprofile(Some(userrmpkgs)).await,
                            } {
                                Ok(true) => {
                                    sender.output(UpdatePageMsg::DoneWorking);
                                }
                                _ => {
                                    warn!("UPDATE ALL FAILED");
                                    sender.output(UpdatePageMsg::FailedWorking);
                                }
                            }
                        }
                        _ => {
                            warn!("UPDATE ALL FAILED");
                            sender.output(UpdatePageMsg::FailedWorking);
                        }
                    }
                });
            }
        }
    }
}

async fn runcmd(
    cmd: NscCmd,
    config: NixDataConfig,
    syspkgs: SystemPkgs,
    rmpkgs: Option<Vec<String>>,
) -> Result<bool> {
    let systemconfig = config.systemconfig.unwrap_or_default();
    let flakeargs = if let Some(flake) = config.flake {
        if let Some(flakearg) = config.flakearg {
            Some(format!("{}#{}", flake, flakearg))
        } else {
            Some(flake)
        }
    } else {
        None
    };
    let f = fs::read_to_string(&systemconfig)?;
    let exe = match std::env::current_exe() {
        Ok(mut e) => {
            e.pop(); // root/bin
            e.pop(); // root/
            e.push("libexec"); // root/libexec
            e.push("nsc-helper");
            let x = e.to_string_lossy().to_string();
            info!("nsc-helper path: {}", x);
            if Path::new(&x).is_file() {
                x
            } else {
                String::from("nsc-helper")
            }
        }
        Err(_) => String::from("nsc-helper"),
    };

    let flakepathsplit = flakeargs.clone().unwrap_or_default().to_string();
    let flakepath = flakepathsplit
        .split('#')
        .collect::<Vec<&str>>()
        .first()
        .cloned()
        .unwrap_or_default();

    let rebuildargs = if let Some(x) = flakeargs {
        let mut v = vec![String::from("--flake")];
        for arg in x.split(' ') {
            if !arg.is_empty() {
                v.push(String::from(arg));
            }
        }
        v
    } else {
        vec![]
    };

    let mut cmd = match cmd {
        NscCmd::Rebuild => tokio::process::Command::new("pkexec")
            .arg(&exe)
            .arg("rebuild")
            .arg("--generations")
            .arg(config.generations.unwrap_or(0).to_string())
            .arg("--")
            .arg("switch")
            .args(&rebuildargs)
            .stderr(Stdio::piped())
            .spawn()?,
        NscCmd::Channel => tokio::process::Command::new("pkexec")
            .arg(&exe)
            .arg("channel")
            .arg("--output")
            .arg(&systemconfig)
            .stderr(Stdio::piped())
            .spawn()?,
        NscCmd::All => match syspkgs {
            SystemPkgs::Legacy => {
                if let Some(rmpkgs) = rmpkgs {
                    let newconfig =
                        match nix_editor::write::rmarr(&f, "environment.systemPackages", rmpkgs) {
                            Ok(x) => x,
                            Err(_) => {
                                return Err(anyhow!("Failed to write configuration.nix"));
                            }
                        };
                    let mut cmd = tokio::process::Command::new("pkexec")
                        .arg(&exe)
                        .arg("channel")
                        .arg("--rebuild")
                        .arg("--update")
                        .arg("--generations")
                        .arg(config.generations.unwrap_or(0).to_string())
                        .arg("--output")
                        .arg(&systemconfig)
                        .arg("--")
                        .arg("switch")
                        .args(&rebuildargs)
                        .stderr(Stdio::piped())
                        .stdin(Stdio::piped())
                        .spawn()?;
                    cmd.stdin
                        .take()
                        .unwrap()
                        .write_all(newconfig.as_bytes())
                        .await?;
                    cmd
                } else {
                    tokio::process::Command::new("pkexec")
                        .arg(&exe)
                        .arg("channel")
                        .arg("--rebuild")
                        .arg("--generations")
                        .arg(config.generations.unwrap_or(0).to_string())
                        .arg("--")
                        .arg("switch")
                        .args(&rebuildargs)
                        .stderr(Stdio::piped())
                        .spawn()?
                }
            }
            SystemPkgs::Flake => {
                if let Some(rmpkgs) = rmpkgs {
                    let newconfig =
                        match nix_editor::write::rmarr(&f, "environment.systemPackages", rmpkgs) {
                            Ok(x) => x,
                            Err(_) => {
                                return Err(anyhow!("Failed to write configuration.nix"));
                            }
                        };
                    let mut cmd = tokio::process::Command::new("pkexec")
                        .arg(&exe)
                        .arg("flake")
                        .arg("--rebuild")
                        .arg("--flakepath")
                        .arg(&flakepath)
                        .arg("--update")
                        .arg("--generations")
                        .arg(config.generations.unwrap_or(0).to_string())
                        .arg("--output")
                        .arg(&systemconfig)
                        .arg("--")
                        .arg("switch")
                        .arg("--impure")
                        .args(&rebuildargs)
                        .stderr(Stdio::piped())
                        .stdin(Stdio::piped())
                        .spawn()?;
                    cmd.stdin
                        .take()
                        .unwrap()
                        .write_all(newconfig.as_bytes())
                        .await?;
                    cmd
                } else {
                    tokio::process::Command::new("pkexec")
                        .arg(&exe)
                        .arg("flake")
                        .arg("--rebuild")
                        .arg("--flakepath")
                        .arg(&flakepath)
                        .arg("--generations")
                        .arg(config.generations.unwrap_or(0).to_string())
                        .arg("--output")
                        .arg(&systemconfig)
                        .arg("--")
                        .arg("switch")
                        .arg("--impure")
                        .args(&rebuildargs)
                        .stderr(Stdio::piped())
                        .spawn()?
                }
            }
            SystemPkgs::None => return Ok(true),
        },
    };

    let stderr = cmd.stderr.take().unwrap();
    let reader = tokio::io::BufReader::new(stderr);

    let mut lines = reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        REBUILD_BROKER.send(RebuildMsg::UpdateText(line.to_string()));
        trace!("CAUGHT REBUILD LINE: {}", line);
    }
    if cmd.wait().await?.success() {
        Ok(true)
    } else {
        Ok(false)
    }
}

async fn updateenv() -> Result<bool> {
    let mut cmd = tokio::process::Command::new("nix-env")
        .arg("-u")
        .stderr(Stdio::piped())
        .spawn()?;

    let stderr = cmd.stderr.take().unwrap();
    let reader = tokio::io::BufReader::new(stderr);

    let mut lines = reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        REBUILD_BROKER.send(RebuildMsg::UpdateText(line.to_string()));
        trace!("CAUGHT NIXENV LINE: {}", line);
    }
    if cmd.wait().await?.success() {
        Ok(true)
    } else {
        Ok(false)
    }
}

async fn updateprofile(rmpkgs: Option<Vec<String>>) -> Result<bool> {
    if let Some(rmpkgs) = rmpkgs {
        if !rmpkgs.is_empty() {
            let mut cmd = tokio::process::Command::new("nix")
                .arg("profile")
                .arg("remove")
                .args(
                    &rmpkgs
                        .iter()
                        .map(|x| format!("legacyPackages.x86_64-linux.{}", x))
                        .collect::<Vec<String>>(),
                )
                // Allow updating potential unfree packages
                .arg("--impure")
                .stderr(Stdio::piped())
                .spawn()?;

            let stderr = cmd.stderr.take().unwrap();
            let reader = tokio::io::BufReader::new(stderr);

            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                REBUILD_BROKER.send(RebuildMsg::UpdateText(line.to_string()));
                trace!("CAUGHT NIX PROFILE LINE: {}", line);
            }
            cmd.wait().await?;
        }
    }

    let mut cmd = tokio::process::Command::new("nix")
        .arg("profile")
        .arg("upgrade")
        .arg(".*")
        // Allow updating potential unfree packages
        .arg("--impure")
        .stderr(Stdio::piped())
        .spawn()?;

    let stderr = cmd.stderr.take().unwrap();
    let reader = tokio::io::BufReader::new(stderr);

    let mut lines = reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        REBUILD_BROKER.send(RebuildMsg::UpdateText(line.to_string()));
        trace!("CAUGHT NIX PROFILE LINE: {}", line);
    }
    if cmd.wait().await?.success() {
        Ok(true)
    } else {
        Ok(false)
    }
}
