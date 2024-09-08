{
  description = "JAWS a cli tool for managing secrets on major cloud providors.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      inherit (nixpkgs) lib;
      pkgs = nixpkgs.legacyPackages.${system};

      utils = import ./nix/utils.nix { inherit pkgs lib self; };
    in {
      packages = rec {
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

        ################
        ### Packages ###
        ################
        bin = jaws { source = lib.cleanSource self; };
        docker = utils.mkContainerImage "jaws" "latest" bin;
      };
      devShells = {
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
      };
      # Default package
      defaultPackage = self.packages.${system}.bin;
    }
  );
}
