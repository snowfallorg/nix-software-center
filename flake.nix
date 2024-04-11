{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    nixos-appstream-data = {
      url = "github:korfuri/nixos-appstream-data/flake";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };
  };

  outputs = { self, nixpkgs, utils, nixos-appstream-data, ... }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
      in
      rec
      {
        packages = let
          nix-software-center = pkgs.callPackage ./default.nix { inherit (nixos-appstream-data.packages."${system}") nixos-appstream-data; };
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
            nixos-appstream-data.packages."${system}".nixos-appstream-data
          ];
          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        };
      })
    // {
      overlays = {
        pkgs = final: prev: {
          nix-software-center = self.packages.${final.system}.nix-software-center;
        };
        nixSoftwareCenterPkgs = final: prev: {
          nixSoftwareCenterPkgs = self.packages.${prev.system};
        };
        default = self.overlays.nixSoftwareCenterPkgs;
      };
      overlay = self.overlays.pkgs;
    };
}
