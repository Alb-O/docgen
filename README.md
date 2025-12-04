# Docgen

A generic documentation generator for Nix projects using [nixdoc](https://github.com/nix-community/nixdoc) and [mdbook](https://rust-lang.github.io/mdBook/).

## Overview

Docgen provides an opinionated but flexible framework for generating API reference documentation from Nix source files. It extracts documentation from:

- **File doc comments** - Top-level comments describing what a file does
- **Function doc comments** - nixdoc-style comments on functions
- **Module options** - NixOS/home-manager style option definitions

The generated markdown is designed to integrate with mdbook-based documentation sites.

## Quick Start

### 1. Add docgen as a flake input

```nix
{
  inputs = {
    docgen.url = "github:Alb-O/docgen";
    # ...
  };
}
```

### 2. Create a documentation manifest

Create a `_docs.nix` file (or similar) describing what to document:

```nix
{
  files = {
    title = "File Reference";
    sections = [
      {
        name = "Core";
        files = [
          "default.nix"
          "api.nix"
          { name = "lib.nix"; fallback = "Internal utilities."; }
        ];
      }
    ];
  };

  methods = {
    title = "API Methods";
    sections = [
      { file = "api.nix"; }
      { file = "lib.nix"; heading = "Utilities"; exports = [ "helper" "util" ]; }
    ];
  };

  options = {
    title = "Module Options";
    anchorPrefix = "opt-";
  };
}
```

### 3. Create a Docgen instance

```nix
{
  perSystem = { pkgs, self', ... }:
    let
      dg = docgen.mkDocgen {
        inherit pkgs;
        manifest = ./src/_docs.nix;
        srcDir = ./src;
        siteDir = ./site;
        name = "myproject";
        # Optional: for options.md generation
        optionsJson = self'.packages.options-json;
      };
    in {
      packages.docs = dg.docs;
      apps.docs.program = dg.serveDocsScript;
      apps.build-docs.program = dg.buildDocsScript;
    };
}
```

## API Reference

### `mkDocgen`

Creates a `docgen` instance with scripts and derivations for generating documentation.

#### Required Arguments

| Argument   | Type            | Description                                  |
| ---------- | --------------- | -------------------------------------------- |
| `pkgs`     | pkgs            | Nixpkgs package set                          |
| `manifest` | path or attrset | Documentation manifest (see Manifest Schema) |
| `srcDir`   | path            | Source directory containing .nix files       |

#### Optional Arguments

| Argument       | Type         | Default                                                                   | Description                                                            |
| -------------- | ------------ | ------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `siteDir`      | path or null | `null`                                                                    | mdbook site directory (contains book.toml)                             |
| `extraFiles`   | attrset      | `{}`                                                                      | Extra files to copy into site (e.g., `{ "README.md" = ./README.md; }`) |
| `optionsJson`  | path or null | `null`                                                                    | JSON file with options for options.md generation                       |
| `anchorPrefix` | string       | `""`                                                                      | Prefix for function anchors (e.g., "mylib")                            |
| `name`         | string       | `"docs"`                                                                  | Project name for derivation naming                                     |
| `referenceDir` | string       | `"reference"`                                                             | Subdirectory within `site/src/` for generated docs                     |
| `localPaths`   | attrset      | `{ site = "./site"; src = "./src"; }`                                     | Paths used by serve/build scripts                                      |
| `outputFiles`  | attrset      | `{ files = "files.md"; methods = "methods.md"; options = "options.md"; }` | Output file names                                                      |
| `nixdocPkg`    | package      | nixdoc from input                                                         | Custom nixdoc package                                                  |
| `mdformatPkg`  | package      | mdformat with plugins                                                     | Custom mdformat package                                                |
| `mdbookPkg`    | package      | mdbook from nixpkgs                                                       | Custom mdbook package                                                  |

#### Returns

| Attribute            | Type       | Description                                   |
| -------------------- | ---------- | --------------------------------------------- |
| `docs`               | derivation | Built mdbook site with generated reference    |
| `apiReference`       | derivation | Just the generated markdown files             |
| `serveDocsScript`    | path       | Script to serve docs locally with live reload |
| `buildDocsScript`    | path       | Script to build docs locally                  |
| `generateDocsScript` | path       | Low-level script to generate markdown         |
| `loadedManifest`     | attrset    | The loaded manifest for inspection            |
| `packages`           | attrset    | The underlying tool packages                  |
| `commands`           | attrset    | Generated shell commands for debugging        |

### `docgen.lib.optionsToJson`

Converts a NixOS-style options module to JSON format for nixdoc's `options` subcommand.

```nix
docgen.lib.optionsToJson {
  optionsModule = ./src/options-schema.nix;  # Or inline module
  prefix = "myApp.";                          # Optional: filter by prefix
}
```

#### Arguments

| Argument        | Type           | Default | Description                              |
| --------------- | -------------- | ------- | ---------------------------------------- |
| `optionsModule` | module         | -       | NixOS-style options module               |
| `prefix`        | string or null | `null`  | Filter options to only those with prefix |

#### Returns

JSON string suitable for passing to nixdoc's `options` subcommand.

#### Example Usage

```nix
let
  # Define your options schema
  optionsSchema = { lib, ... }: {
    options.myApp = {
      enable = lib.mkEnableOption "myApp";
      port = lib.mkOption {
        type = lib.types.port;
        default = 8080;
        description = "Port to listen on";
      };
    };
  };

  # Convert to JSON
  optionsJson = docgen.lib.optionsToJson {
    optionsModule = optionsSchema;
    prefix = "myApp.";
  };

  # Write to file for nixdoc
  optionsJsonFile = pkgs.writeText "options.json" optionsJson;
in
docgen.mkDocgen {
  # ...
  optionsJson = optionsJsonFile;
}
```

This helper handles:

- Evaluating the module with `lib.evalModules`
- Converting options to the documentation list format
- Filtering out internal/invisible options
- Filtering by prefix (if specified)
- Producing JSON in the format nixdoc expects

## Manifest Schema

The manifest defines what documentation to generate. All top-level keys are optional.

### `files`

Generates a file reference document with descriptions extracted from file-level doc comments.

```nix
{
  files = {
    title = "File Reference";        # Document title
    titleLevel = 1;                  # Heading level (1-6)
    sections = [
      {
        name = "Section Name";       # Section heading
        files = [
          "file.nix"                 # Simple: just the filename
          {
            name = "other.nix";      # Detailed: with fallback
            fallback = "Description if no doc comment";
          }
        ];
      }
    ];
  };
}
```

### `methods`

Generates function documentation using nixdoc.

```nix
{
  methods = {
    title = "API Methods";
    titleLevel = 1;
    sections = [
      {
        file = "api.nix";            # Required: source file
        heading = "API Functions";   # Optional: section heading
        exports = [ "fn1" "fn2" ];   # Optional: filter to specific functions
      }
    ];
  };
}
```

### `options`

Generates module options documentation from a JSON options file.

```nix
{
  options = {
    title = "Module Options";
    anchorPrefix = "opt-";           # Prefix for option anchors
  };
}
```

## Project Structure

docgen expects (but doesn't require) this structure:

```
your-project/
├── src/
│   ├── _docs.nix          # Documentation manifest
│   └── *.nix              # Source files to document
├── site/
│   ├── book.toml          # mdbook configuration
│   └── src/
│       ├── SUMMARY.md     # mdbook table of contents
│       └── reference/     # Generated docs go here
│           ├── files.md
│           ├── methods.md
│           └── options.md
└── flake.nix
```

### Customizing Paths

You can customize the output location and file names:

```nix
dg = docgen.mkDocgen {
  # ...
  referenceDir = "api";              # Put in site/src/api/ instead of reference/
  # referenceDir = "";               # Put directly in site/src/
  
  localPaths = {
    site = "./docs";                 # If your site is in ./docs
    src = "./lib";                   # If your source is in ./lib
  };
  
  outputFiles = {
    files = "file-reference.md";     # Custom file names
    methods = "functions.md";
    options = "configuration.md";
  };
};
```

## Writing Documentation

### File Doc Comments

Place a doc comment at the top of your file:

```nix
# This file provides the core API for managing widgets.
#
# It exports functions for creating, updating, and deleting widgets,
# as well as querying widget state.
{ lib }:
{
  # ...
}
```

### Function Doc Comments

Use nixdoc-style comments:

````nix
{
  /**
    Create a new widget with the given name.

    # Arguments

    - `name` (string): The widget name
    - `options` (attrset): Optional configuration

    # Returns

    A widget attrset.

    # Example

    ```nix
    mkWidget "foo" { color = "red"; }
    => { name = "foo"; color = "red"; }
    ```
  */
  mkWidget = name: options: { inherit name; } // options;
}
````

## Development

### Running Tests

```bash
cd tmp/docgen
nix flake check
```

### Testing with a Consumer

From a consuming project:

```bash
nix flake check --override-input docgen path:./tmp/docgen
nix run .#docs --override-input docgen path:./tmp/docgen
```

## Dependencies

- [nixdoc](https://github.com/nix-community/nixdoc) - Extracts documentation from Nix files
- [mdbook](https://rust-lang.github.io/mdBook/) - Builds the documentation site
- [mdformat](https://github.com/executablebooks/mdformat) - Formats generated markdown

## License

[MIT](LICENSE)
