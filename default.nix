{
  pkgs ? import <nixpkgs> { },
  lib ? import <nixpkgs/lib>,
  nixos-appstream-data ? (
    import
      (pkgs.fetchFromGitHub {
        owner = "vlinkz";
        repo = "nixos-appstream-data";
        rev = "66b3399e6d81017c10265611a151d1109ff1af1b";
        hash = "sha256-oiEZD4sMpb2djxReg99GUo0RHWAehxSyQBbiz8Z4DJk=";
      })
      {
        set = "all";
        stdenv = pkgs.stdenv;
        lib = pkgs.lib;
        pkgs = pkgs;
      }
  ),
}:
pkgs.stdenv.mkDerivation {
  pname = "nix-software-center";
  version = "0.1.2";

  src = [ ./. ];

  cargoDeps = pkgs.rustPlatform.importCargoLock {
    lockFile = ./Cargo.lock;
    outputHashes = {
      "nix-data-0.0.2" = "sha256-yts2bkp9cn4SuYPYjgTNbOwTtpFxps3TU8zmS/ftN/Q=";
    };
  };

  nativeBuildInputs =
    with pkgs;
    [
      appstream-glib
      polkit
      gettext
      desktop-file-utils
      meson
      ninja
      pkg-config
      git
      wrapGAppsHook4
    ]
    ++ (with pkgs.rustPlatform; [
      cargoSetupHook
      cargo
      rustc
    ]);

  buildInputs = with pkgs; [
    gdk-pixbuf
    glib
    gtk4
    gtksourceview5
    libadwaita
    libxml2
    openssl
    wayland
    adwaita-icon-theme
    desktop-file-utils
    nixos-appstream-data
  ];

  patchPhase = ''
    substituteInPlace ./src/lib.rs \
        --replace "/usr/share/app-info" "${nixos-appstream-data}/share/app-info"
  '';

  postInstall = ''
    wrapProgram $out/bin/nix-software-center --prefix PATH : '${
      lib.makeBinPath [
        pkgs.gnome-console
        pkgs.gtk3 # provides gtk-launch
        pkgs.sqlite
      ]
    }'
  '';
}
