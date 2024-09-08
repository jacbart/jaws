{
  description = "JAWS a cli tool for managing secrets on major cloud providors.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs, ... }:
    let
      inherit (nixpkgs) lib;

      allSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      # A function that provides a system-specific Nixpkgs for the desired systems
      forAllSystems = f: nixpkgs.lib.genAttrs allSystems (system: f {
        pkgs = import nixpkgs { inherit system; };
      });

    in {
      packages = forAllSystems ({ pkgs }: let
        utils = import ./nix/utils.nix { inherit pkgs lib self; };
        jaws = { source, version ? (utils.mkVersion "jaws" source) }: pkgs.buildGoModule rec {
          pname = "jaws";
          src = source;
          inherit version;
          ldflags = [
            "-s" "-w"
            "-X 'main.Version=${version}'"
            "-X 'main.Date=${utils.getLastModifiedDate source}'"
          ];
          vendorHash = null;

          meta = with pkgs.lib; {
            mainProgram = pname;
            description = "JAWS a cli tool for managing secrets on major cloud providers.";
            longDescription = ''
              JAWS was insired by AWS's bad UX for their secrets manager. The tool
              utilizes a fuzzy finder to make filtering and selecting multiple
              secrets easy. Once you have the secrets downloaded just edit them
              and run the push command to update them.
            '';
            homepage = "https://github.com/jacbart/${pname}";
            license = licenses.mpl20;
            maintainers = with maintainers; [ jacbart ];
            platforms = platforms.unix;
          };
        };
      in {
        ################
        ### Packages ###
        ################
        bin = jaws { source = lib.cleanSource self; };
        # docker = utils.mkContainerImage "jaws" "latest" bin;
      });
      devShells = forAllSystems ({ pkgs }: {
        default = pkgs.mkShell {
          name = "jaws";
          buildInputs = with pkgs; [
            bitwarden-cli
            figlet
            go
            gopls
            gotools
            go-tools
            goreleaser
            just
            vhs
          ];
          shellHook = ''
            figlet -k "JAWS env"
          '';
        };
      });
      defaultPackage = forAllSystems ({ pkgs }: self.packages.${pkgs.stdenv.system}.bin);
      hydraJobs."jaws-binary" = self.defaultPackage;
    };
}
