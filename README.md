# docgen

Documentation generator for Nix projects. Extracts documentation from file comments, function doc comments, and NixOS-style module options, producing markdown suitable for [mdbook](https://rust-lang.github.io/mdBook/) sites.

## Structure

```
docgen/
├── rs/       # Rust CLI that parses Nix and emits markdown
├── nix/      # Nix library for orchestrating doc generation
└── tests/    # nix-unit tests
```

## Quick start

```nix
{
  inputs.docgen.url = "github:imp-nix/imp.docgen";
}
```

Create a manifest defining what to document:

```nix
# docs/manifest.nix
{
  methods = {
    title = "API Methods";
    sections = [
      { file = "api.nix"; }
      { file = "lib.nix"; heading = "Utilities"; exports = [ "helper" ]; }
    ];
  };

  options = {
    title = "Module Options";
  };
}
```

Wire it up:

```nix
{
  perSystem = { pkgs, ... }:
    let
      dg = docgen.mkDocgen {
        inherit pkgs;
        manifest = ./docs/manifest.nix;
        srcDir = ./src;
        siteDir = ./docs;
        name = "myproject";
      };
    in {
      packages.docs = dg.docs;
      apps.docs.program = dg.serveDocsScript;
    };
}
```

## mkDocgen

Required arguments:

| Argument   | Description                               |
| ---------- | ----------------------------------------- |
| `pkgs`     | Nixpkgs package set                       |
| `manifest` | Path or attrset defining what to document |
| `srcDir`   | Source directory containing .nix files    |

Optional arguments:

| Argument       | Default       | Description                                |
| -------------- | ------------- | ------------------------------------------ |
| `siteDir`      | `null`        | mdbook site directory (contains book.toml) |
| `extraFiles`   | `{}`          | Extra files to copy into site              |
| `optionsJson`  | `null`        | JSON file for options.md generation        |
| `anchorPrefix` | `""`          | Prefix for function anchors                |
| `name`         | `"docs"`      | Project name for derivation                |
| `referenceDir` | `"reference"` | Subdirectory for generated reference docs  |

Returns:

| Attribute         | Description                               |
| ----------------- | ----------------------------------------- |
| `docs`            | Built mdbook site derivation              |
| `apiReference`    | Generated markdown files only             |
| `serveDocsScript` | Script for local serving with live reload |
| `buildDocsScript` | Script for local building                 |

## Manifest schema

### methods

Function documentation from `/** ... */` doc comments:

```nix
{
  methods = {
    title = "API Methods";
    sections = [
      { file = "api.nix"; }
      { file = "lib.nix"; heading = "Utilities"; exports = [ "fn1" "fn2" ]; }
    ];
  };
}
```

### files

File-level descriptions from `# ...` comments at the top of files:

```nix
{
  files = {
    title = "File Reference";
    sections = [
      {
        name = "Core";
        files = [
          "default.nix"
          { name = "lib.nix"; fallback = "Internal utilities."; }
        ];
      }
    ];
  };
}
```

### options

Module options from JSON (use `docgen.lib.optionsToJson` to generate):

```nix
{
  options = {
    title = "Module Options";
    anchorPrefix = "opt-";
  };
}
```

## Writing doc comments

Function-level:

````nix
{
  /**
    Short description.

    # Arguments

    - `arg1` (type): description

    # Example

    ```nix
    myFn "foo" { }
    => { result = "foo"; }
    ```
  */
  myFn = arg1: arg2: { ... };
}
````

File-level (before any code):

```nix
# Brief description of what this file provides.
{ lib }:
{ ... }
```

## Development

```sh
nix build             # build docgen CLI
nix flake check       # run all checks
nix fmt               # format everything
nix develop           # shell with Rust toolchain
```

Rust tests with snapshot updates:

```sh
cd rs && cargo insta test
cargo insta review
```

## Attribution

Rust component originally developed by @infinisil as [nixdoc](https://github.com/nix-community/nixdoc)

## License

[GPL-3.0](LICENSE)
