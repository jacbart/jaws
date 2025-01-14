{ pkgs
, lib
, self
,
}:
let
  inherit (pkgs.dockerTools) buildImage;
in
rec {
  # get last modifidated date
  getLastModifiedDate = input:
    let
      date =
        input.lastModifiedDate or self.sourceInfo.lastModifiedDate;

      year = builtins.substring 0 4 date;
      month = builtins.substring 4 2 date;
      day = builtins.substring 6 2 date;
    in
    "${year}-${month}-${day}";

  # generate a version from the flake.lock
  mkVersion = name: input:
    let
      inputs = (builtins.fromJSON (builtins.readFile ../flake.lock)).nodes;

      ref =
        builtins.trace "ref value: ${
        if lib.hasAttrByPath [name "original" "ref"] inputs
        then inputs.${name}.original.ref
        else ""
      }"
          (
            if lib.hasAttrByPath [ name "original" "ref" ] inputs
            then inputs.${name}.original.ref
            else ""
          );

      version =
        let
          version' =
            builtins.match
              "[[:alpha:]]*[-._]?([0-9]+(\.[0-9]+)*)+"
              ref;
        in
        if lib.isList version'
        then lib.head version'
        else if input ? lastModifiedDate && input ? shortRev
        then "${lib.substring 0 10 (getLastModifiedDate input)}_${input.shortRev}"
        else "${lib.substring 0 10 (getLastModifiedDate self.sourceInfo)}_rc";
    in
    version;

  # build docker image
  mkContainerImage = name: tag: mainPkg:
    let
      container = buildImage {
        inherit name;
        inherit tag;
        copyToRoot = pkgs.buildEnv {
          name = "image-root";
          paths = [ mainPkg ];
          pathsToLink = [ "/bin" ];
        };
        config = {
          Entrypoint = "/bin/${name}";
        };
        created = "now";
      };
    in
    container;
}
