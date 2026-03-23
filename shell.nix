{
  pkgs,
  appstream-data,
  ...
}:
pkgs.mkShell {
  packages = with pkgs; [
    nil
    nixfmt

    rustc
    cargo
    rustfmt
    clippy
    rust-analyzer
    bacon

    openssl
    sqlite
    just

    gtk4
    meson
    ninja
    parted
    gettext
    appstream
    pkg-config
    gdk-pixbuf
    libadwaita
    gnome-desktop
    wrapGAppsHook4
    desktop-file-utils
    gobject-introspection
    rustPlatform.bindgenHook
    blueprint-compiler
  ];

  RUST_BACKTRACE = "full";
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
  NSC_APPSTREAM_DATA = "${appstream-data}/share/swcatalog";
}
