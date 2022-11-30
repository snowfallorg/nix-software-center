use super::pkgpage::{InstallType, PkgAction, PkgMsg, WorkPkg};
use super::rebuild::RebuildMsg;
use super::window::{SystemPkgs, UserPkgs, REBUILD_BROKER};
use log::*;
use nix_data::config::configfile::NixDataConfig;
use relm4::*;
use anyhow::{Result, anyhow};
use std::path::Path;
use std::process::Stdio;
use std::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

#[tracker::track]
#[derive(Debug)]
pub struct InstallAsyncHandler {
    #[tracker::no_eq]
    process: Option<JoinHandle<()>>,
    work: Option<WorkPkg>,
    config: NixDataConfig,
    pid: Option<u32>,
    syspkgs: SystemPkgs,
    userpkgs: UserPkgs,
}

#[derive(Debug)]
pub enum InstallAsyncHandlerMsg {
    SetConfig(NixDataConfig),
    SetPkgTypes(SystemPkgs, UserPkgs),
    Process(WorkPkg),
    CancelProcess,
    SetPid(Option<u32>),
}

#[derive(Debug)]
pub struct InstallAsyncHandlerInit {
    pub syspkgs: SystemPkgs,
    pub userpkgs: UserPkgs,
}

impl Worker for InstallAsyncHandler {
    type Init = InstallAsyncHandlerInit;
    type Input = InstallAsyncHandlerMsg;
    type Output = PkgMsg;

