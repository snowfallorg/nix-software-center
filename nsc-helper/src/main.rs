use clap::{self, FromArgMatches, Subcommand};
use std::{
    error::Error,
    fs::{self, File},
    io::{self, Read, Write},
    process::Command,
};

#[derive(Subcommand, Debug)]
enum SubCommands {
    Config {
        /// Write stdin to file in path output
        #[arg(short, long)]
        output: String,
        /// How many generations to keep
        #[arg(short, long)]
        generations: Option<u32>,
        /// Run `nixos-rebuild` with the given arguments
        arguments: Vec<String>,
    },
    Rebuild {
        /// Run `nixos-rebuild` with the given arguments
        arguments: Vec<String>,
        /// How many generations to keep
        #[arg(short, long)]
        generations: Option<u32>,
    },
    Channel {
        /// Whether to rebuild the system after updating channels
        #[arg(short, long)]
        rebuild: bool,
        /// Update file
        #[arg(short, long)]
        update: bool,
        /// Write stdin to file in path output
        #[arg(short, long)]
        output: String,
        /// How many generations to keep
        #[arg(short, long)]
        generations: Option<u32>,
        /// Run `nixos-rebuild` with the given arguments
        arguments: Vec<String>,
    },
    Flake {
        /// Whether to rebuild the system after updating flake
        #[arg(short, long)]
        rebuild: bool,
        /// Path to the flake file
        #[arg(short, long)]
        flakepath: String,
        /// Update file
        #[arg(short, long)]
        update: bool,
        /// Write stdin to file in path output
        #[arg(short, long)]
        output: String,
        /// How many generations to keep
        #[arg(short, long)]
        generations: Option<u32>,
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
        SubCommands::Config {
            output,
            generations,
            arguments,
        } => {
            let old = fs::read_to_string(&output);
            match write_file(&output) {
                Ok(_) => match rebuild(arguments, generations) {
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
        SubCommands::Rebuild {
            generations,
            arguments,
        } => match rebuild(arguments, generations) {
            Ok(_) => (),
            Err(err) => {
                eprintln!("{}", err);
                std::process::exit(1);
            }
        },
        SubCommands::Channel {
            rebuild: dorebuild,
            update,
            output,
            generations,
            arguments,
        } => {
            if update {
                if let Err(e) = write_file(&output) {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
            match channel() {
                Ok(_) => {
                    if dorebuild {
                        match rebuild(arguments, generations) {
                            Ok(_) => (),
                            Err(err) => {
                                eprintln!("{}", err);
                                std::process::exit(1);
                            }
                        }
                    }
                }
                Err(err) => {
                    eprintln!("{}", err);
                    std::process::exit(1);
                }
            }
        }
        SubCommands::Flake {
            rebuild: dorebuild,
            flakepath,
            update,
            output,
            generations,
            arguments,
        } => {
            if update {
                if let Err(e) = write_file(&output) {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
            match flake(&flakepath) {
                Ok(_) => {
                    if dorebuild {
                        match rebuild(arguments, generations) {
                            Ok(_) => (),
                            Err(err) => {
                                eprintln!("{}", err);
                                std::process::exit(1);
                            }
                        }
                    }
                }
                Err(err) => {
                    eprintln!("{}", err);
                    std::process::exit(1);
                }
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

fn rebuild(args: Vec<String>, generations: Option<u32>) -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new("nixos-rebuild").args(args).spawn()?;
    let x = cmd.wait()?;
    if !x.success() {
        eprintln!("nixos-rebuild failed with exit code {}", x.code().unwrap());
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "nixos-rebuild failed",
        )));
    }
    if let Some(g) = generations {
        if g > 0 {
            let mut cmd = Command::new("nix-env")
                .arg("--delete-generations")
                .arg("-p")
                .arg("/nix/var/nix/profiles/system")
                .arg(&format!("+{}", g))
                .spawn()?;
            let x = cmd.wait()?;
            if !x.success() {
                eprintln!(
                    "nix-env --delete-generations failed with exit code {}",
                    x.code().unwrap()
                );
                return Err(Box::new(io::Error::new(
                    io::ErrorKind::Other,
                    "nix-env failed",
                )));
            }
        }
    }
    Ok(())
}

fn channel() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new("nix-channel").arg("--update").spawn()?;
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
        .arg("update")
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
