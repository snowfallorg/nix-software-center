{
  pkgs ? import <nixpkgs> {},
  lib ? import <nixpkgs/lib>,
}:
let
  libadwaita-git = pkgs.libadwaita.overrideAttrs (oldAttrs: rec {
    version = "1.2.rc";
    src = pkgs.fetchFromGitLab {
      domain = "gitlab.gnome.org";
      owner = "GNOME";
      repo = "libadwaita";
      rev = version;
      hash = "sha256-p7nsaAqrzQKeUiu7aGlgoKu0AH9KV/sMsVcLLpgl4Lo=";
    };
  });
  nixos-appstream-data = (import (pkgs.fetchFromGitHub {
    owner = "vlinkz";
    repo = "nixos-appstream-data";
    rev = "66b3399e6d81017c10265611a151d1109ff1af1b";
    hash = "sha256-oiEZD4sMpb2djxReg99GUo0RHWAehxSyQBbiz8Z4DJk=";
  }) {stdenv = pkgs.stdenv; lib = pkgs.lib; pkgs = pkgs; });
in pkgs.stdenv.mkDerivation rec {
  pname = "nix-software-center";
  version = "0.0.1";

  src = [ ./. ];

  cargoDeps = pkgs.rustPlatform.fetchCargoTarball {
    inherit src;
    name = "${pname}-${version}";
    hash = "sha256-c21JMLq1m5bLhmZihCbOjAr8WShDimaAjYISpai8oXE=";
  };

  nativeBuildInputs = with pkgs; [
    appstream-glib
    polkit
    gettext
    desktop-file-utils
    meson
    ninja
    pkg-config
    git
    wrapGAppsHook4
  ] ++ (with pkgs.rustPlatform; [
    cargoSetupHook
    rust.cargo
    rust.rustc
  ]);

  buildInputs = with pkgs; [
    gdk-pixbuf
    glib
    gtk4
    gtksourceview5
    libadwaita-git
    openssl
    wayland
    gnome.adwaita-icon-theme
    desktop-file-utils
    nixos-appstream-data
  ];

  patchPhase = ''
    substituteInPlace ./src/lib.rs \
        --replace "/usr/share/app-info" "${nixos-appstream-data}/share/app-info"
  '';
}
