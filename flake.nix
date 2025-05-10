{
  description = "JAWS a cli tool for managing secrets on major cloud providors.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    nix-formatter-pack.url = "github:Gerschtli/nix-formatter-pack";
    nix-formatter-pack.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    { self
    , nixpkgs
    , nix-formatter-pack
    , ...
    }:
    let
      inherit (nixpkgs) lib;

      allSystems = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      # A function that provides a system-specific Nixpkgs for the desired systems
      forAllSystems = f:
        allSystems (system:
          f {
            pkgs = import nixpkgs { inherit system; };
          });
    in
    {
      packages = forAllSystems ({ pkgs }:
        let
          utils = import ./nix/utils.nix { inherit pkgs lib self; };
          jaws =
            { source
            , version ? (utils.mkVersion "jaws" source)
            ,
            }:
            pkgs.buildGoModule rec {
              pname = "jaws";
              src = source;
              inherit version;
              ldflags = [
                "-s"
                "-w"
                "-X 'main.Version=${version}'"
                "-X 'main.Date=${utils.getLastModifiedDate source}'"
              ];
              vendorHash = "sha256-xBCJStWP7RbyEUSZxHa9NPiO4V3UjEB7LxtnTe4t8tA=";
              env = {
                CGO_ENABLED = 1;
              };

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
        in
        rec {
          ################
          ### Packages ###
          ################
          bin = jaws { source = lib.cleanSource ./.; };
          docker = utils.mkContainerImage "jaws" "latest" bin;
          default = bin;
        });
      devShells = forAllSystems ({ pkgs }: {
        default = pkgs.mkShell {
          name = "jaws";
          buildInputs = with pkgs;
            [
              go
              gopls
              gotools
              go-tools
              goreleaser
              just
              vhs
            ]
            ++ lib.optional pkgs.stdenv.isLinux bitwarden-cli;
          CGO_ENABLED = 1;
        };
      });

      # sh: nix fmt
      formatter = allSystems (
        system:
        nix-formatter-pack.lib.mkFormatter {
          pkgs = nixpkgs.legacyPackages.${system};
          config.tools = {
            alejandra.enable = true;
            deadnix.enable = true;
            nixpkgs-fmt.enable = true;
            statix.enable = true;
          };
        }
      );

      hydraJobs."jaws-binary" = forAllSystems ({ pkgs }: self.packages.${pkgs.stdenv.system}.bin);
    };
}
