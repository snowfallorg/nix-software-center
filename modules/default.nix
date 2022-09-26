{ config, lib, pkgs, nix-software-center, ... }:
with lib;
let
  cfg = config.programs.nix-software-center;
  jsonFormat = pkgs.formats.json { };
in
{
  options = {
    programs.nix-software-center = {
      enable = mkEnableOption (lib.mdDoc "nix-software-center");
      systemconfig = mkOption {
        type = with types; nullOr str;
        default = null;
        example = literalExpression ''"/etc/nixos/configuration.nix"'';
        description = ''Where Nix Software Center looks for your system configuration.'';
      };
      flake = mkOption {
        type = with types; nullOr str;
        default = null;
        example = literalExpression ''"/etc/nixos/flake.nix"'';
        description = ''Where Nix Software Center looks for your system flake file.'';
      };
      flakearg = mkOption {
        type = with types; nullOr str;
        default = null;
        example = literalExpression ''user'';
        description = lib.mdDoc ''The flake argument to use when rebuilding the system. `nixos-rebuild switch --flake $\{programs.nix-software-center.flake}#$\{programs.nix-software-center.flakearg}`'';
      };
    };
  };

  config = mkMerge [
    (mkIf (cfg.enable || cfg.systemconfig != null || cfg.flake != null || cfg.flakearg != null) {
      environment.etc."nix-software-center/config.json".source = jsonFormat.generate "config.json" cfg;
    })
    (mkIf (cfg.enable) {
      environment.systemPackages = [ nix-software-center ];
    })
  ];
}
