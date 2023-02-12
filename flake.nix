{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils }:
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
        name = "nix-software-center";
      in
      rec
      {
        packages.${name} = pkgs.callPackage ./default.nix { };

        # `nix build`
        defaultPackage = packages.${name}; # legacy
        packages.default = packages.${name};

        # `nix run`
        apps.${name} = utils.lib.mkApp {
          inherit name;
          drv = packages.${name};
        };
        defaultApp = apps.${name};

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
      });
}
