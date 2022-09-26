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
        libadwaita-git = pkgs.libadwaita.overrideAttrs (oldAttrs: rec {
          version = "1.2.0";
          src = pkgs.fetchFromGitLab {
            domain = "gitlab.gnome.org";
            owner = "GNOME";
            repo = "libadwaita";
            rev = version;
            hash = "sha256-3lH7Vi9M8k+GSrCpvruRpLrIpMoOakKbcJlaAc/FK+U=";
          };
        });
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
        packages.${name} = pkgs.callPackage ./default.nix {
          inherit (inputs);
        };

        # `nix build`
        defaultPackage = packages.${name};

        # `nix run`
        apps.${name} = utils.lib.mkApp {
          inherit name;
          drv = packages.${name};
        };
        defaultApp = packages.${name};

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
            libadwaita-git
            meson
            ninja
            openssl
            pandoc
            pango
            pkgconfig
            polkit
            wrapGAppsHook4
            nixos-appstream-data
          ];
          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        };

        nixosModules.nix-software-center = ({ config, ... }: import ./modules/default.nix {
          inherit pkgs;
          inherit (pkgs) lib;
          inherit config;
          nix-software-center = defaultPackage;
        });
        nixosModules.default = nixosModules.nix-software-center;
      });
}
