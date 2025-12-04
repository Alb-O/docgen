# Unit tests for docgen
{ lib }:
let
  docgenLib = import ../src/lib.nix { inherit lib; };
  schema = import ../src/schema.nix { inherit lib; };
  args = {
    inherit lib docgenLib schema;
  };
in
(import ./lib.nix args) // (import ./schema.nix args)
