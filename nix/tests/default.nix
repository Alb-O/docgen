# Unit tests for docgen
{ lib }:
let
  docgenLib = import ../lib.nix { inherit lib; };
  schema = import ../schema.nix { inherit lib; };
  args = {
    inherit lib docgenLib schema;
  };
in
(import ./lib.nix args) // (import ./schema.nix args) // (import ./mkDocgen.nix args)
