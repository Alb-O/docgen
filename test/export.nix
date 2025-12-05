/**
  Test file for --export flag functionality.
*/
let
  /**
    A documented function that will be exported.

    # Example

    ```nix
    exportedFunc "test"
    # => "test-result"
    ```

    # Arguments

    arg
    : The input argument.
  */
  exportedFunc = arg: "${arg}-result";

  /**
    Another documented function.

    # Arguments

    x
    : First value.

    y
    : Second value.
  */
  anotherExported = x: y: x + y;

  # This one has no doc comment
  undocumented = x: x;

  /**
    This function is documented but won't be exported.

    # Arguments

    input
    : Some input value.
  */
  notExported = input: input * 2;

in
{
  # The file returns an attrset that doesn't include exportedFunc or anotherExported
  # But with --export, we can still document those let bindings
  inherit notExported;
  other = "value";
}
