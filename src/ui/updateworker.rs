use nix_data::config::configfile::NixDataConfig;
use relm4::*;
use std::{error::Error, path::Path, process::Stdio};
use tokio::io::AsyncBufReadExt;
use log::*;

use super::{
    updatepage::UpdatePageMsg,
    window::{SystemPkgs, UserPkgs},
};

#[tracker::track]
#[derive(Debug)]
pub struct UpdateAsyncHandler {
    #[tracker::no_eq]
    process: Option<JoinHandle<()>>,
    systemconfig: Option<String>,
    flakeargs: Option<String>,
    syspkgs: SystemPkgs,
    userpkgs: UserPkgs,
}

#[derive(Debug)]
pub enum UpdateAsyncHandlerMsg {
    UpdateConfig(NixDataConfig),
    UpdatePkgTypes(SystemPkgs, UserPkgs),

    UpdateChannels,
    UpdateChannelsAndSystem,

    RebuildSystem,
    UpdateUserPkgs,

    UpdateAll,
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
            systemconfig: None,
            flakeargs: None,
            syspkgs: params.syspkgs,
            userpkgs: params.userpkgs,
            tracker: 0,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            UpdateAsyncHandlerMsg::UpdateConfig(config) => {
                self.systemconfig = config.systemconfig;
                self.flakeargs = if let Some(flake) = config.flake {
                    if let Some(flakearg) = config.flakearg {
                        Some(format!("{}#{}", flake, flakearg))
                    } else {
                        Some(flake)
                    }
                } else {
                    None
                }
            }
            UpdateAsyncHandlerMsg::UpdatePkgTypes(syspkgs, userpkgs) => {
                self.syspkgs = syspkgs;
                self.userpkgs = userpkgs;
            }
            UpdateAsyncHandlerMsg::UpdateChannels => {
                let systemconfig = self.systemconfig.clone();
                let flakeargs = self.flakeargs.clone();
                let syspkgs = self.syspkgs.clone();
                relm4::spawn(async move {
                    let result = runcmd(NscCmd::Channel, systemconfig, flakeargs, syspkgs).await;
                    match result {
                        Ok(true) => {
                            sender.output(UpdatePageMsg::DoneWorking);
                        }
                        _ => {
                            warn!("UPDATE CHANNEL FAILED");
                            sender.output(UpdatePageMsg::FailedWorking);
                        }
                    }
                });
            }
            UpdateAsyncHandlerMsg::UpdateChannelsAndSystem => {
                let systenconfig = self.systemconfig.clone();
                let flakeargs = self.flakeargs.clone();
                let syspkgs = self.syspkgs.clone();
                relm4::spawn(async move {
                    let result = runcmd(NscCmd::All, systenconfig, flakeargs, syspkgs).await;
                    match result {
                        Ok(true) => {
                            sender.output(UpdatePageMsg::DoneWorking);
                        }
                        _ => {
                            warn!("UPDATE CHANNEL AND SYSTEM FAILED");
                            sender.output(UpdatePageMsg::FailedWorking);
                        }
                    }
                });
            }
            UpdateAsyncHandlerMsg::RebuildSystem => {
                let systemconfig = self.systemconfig.clone();
                let flakeargs = self.flakeargs.clone();
                let syspkgs = self.syspkgs.clone();
                relm4::spawn(async move {
                    let result = match syspkgs {
                        SystemPkgs::Legacy => runcmd(NscCmd::Rebuild, systemconfig, flakeargs, syspkgs).await,
                        SystemPkgs::Flake => runcmd(NscCmd::All, systemconfig, flakeargs, syspkgs).await,
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
                        UserPkgs::Profile => updateprofile().await,
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
                let systemconfig = self.systemconfig.clone();
                let flakeargs = self.flakeargs.clone();
                let syspkgs = self.syspkgs.clone();
                let userpkgs = self.userpkgs.clone();
                relm4::spawn(async move {
                    let result = runcmd(NscCmd::All, systemconfig, flakeargs, syspkgs).await;
                    match result {
                        Ok(true) => {
                            match match userpkgs {
                                UserPkgs::Env => updateenv().await,
                                UserPkgs::Profile => updateprofile().await,
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
    _systemconfig: Option<String>,
    flakeargs: Option<String>,
    syspkgs: SystemPkgs,
) -> Result<bool, Box<dyn Error + Send + Sync>> {
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
    let flakepath = flakepathsplit.split('#').collect::<Vec<&str>>().first().cloned().unwrap_or_default();

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
            .arg("--")
            .arg("switch")
            .args(&rebuildargs)
            .stderr(Stdio::piped())
            .spawn()?,
        NscCmd::Channel => tokio::process::Command::new("pkexec")
            .arg(&exe)
            .arg("channel")
            .stderr(Stdio::piped())
            .spawn()?,
        NscCmd::All => match syspkgs {
            SystemPkgs::Legacy => tokio::process::Command::new("pkexec")
                .arg(&exe)
                .arg("channel")
                .arg("--rebuild")
                .arg("--")
                .arg("switch")
                .args(&rebuildargs)
                .stderr(Stdio::piped())
                .spawn()?,
            SystemPkgs::Flake => tokio::process::Command::new("pkexec")
                .arg(&exe)
                .arg("flake")
                .arg("--rebuild")
                .arg("--flakepath")
                .arg(flakepath)
                .arg("--")
                .arg("switch")
                .args(&rebuildargs)
                .stderr(Stdio::piped())
                .spawn()?,
            SystemPkgs::None => return Ok(true),
        },
    };

    let stderr = cmd.stderr.take().unwrap();
    let reader = tokio::io::BufReader::new(stderr);

    let mut lines = reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        trace!("CAUGHT REBUILD LINE: {}", line);
    }
    if cmd.wait().await?.success() {
        Ok(true)
    } else {
        Ok(false)
    }
}

async fn updateenv() -> Result<bool, Box<dyn Error + Send + Sync>> {
    let mut cmd = tokio::process::Command::new("nix-env")
        .arg("-u")
        .stderr(Stdio::piped())
        .spawn()?;

    let stderr = cmd.stderr.take().unwrap();
    let reader = tokio::io::BufReader::new(stderr);

    let mut lines = reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        trace!("CAUGHT NIXENV LINE: {}", line);
    }
    if cmd.wait().await?.success() {
        Ok(true)
    } else {
        Ok(false)
    }
}

async fn updateprofile() -> Result<bool, Box<dyn Error + Send + Sync>> {
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
        trace!("CAUGHT NIX PROFILE LINE: {}", line);
    }
    if cmd.wait().await?.success() {
        Ok(true)
    } else {
        Ok(false)
    }
}