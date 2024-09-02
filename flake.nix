{
  description = "JAWS a cli tool for managing secrets on major cloud providors.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    jaws-stable.url = "git+ssh://git@github.com/jacbart/jaws";
  };

  outputs = { self, nixpkgs, flake-utils, jaws-stable, ... }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      inherit (nixpkgs) lib;
      pkgs = nixpkgs.legacyPackages.${system};

      utils = import ./nix/utils.nix { inherit pkgs lib self; };
    in {
      packages = rec {
        jaws = { source }: pkgs.buildGoModule rec {
          pname = "jaws";
          src = source;
          version = utils.mkVersion pname source;
          ldflags = [
            "-s" "-w"
            "-X 'main.Version=${version}'"
            "-X 'main.Date=${utils.getLastModifiedDate source}'"
          ];
          vendorHash = null;

          meta = with pkgs.lib; {
            mainProgram = "jaws";
            description = "JAWS a cli tool for managing secrets on major cloud providors.";
            longDescription = ''
              JAWS was insired by AWS's bad UX for their secrets manager. The tool
              utilizes a fuzzy finder to make filtering and selecting multiple
              secrets easy. Once you have the secrets downloaded just edit them
              and run the push command to update them.
            '';
            homepage = "https://github.com/jacbart/jaws";
            license = licenses.mpl20;
            maintainers = with maintainers; [ jacbart ];
            platforms = platforms.all;
          };
        };

        ################
        ### Packages ###
        ################
        stable = jaws-stable;
        rc = jaws { source = lib.cleanSource self; };
        docker = utils.mkDocker "jaws" "latest" rc;
    };
    devShells = {
      default = pkgs.mkShell {
        name = "jaws";
        buildInputs = with pkgs; [
          go
          gopls
          gotools
          go-tools
          goreleaser
          just
          bitwarden-cli
          vhs
        ];
      };
    };

    defaultPackage = self.packages.${system}.rc;
  });
}
