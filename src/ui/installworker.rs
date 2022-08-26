use crate::parse::config::NscConfig;

use super::pkgpage::{InstallType, PkgAction, PkgMsg, WorkPkg};
use relm4::*;
use std::error::Error;
use std::path::Path;
use std::process::Stdio;
use std::{fs, io};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

#[tracker::track]
#[derive(Debug)]
pub struct InstallAsyncHandler {
    #[tracker::no_eq]
    process: Option<JoinHandle<()>>,
    work: Option<WorkPkg>,
    systemconfig: String,
    flakeargs: Option<String>,
    pid: Option<u32>,
}

#[derive(Debug)]
pub enum InstallAsyncHandlerMsg {
    SetConfig(NscConfig),
    Process(WorkPkg),
    CancelProcess,
    SetPid(Option<u32>),
}

impl Worker for InstallAsyncHandler {
    type InitParams = ();
    type Input = InstallAsyncHandlerMsg;
    type Output = PkgMsg;

    fn init(_params: Self::InitParams, _sender: relm4::ComponentSender<Self>) -> Self {
        Self {
            process: None,
            work: None,
            systemconfig: String::new(),
            flakeargs: None,
            pid: None,
            tracker: 0,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.reset();
        match msg {
            InstallAsyncHandlerMsg::SetConfig(config) => {
                self.systemconfig = config.systemconfig;
                self.flakeargs = config.flake;
            }
            InstallAsyncHandlerMsg::Process(work) => {
                if work.block {
                    return;
                }
                let systemconfig = self.systemconfig.clone();
                let rebuildargs = self.flakeargs.clone();
                match work.pkgtype {
                    InstallType::User => {
                        match work.action {
                            PkgAction::Install => {
                                println!("Installing user package: {}", work.pkg);
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
                                        println!("CAUGHT LINE: {}", line);
                                    }

                                    match p.wait().await {
                                        Ok(o) => {
                                            if o.success() {
                                                println!(
                                                    "Removed user package: {} success",
                                                    work.pkg
                                                );
                                                // println!("{}", String::from_utf8_lossy(&pstdout));
                                                sender.output(PkgMsg::FinishedProcess(work))
                                            } else {
                                                println!(
                                                    "Removed user package: {} failed",
                                                    work.pkg
                                                );
                                                // println!("{}", String::from_utf8_lossy(&p.stderr));
                                                sender.output(PkgMsg::FailedProcess(work));
                                            }
                                        }
                                        Err(e) => {
                                            println!("Error removing user package: {}", e);
                                            sender.output(PkgMsg::FailedProcess(work));
                                        }
                                    }
                                }));
                            }
                            PkgAction::Remove => {
                                println!("Removing user package: {}", work.pkg);
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
                                        println!("CAUGHT LINE: {}", line);
                                    }
                                    match p.wait().await {
                                        Ok(o) => {
                                            if o.success() {
                                                println!(
                                                    "Removed user package: {} success",
                                                    work.pkg
                                                );
                                                // println!("{}", String::from_utf8_lossy(&pstdout));
                                                sender.output(PkgMsg::FinishedProcess(work))
                                            } else {
                                                println!(
                                                    "Removed user package: {} failed",
                                                    work.pkg
                                                );
                                                // println!("{}", String::from_utf8_lossy(&p.stderr));
                                                sender.output(PkgMsg::FailedProcess(work));
                                            }
                                        }
                                        Err(e) => {
                                            println!("Error removing user package: {}", e);
                                            sender.output(PkgMsg::FailedProcess(work));
                                        }
                                    }
                                }));
                            }
                        }
                    }
                    InstallType::System => match work.action {
                        PkgAction::Install => {
                            println!("Installing system package: {}", work.pkg);
                            self.process = Some(relm4::spawn(async move {
                                match installsys(
                                    work.pkg.to_string(),
                                    work.action.clone(),
                                    systemconfig,
                                    rebuildargs,
                                    sender.clone(),
                                )
                                .await
                                {
                                    Ok(b) => {
                                        if b {
                                            sender.output(PkgMsg::FinishedProcess(work))
                                        } else {
                                            sender.output(PkgMsg::FailedProcess(work))
                                        }
                                    }
                                    Err(e) => {
                                        sender.output(PkgMsg::FailedProcess(work));
                                        println!("Error installing system package: {}", e);
                                    }
                                }
                            }));
                        }
                        PkgAction::Remove => {
                            println!("Removing system package: {}", work.pkg);
                            self.process = Some(relm4::spawn(async move {
                                match installsys(
                                    work.pkg.to_string(),
                                    work.action.clone(),
                                    systemconfig,
                                    rebuildargs,
                                    sender.clone(),
                                )
                                .await
                                {
                                    Ok(b) => {
                                        if b {
                                            sender.output(PkgMsg::FinishedProcess(work))
                                        } else {
                                            sender.output(PkgMsg::FailedProcess(work))
                                        }
                                    }
                                    Err(e) => {
                                        sender.output(PkgMsg::FailedProcess(work));
                                        println!("Error removing system package: {}", e);
                                    }
                                }
                            }));
                        }
                    },
                }
            }
            InstallAsyncHandlerMsg::CancelProcess => {
                println!("CANCELING PROCESS");
                // if let Some(p) = self.pid {
                //     println!("Killing process: {}", p);
                //     Command::new("pkexec")
                //         .arg("kill")
                //         .arg("-INT")
                //         .arg(p.to_string())
                //         .spawn()
                //         .expect("Failed to kill process");
                // }
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
    systemconfig: String,
    flakeargs: Option<String>,
    _sender: ComponentSender<InstallAsyncHandler>,
) -> Result<bool, Box<dyn Error>> {
    let mut p = pkg;
    let f = fs::read_to_string(&systemconfig)?;
    if let Ok(s) = nix_editor::read::getwithvalue(&f, "environment.systemPackages") {
        if !s.contains(&"pkgs".to_string()) {
            p = format!("pkgs.{}", p);
        }
    } else {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidData,
            "Failed to write configuration.nix",
        )));
    }

    let out = match action {
        PkgAction::Install => {
            match nix_editor::write::addtoarr(&f, "environment.systemPackages", vec![p]) {
                Ok(x) => x,
                Err(_) => {
                    return Err(Box::new(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Failed to write configuration.nix",
                    )))
                }
            }
        }
        PkgAction::Remove => {
            match nix_editor::write::rmarr(&f, "environment.systemPackages", vec![p]) {
                Ok(x) => x,
                Err(_) => {
                    return Err(Box::new(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Failed to write configuration.nix",
                    )))
                }
            }
        }
    };

    let exe = match std::env::current_exe() {
        Ok(mut e) => {
            e.pop(); // root/bin
                     // e.pop(); // root/
                     // e.push("libexec"); // root/libexec
            e.push("nsc-helper");
            let x = e.to_string_lossy().to_string();
            println!("CURRENT PATH {}", x);
            if Path::new(&x).is_file() {
                x
            } else {
                String::from("nsc-helper")
            }
        }
        Err(_) => String::from("nsc-helper"),
    };

    println!("EXECUTING {}", exe);

    // let rebuildargs = match flakeargs {
    //     Some(x) => format!("--flake {}", x),//.split(' ').map(|x| x.to_string()).collect::<Vec<String>>(),
    //     None => String::default(),
    // };

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
    println!("Rebuild args: {:?}", rebuildargs);

    let mut cmd = tokio::process::Command::new("pkexec")
        .arg(&exe)
        .arg("config")
        .arg("--output")
        .arg(&systemconfig)
        .arg("--")
        .arg("switch")
        .args(&rebuildargs)
        .stderr(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()?;
        
    // sender.input(InstallAsyncHandlerMsg::SetPid(cmd.id()));

    cmd.stdin.take().unwrap().write_all(out.as_bytes()).await?;
    println!("SENT INPUT");
    let stderr = cmd.stderr.take().unwrap();
    let reader = tokio::io::BufReader::new(stderr);

    let mut lines = reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        println!("CAUGHT LINE: {}", line);
    }
    println!("READER DONE");
    if cmd.wait().await?.success() {
        println!("SUCCESS");
        // sender.input(InstallAsyncHandlerMsg::SetPid(None));
        Ok(true)
    } else {
        println!("FAILURE");
        // sender.input(InstallAsyncHandlerMsg::SetPid(None));
        Ok(false)
    }
}
