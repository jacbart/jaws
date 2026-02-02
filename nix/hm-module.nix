{
  config,
  lib,
  pkgs,
  ...
}:

with lib;

let
  cfg = config.programs.jaws;
in
{
  options.programs.jaws = {
    enable = mkEnableOption "jaws secrets manager";

    package = mkOption {
      type = types.package;
      default = pkgs.callPackage ../default.nix { };
      description = "The jaws package to install.";
    };
  };

  config = mkIf cfg.enable {
    home.packages = [ cfg.package ];
  };
}
