{ pkgs, lib, self }: let
  inherit (pkgs.dockerTools) buildImage;
in
rec {
  # get last modifidated date
  getLastModifiedDate = input: let
    date = if input ? lastModifiedDate
      then input.lastModifiedDate
      else self.sourceInfo.lastModifiedDate;

    year = builtins.substring 0 4 date;
    month = builtins.substring 5 2 date;
    day = builtins.substring 8 2 date;
  in
    "${year}-${month}-${day}";

  # generate a version from the flake.lock
  mkVersion = name: input: let
    inputs = (builtins.fromJSON (builtins.readFile ../flake.lock)).nodes;

    ref = builtins.trace "ref value: ${if lib.hasAttrByPath [name "original" "ref"] inputs
      then inputs.${name}.original.ref
      else ""}" (
      if lib.hasAttrByPath [name "original" "ref"] inputs
      then inputs.${name}.original.ref
      else "");

    version = let
      version' =
        builtins.match
        "[[:alpha:]]*[-._]?([0-9]+(\.[0-9]+)*)+"
        ref;
    in
      if lib.isList version'
      then lib.head version'
      else if input ? lastModifiedDate && input ? shortRev
      then (
        "${lib.substring 0 8 (getLastModifiedDate input)}_${input.shortRev}"
      ) else (
        "${lib.substring 0 8 (getLastModifiedDate self.sourceInfo)}_rc"
      );
  in
    version;

  # build docker image
  mkDocker = name: tag: mainPkg: let 
    pkgImage = buildImage {
      name = name;
      tag = tag;
      copyToRoot = pkgs.buildEnv {
        name = "image-root";
        paths = [ mainPkg ];
        pathsToLink = [ "/bin" ];
      };
      config = {
        Cmd = "/bin/${name}";
      };
    };
  in
    pkgImage;
}
