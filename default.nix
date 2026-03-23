{
  pkgs,
  nixos-appstream-data ? null,
  ...
}:
let
  manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
in
pkgs.stdenv.mkDerivation {
  pname = manifest.name;
  version = manifest.version;

  src = pkgs.lib.cleanSource ./.;

  cargoDeps = pkgs.rustPlatform.importCargoLock {
    lockFile = ./Cargo.lock;
    outputHashes = {
      "libsnow-0.0.2-alpha.1" = "sha256-R3OES4w1WPrfIwrxqGam2gwpxwDN3BHtUpbf8vhyARQ=";
      "libappstream-0.5.0" = "sha256-pJmRtoT28OGlSrMujESrJkrL4HmIXDjHWqn96jBgfhM=";
    };
  };

  nativeBuildInputs = with pkgs; [
    git
    rustc
    cargo
    ninja
    meson
    clippy
    gettext
    pkg-config
    rust-analyzer
    wrapGAppsHook4
    appstream
    desktop-file-utils
    blueprint-compiler
    rustPlatform.cargoSetupHook
  ];

  buildInputs = with pkgs; [
    gtk4
    glib
    openssl
    sqlite
    libadwaita
    gdk-pixbuf
    gnome-desktop
    adwaita-icon-theme
    desktop-file-utils
    rustPlatform.bindgenHook
  ];

  preFixup = pkgs.lib.optionalString (nixos-appstream-data != null) ''
    gappsWrapperArgs+=(--set-default NSC_APPSTREAM_DATA "${nixos-appstream-data}/share/swcatalog")
  '';
}
