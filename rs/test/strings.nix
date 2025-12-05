/**
  String manipulation functions.
*/
{ lib }:
let
  inherit (builtins) length;
in
rec {
  /**
    Concatenate a list of strings.

    # Example

    ```nix
    concatStrings ["foo" "bar"]
    => "foobar"
    ```
  */
  concatStrings = builtins.concatStringsSep "";

  /**
    Map a function over a list and concatenate the resulting strings.

    # Arguments

    - `f`: Function to map
    - `list`: List of values

    # Example

    ```nix
    concatMapStrings (x: "a" + x) ["foo" "bar"]
    => "afooabar"
    ```
  */
  concatMapStrings = f: list: concatStrings (map f list);

  /**
    Determine whether a string has given prefix.

    # Arguments

    - `pref`: Prefix to check for
    - `str`: Input string

    # Example

    ```nix
    hasPrefix "foo" "foobar"
    => true
    hasPrefix "foo" "barfoo"
    => false
    ```
  */
  hasPrefix = pref: str: builtins.substring 0 (builtins.stringLength pref) str == pref;
}
