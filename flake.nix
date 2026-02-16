{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nixos-appstream-data = {
      url = "github:snowfallorg/nixos-appstream-data";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      nixos-appstream-data,
      ...
    }:
    let
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs nixpkgs.lib.systems.flakeExposed (
          system: f nixpkgs.legacyPackages.${system} system
        );
    in
    {
      formatter = forAllSystems (pkgs: _: pkgs.nixfmt);

      devShells = forAllSystems (
        pkgs: system: {
          default = import ./shell.nix {
            inherit pkgs;
            appstream-data = nixos-appstream-data.packages.${system}.appstream-data-all;
          };
        }
      );

      packages = forAllSystems (
        pkgs: system: {
          default = pkgs.callPackage ./. {
            inherit pkgs;
            appstream-data = nixos-appstream-data.packages.${system}.appstream-data-all;
          };
        }
      );
    };
}
