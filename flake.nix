{
  description = "jaws flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }: 
  let
    supportedSystems = [ "x86_64-linux" "x86_64-darwin" "aarch64-linux" "aarch64-darwin" ];
    forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    nixpkgsFor = forAllSystems (system: import nixpkgs { inherit system; });
  in {
    packages = forAllSystems (system:
      let
        pkgs = nixpkgsFor.${system};
      in {
        jaws = pkgs.buildGoModule rec {
          pname = "jaws";
          src = pkgs.lib.cleanSource ./.;
          version = "1.0.6-rc";
          ldflags = [
            "-s" "-w"
            "-X 'main.Version=${version}'"
            "-X 'main.Date=20xx-xx-xx'"
          ];
          # vendorHash = "";
          vendorHash = pkgs.lib.fakeHash;

          meta = with pkgs.lib; {
            mainProgram = "jaws";
            description = "JAWS is a cli tool for managing secrets on major cloud providors.";
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
    });
    devShells = forAllSystems (system:
      let
        pkgs = nixpkgsFor.${system};
      in {
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
    });

    defaultPackage = forAllSystems (system: self.packages.${system}.jaws);
  };
}