    fn init(params: Self::Init, _sender: relm4::ComponentSender<Self>) -> Self {
        Self {
            process: None,
            work: None,
            config: NixDataConfig {
                systemconfig: None,
                flake: None,
                flakearg: None,
                generations: None
            },
            pid: None,
            syspkgs: params.syspkgs,
            userpkgs: params.userpkgs,
            tracker: 0,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.reset();
        match msg {
            InstallAsyncHandlerMsg::SetConfig(config) => {
                self.config = config;
            }
            InstallAsyncHandlerMsg::SetPkgTypes(syspkgs, userpkgs) => {
                self.syspkgs = syspkgs;
                self.userpkgs = userpkgs;
            }
            
            InstallAsyncHandlerMsg::Process(work) => {
                if work.block {
                    return;
                }
                let config = self.config.clone();
                match work.pkgtype {
                    InstallType::User => match work.action {
                        PkgAction::Install => {
                            info!("Installing user package: {}", work.pkg);
                            match self.userpkgs {
                                UserPkgs::Env => {
                                    self.process = Some(relm4::spawn(async move {
                                        let mut p = tokio::process::Command::new("nix-env")
                                            .arg("-iA")
                                            .arg(format!("nixos.{}", work.pkg))
                                            .kill_on_drop(true)
                                            .stdout(Stdio::piped())
                                            .stderr(Stdio::piped())
                                            .spawn()
                                            .expect("Failed to run nix-env");

                                        let stderr = p.stderr.take().unwrap();
                                        let reader = tokio::io::BufReader::new(stderr);

                                        let mut lines = reader.lines();
                                        while let Ok(Some(line)) = lines.next_line().await {
                                            trace!("CAUGHT LINE: {}", line);
                                        }

                                        match p.wait().await {
                                            Ok(o) => {
                                                if o.success() {
                                                    info!(
                                                        "Removed user package: {} success",
                                                        work.pkg
                                                    );
                                                    sender.output(PkgMsg::FinishedProcess(work));
                                                } else {
                                                    warn!(
                                                        "Removed user package: {} failed",
                                                        work.pkg
                                                    );
                                                    sender.output(PkgMsg::FailedProcess(work));
                                                }
                                            }
                                            Err(e) => {
                                                warn!("Error removing user package: {}", e);
                                                sender.output(PkgMsg::FailedProcess(work));
                                            }
                                        }
                                    }));
                                }
                                UserPkgs::Profile => {
                                    self.process = Some(relm4::spawn(async move {
                                        let mut p = tokio::process::Command::new("nix")
                                            .arg("profile")
                                            .arg("install")
                                            .arg(format!("nixpkgs#{}", work.pkg))
                                            .arg("--impure")
                                            .kill_on_drop(true)
                                            .stdout(Stdio::piped())
                                            .stderr(Stdio::piped())
                                            .spawn()
                                            .expect("Failed to run nix profile");

                                        let stderr = p.stderr.take().unwrap();
                                        let reader = tokio::io::BufReader::new(stderr);

                                        let mut lines = reader.lines();
                                        while let Ok(Some(line)) = lines.next_line().await {
                                            trace!("CAUGHT LINE: {}", line);
                                        }

                                        match p.wait().await {
                                            Ok(o) => {
                                                if o.success() {
                                                    info!(
                                                        "Removed user package: {} success",
                                                        work.pkg
                                                    );
                                                    sender.output(PkgMsg::FinishedProcess(work));
                                                } else {
                                                    warn!(
                                                        "Removed user package: {} failed",
                                                        work.pkg
                                                    );
                                                    sender.output(PkgMsg::FailedProcess(work));
                                                }
                                            }
                                            Err(e) => {
                                                warn!("Error removing user package: {}", e);
                                                sender.output(PkgMsg::FailedProcess(work));
                                            }
                                        }
                                    }));
                                }
                            }
                        }
                        PkgAction::Remove => {
                            info!("Removing user package: {}", work.pkg);
                            match self.userpkgs {
                                UserPkgs::Env => {
                                    self.process = Some(relm4::spawn(async move {
                                        let mut p = tokio::process::Command::new("nix-env")
                                            .arg("-e")
                                            .arg(&work.pname)
                                            .kill_on_drop(true)
                                            .stdout(Stdio::piped())
                                            .stderr(Stdio::piped())
                                            .spawn()
                                            .expect("Failed to run nix-env");
                                        let stderr = p.stderr.take().unwrap();
                                        let reader = tokio::io::BufReader::new(stderr);

                                        let mut lines = reader.lines();
                                        while let Ok(Some(line)) = lines.next_line().await {
                                            trace!("CAUGHT LINE: {}", line);
                                        }
                                        match p.wait().await {
                                            Ok(o) => {
                                                if o.success() {
                                                    info!(
                                                        "Removed user package: {} success",
                                                        work.pkg
                                                    );
                                                    sender.output(PkgMsg::FinishedProcess(work));
                                                } else {
                                                    warn!(
                                                        "Removed user package: {} failed",
                                                        work.pkg
                                                    );
                                                    sender.output(PkgMsg::FailedProcess(work));
                                                }
                                            }
                                            Err(e) => {
                                                warn!("Error removing user package: {}", e);
                                                sender.output(PkgMsg::FailedProcess(work));
                                            }
                                        }
                                    }));
                                }
                                UserPkgs::Profile => {
                                    self.process = Some(relm4::spawn(async move {
                                        let mut p = tokio::process::Command::new("nix")
                                            .arg("profile")
                                            .arg("remove")
                                            .arg(&format!(
                                                "legacyPackages.x86_64-linux.{}",
                                                work.pkg
                                            ))
                                            .kill_on_drop(true)
                                            .stdout(Stdio::piped())
                                            .stderr(Stdio::piped())
                                            .spawn()
                                            .expect("Failed to run nix profile");
                                        let stderr = p.stderr.take().unwrap();
                                        let reader = tokio::io::BufReader::new(stderr);

                                        let mut lines = reader.lines();
                                        while let Ok(Some(line)) = lines.next_line().await {
                                            trace!("CAUGHT LINE: {}", line);
                                        }
                                        match p.wait().await {
                                            Ok(o) => {
                                                if o.success() {
                                                    info!(
                                                        "Removed user package: {} success",
                                                        work.pkg
                                                    );
                                                    sender.output(PkgMsg::FinishedProcess(work));
                                                } else {
                                                    warn!(
                                                        "Removed user package: {} failed",
                                                        work.pkg
                                                    );
                                                    sender.output(PkgMsg::FailedProcess(work));
                                                }
                                            }

                                            Err(e) => {
                                                warn!("Error removing user package: {}", e);
                                                sender.output(PkgMsg::FailedProcess(work));
                                            }
                                        }
                                    }));
                                }
                            }
                        }
                    },
                    InstallType::System => {
                        REBUILD_BROKER.send(RebuildMsg::Show);
                        if let Some(systemconfig) = &config.systemconfig {
                            match work.action {
                                PkgAction::Install => {
                                    info!("Installing system package: {}", work.pkg);
                                    self.process = Some(relm4::spawn(async move {
                                        match installsys(
                                            work.pkg.to_string(),
                                            work.action.clone(),
                                            config,
                                            sender.clone(),
                                        )
                                        .await
                                        {
                                            Ok(b) => {
                                                if b {
                                                    REBUILD_BROKER.send(RebuildMsg::FinishSuccess);
                                                    sender.output(PkgMsg::FinishedProcess(work));
                                                } else {
                                                    REBUILD_BROKER.send(RebuildMsg::FinishError(None));
                                                    sender.output(PkgMsg::FailedProcess(work));
                                                }
                                            }
                                            Err(e) => {
                                                REBUILD_BROKER.send(RebuildMsg::FinishError(None));
                                                sender.output(PkgMsg::FailedProcess(work));
                                                warn!("Error installing system package: {}", e);
                                            }
                                        }
                                    }));
                                }
                                PkgAction::Remove => {
                                    info!("Removing system package: {}", work.pkg);
                                    self.process = Some(relm4::spawn(async move {
                                        match installsys(
                                            work.pkg.to_string(),
                                            work.action.clone(),
                                            config,
                                            sender.clone(),
                                        )
                                        .await
                                        {
                                            Ok(b) => {
                                                if b {
                                                    REBUILD_BROKER.send(RebuildMsg::FinishSuccess);
                                                    sender.output(PkgMsg::FinishedProcess(work));
                                                } else {
                                                    REBUILD_BROKER.send(RebuildMsg::FinishError(None));
                                                    sender.output(PkgMsg::FailedProcess(work));
                                                }
                                            }
                                            Err(e) => {
                                                REBUILD_BROKER.send(RebuildMsg::FinishError(None));
                                                sender.output(PkgMsg::FailedProcess(work));
                                                warn!("Error removing system package: {}", e);
                                            }
                                        }
                                    }));
                                }
                            }
                        }
                    }
                }
            }
            InstallAsyncHandlerMsg::CancelProcess => {
                info!("CANCELING PROCESS");
                if let Some(p) = &mut self.process {
                    p.abort()
                }
                self.process = None;
                self.pid = None;
                sender.output(PkgMsg::CancelFinished);
            }
            InstallAsyncHandlerMsg::SetPid(p) => self.pid = p,
        }
    }
}

async fn installsys(
    pkg: String,
    action: PkgAction,
    config: NixDataConfig,
    _sender: ComponentSender<InstallAsyncHandler>,
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
    let mut p = pkg;
    let f = fs::read_to_string(&systemconfig)?;
    if let Ok(s) = nix_editor::read::getwithvalue(&f, "environment.systemPackages") {
        if !s.contains(&"pkgs".to_string()) {
            p = format!("pkgs.{}", p);
        }
    } else {
        p = format!("pkgs.{}", p);
    }

    let out = match action {
        PkgAction::Install => {
            match nix_editor::write::addtoarr(&f, "environment.systemPackages", vec![p]) {
                Ok(x) => x,
                Err(_) => {
                    return Err(anyhow!("Failed to write configuration.nix"));
                }
            }
        }
        PkgAction::Remove => {
            match nix_editor::write::rmarr(&f, "environment.systemPackages", vec![p]) {
                Ok(x) => x,
                Err(_) => {
                    return Err(anyhow!("Failed to write configuration.nix"));
                }
            }
        }
    };

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

    let mut cmd = tokio::process::Command::new("pkexec")
        .arg(&exe)
        .arg("config")
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

    cmd.stdin.take().unwrap().write_all(out.as_bytes()).await?;
    let stderr = cmd.stderr.take().unwrap();
    let reader = tokio::io::BufReader::new(stderr);

    let mut lines = reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        trace!("CAUGHT LINE: {}", line);
        REBUILD_BROKER.send(RebuildMsg::UpdateText(line));
    }
    if cmd.wait().await?.success() {
        Ok(true)
    } else {
        Ok(false)
    }
}
