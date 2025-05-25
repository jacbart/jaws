self: { config
      , pkgs
      , lib
      , ...
      }:
with lib; let
  cfg = config.programs.jaws;
  defaultPackage = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
  utils = import ./nix/utils.nix { inherit pkgs lib self; };
  hclFormat = utils.hcl { };
in
{
  options.programs.jaws = with types; {
    enable = mkEnableOption "Whether or not to enable jaws";
    package = mkOption {
      type = with types; nullOr package;
      default = defaultPackage;
      defaultText = literalExpression "inputs.jaws.pacakges.${pkgs.stdenv.hostPlatform.system}.default";
      description = ''
        The jaws package to use.

        By default, this option will use the `packages.default` as exposed by this flake.
      '';
    };
    settings = lib.mkOption {
      inherit (hclFormat) type;
      default = {
        general = {
          default_profile = "default";
          disable_auto_detect = true;
          editor = "hx";
          gh_token = "";
          repo_warn = false;
          safe_mode = true;
          secrets_path = "~/.config/jaws/secrets";
        };
      };
      example = builtins.from;
      description = ''
        Configuration written to {file}`$XDG_CONFIG_HOME/jaws/jaws.conf`
      '';
    };
  };

  config = mkIf cfg.enable {
    home.packages = [
      cgf.package
    ];

    xdg.configFile."jaws/jaws.conf" = lib.mkIf (cfg.settings != { }) {
      source = hclFormat.generate "jaws.conf" cfg.settings;
    };
  };
}
