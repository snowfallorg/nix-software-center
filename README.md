<div align="center">

<img src="data/icons/dev.vlinkz.NixSoftwareCenter.svg"/>

Nix Software Center
===

[![Built with Nix][builtwithnix badge]][builtwithnix]
[![License: GPLv3][GPLv3 badge]][GPLv3]
[![Chat on Matrix][matrix badge]][matrix]
[![Chat on Discord][discord badge]][discord]

A graphical app store for Nix built with [libadwaita](https://gitlab.gnome.org/GNOME/libadwaita), [GTK4](https://www.gtk.org/), and [Relm4](https://relm4.org/). Heavily inspired by [GNOME Software](https://gitlab.gnome.org/GNOME/gnome-software).

<img src="data/screenshots/overview-light.png#gh-light-mode-only"/>
<img src="data/screenshots/overview-dark.png#gh-dark-mode-only"/> 

</div>

# Features
- Install packages to `configuration.nix`
  - Flakes support can be enabled in the preferences menu
- Install packages with `nix profile` or `nix-env`
- Show updates for all installed packages
- Search for packages
- Launch applications without installing via `nix-shell` and `nix run`

## NixOS Flakes Installation
`flake.nix`
```nix
{
  inputs = {
    # other inputs
    nix-software-center.url = "github:snowfallorg/nix-software-center";
# rest of flake.nix
```

`configuration.nix`
```
environment.systemPackages = with pkgs; [
    inputs.nix-software-center.packages.${system}.nix-software-center
    # rest of your packages
];
```

## NixOS Installation

Head of `configuration.nix`

if you are on unstable channel or any version after 22.11:
```nix
{ config, pkgs, lib, ... }:
let
  nix-software-center = import (pkgs.fetchFromGitHub {
    owner = "snowfallorg";
    repo = "nix-software-center";
    rev = "0.1.2";
    sha256 = "xiqF1mP8wFubdsAQ1BmfjzCgOD3YZf7EGWl9i69FTls=";
  }) {};
in

...

environment.systemPackages =
with pkgs; [
  nix-software-center
  # rest of your packages
];
```

For any other method of installation, when rebuilding you might be prompted to authenticate twice in a row by `pkexec`

## 'nix profile' installation
```bash
nix profile install github:snowfallorg/nix-software-center
```

## 'nix-env' Installation

```bash
git clone https://github.com/snowfallorg/nix-software-center
nix-env -f nix-software-center -i nix-software-center
```

## Single run on an flakes enabled system:
```bash
nix run github:snowfallorg/nix-software-center
```

## Single run on non-flakes enabled system:
```bash
nix --extra-experimental-features "nix-command flakes" run github:snowfallorg/nix-software-center
```

## Debugging

```bash
RUST_LOG=nix_software_center=trace nix-software-center
```

## Screenshots
<p align="middle">
  <img src="data/screenshots/frontpage-light.png#gh-light-mode-only"/>
  <img src="data/screenshots/frontpage-dark.png#gh-dark-mode-only"/> 
</p>

<p align="middle">
  <img src="data/screenshots/application-light.png#gh-light-mode-only"/>
  <img src="data/screenshots/application-dark.png#gh-dark-mode-only"/> 
</p>

<p align="middle">
  <img src="data/screenshots/searchpage-light.png#gh-light-mode-only"/>
  <img src="data/screenshots/searchpage-dark.png#gh-dark-mode-only"/> 
</p>

## Licenses

Some icons in [data/icons](data/icons/) contains assets from the [NixOS logo](https://github.com/NixOS/nixos-artwork/tree/master/logo) and are licensed under a [CC-BY license](https://creativecommons.org/licenses/by/4.0/).

Some icons in [data/icons](data/icons/) contains assets from [GNOME Software](https://gitlab.gnome.org/GNOME/gnome-software/-/tree/main/data/icons/hicolor/scalable) and are licensed under [CC0-1.0](https://creativecommons.org/publicdomain/zero/1.0/).

[builtwithnix badge]: https://img.shields.io/badge/Built%20With-Nix-41439A?style=for-the-badge&logo=nixos&logoColor=white
[builtwithnix]: https://builtwithnix.org/
[GPLv3 badge]: https://img.shields.io/badge/License-GPLv3-blue.svg?style=for-the-badge
[GPLv3]: https://opensource.org/licenses/GPL-3.0
[matrix badge]: https://img.shields.io/badge/matrix-join%20chat-0cbc8c?style=for-the-badge&logo=matrix&logoColor=white
[matrix]: https://matrix.to/#/#snowflakeos:matrix.org
[discord badge]: https://img.shields.io/discord/1021080090676842506?color=7289da&label=Discord&logo=discord&logoColor=ffffff&style=for-the-badge
[discord]: https://discord.gg/6rWNMmdkgT
