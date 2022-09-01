use clap::{self, FromArgMatches, Subcommand};
use std::{
    error::Error,
    fs::{File, self},
    io::{self, Read, Write},
    process::Command,
};

#[derive(clap::Subcommand)]
enum SubCommands {
    Config {
        /// Write stdin to file in path output
        #[clap(short, long)]
        output: String,
        /// Run `nixos-rebuild` with the given arguments
        arguments: Vec<String>,
    },
    Rebuild {
        /// Run `nixos-rebuild` with the given arguments
        arguments: Vec<String>,
    },
    Channel {
        /// Whether to rebuild the system after updating channels
        #[clap(short, long)]
        rebuild: bool,
        /// Run `nixos-rebuild` with the given arguments
        arguments: Vec<String>,
    },
    Flake {
        /// Whether to rebuild the system after updating flake
        #[clap(short, long)]
        rebuild: bool,
        /// Path to the flake file
        flakepath: String,
        /// Run `nixos-rebuild` with the given arguments
        arguments: Vec<String>,
    },
}

fn main() {
    let cli = SubCommands::augment_subcommands(clap::Command::new(
        "Helper binary for Nix Software Center",
    ));
    let matches = cli.get_matches();
    let derived_subcommands = SubCommands::from_arg_matches(&matches)
        .map_err(|err| err.exit())
        .unwrap();

    if users::get_effective_uid() != 0 {
        eprintln!("nsc-helper must be run as root");
        std::process::exit(1);
    }

    match derived_subcommands {
        SubCommands::Config { output, arguments } => {
            let old = fs::read_to_string(&output);
            match write_file(&output) {
                Ok(_) => match rebuild(arguments) {
                    Ok(_) => {}
                    Err(err) => {
                        eprintln!("{}", err);
                        if let Ok(o) = old {
                            if fs::write(&output, o).is_err() {
                                eprintln!("Could not restore old file");
                            }
                        }
                        std::process::exit(1);
                    }
                },
                Err(err) => {
                    eprintln!("{}", err);
                    std::process::exit(1);
                }
            };
        }
        SubCommands::Rebuild { arguments } => match rebuild(arguments) {
            Ok(_) => (),
            Err(err) => {
                eprintln!("{}", err);
                std::process::exit(1);
            }
        },
        SubCommands::Channel { rebuild: dorebuild, arguments } => {
            match dorebuild {
                true => match rebuild(arguments) {
                    Ok(_) => (),
                    Err(err) => {
                        eprintln!("{}", err);
                        std::process::exit(1);
                    }
                },
                false => match channel() {
                    Ok(_) => (),
                    Err(err) => {
                        eprintln!("{}", err);
                        std::process::exit(1);
                    }
                },
            }
        },
        SubCommands::Flake { rebuild: dorebuild, flakepath, arguments } => {
            match dorebuild {
                true => match rebuild(arguments) {
                    Ok(_) => (),
                    Err(err) => {
                        eprintln!("{}", err);
                        std::process::exit(1);
                    }
                },
                false => match flake(&flakepath) {
                    Ok(_) => (),
                    Err(err) => {
                        eprintln!("{}", err);
                        std::process::exit(1);
                    }
                },
            } 
        }
    }
}

fn write_file(path: &str) -> Result<(), Box<dyn Error>> {
    let stdin = io::stdin();
    let mut buf = String::new();
    stdin.lock().read_to_string(&mut buf)?;
    let mut file = File::create(path)?;
    write!(file, "{}", &buf)?;
    Ok(())
}

fn rebuild(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new("nixos-rebuild")
        .args(args)
        .spawn()?;
    let x = cmd.wait()?;
    if x.success() {
        Ok(())
    } else {
        eprintln!("nixos-rebuild failed with exit code {}", x.code().unwrap());
        Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "nixos-rebuild failed",
        )))
    }
}

fn channel() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new("nix-channel")
        .arg("--update")
        .spawn()?;
    let x = cmd.wait()?;
    if x.success() {
        Ok(())
    } else {
        eprintln!("nix-channel failed with exit code {}", x.code().unwrap());
        Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "nix-channel failed",
        )))
    }
}

fn flake(path: &str) -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new("nix")
        .arg("flake")
        .arg("upgrade")
        .arg(path)
        .spawn()?;
    let x = cmd.wait()?;
    if x.success() {
        Ok(())
    } else {
        eprintln!("nix flake failed with exit code {}", x.code().unwrap());
        Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "nix flake failed",
        )))
    }
}