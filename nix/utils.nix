{ lib, self }:

rec {
  getLastModifiedDate = input: let
    date = if input ? lastModifiedDate
      then input.lastModifiedDate
      else self.sourceInfo.lastModifiedDate;

    year = builtins.substring 0 4 date;
    month = builtins.substring 5 2 date;
    day = builtins.substring 8 2 date;
  in
    "${year}-${month}-${day}";

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
        "${lib.substring 0 8 (getLastModifiedDate self.sourceInfo)}_developer"
      );
  in
    version;
}