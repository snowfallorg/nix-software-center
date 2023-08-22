{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, ... }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
        nixos-appstream-data = pkgs.fetchFromGitHub {
          owner = "vlinkz";
          repo = "nixos-appstream-data";
          rev = "66b3399e6d81017c10265611a151d1109ff1af1b";
          hash = "sha256-oiEZD4sMpb2djxReg99GUo0RHWAehxSyQBbiz8Z4DJk=";
        };
      in
      rec
      {
        packages = let
          nix-software-center = pkgs.callPackage ./default.nix {};
        in {
          inherit nix-software-center;
          default = nix-software-center;
        };

        checks = self.packages.${system};
        hydraJobs = self.packages.${system};

        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            clippy
            desktop-file-utils
            rust-analyzer
            rustc
            rustfmt
            cairo
            gdk-pixbuf
            gobject-introspection
            graphene
            gtk4
            gtksourceview5
            libadwaita
            libxml2
            meson
            ninja
            openssl
            pandoc
            pango
            pkg-config
            polkit
            sqlite
            wrapGAppsHook4
            nixos-appstream-data
          ];
          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        };
      })
    // {
      overlays = {
        default = final: prev: {
          nix-software-center = self.packages.${final.system}.nix-software-center;
        };
        pkgs = final: prev: {
          nixSoftwareCenterPkgs = self.packages.${prev.system};
        };
      };
      overlay = self.overlays.default;
    };
}
