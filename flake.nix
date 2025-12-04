{
  description = "Generic documentation generator for Nix projects using nixdoc and mdbook";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nixdoc.url = "github:Alb-O/nixdoc/feat/render-options";
    nixdoc.inputs.nixpkgs.follows = "nixpkgs";
    nix-unit.url = "github:nix-community/nix-unit";
    nix-unit.inputs.nixpkgs.follows = "nixpkgs";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      nixdoc,
      nix-unit,
      treefmt-nix,
    }:
    let
      lib = nixpkgs.lib;

      # Systems to support
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      forAllSystems = f: lib.genAttrs systems (system: f system);

      # Core library - pure functions, no pkgs needed
      coreLib = import ./src/lib.nix { inherit lib; };

      # Schema types for documentation
      schema = import ./src/schema.nix { inherit lib; };
    in
    {
      # Export schema types for consumers to reference
      inherit schema;

      # Export core library functions
      lib = coreLib;

      # Per-system outputs
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          # mdformat with standard plugins
          mdformat = pkgs.mdformat.withPlugins (
            ps: with ps; [
              mdformat-gfm
              mdformat-frontmatter
              mdformat-footnote
            ]
          );

          # mdbook for building documentation sites
          mdbook = pkgs.mdbook;
        }
      );

      # Checks (tests and formatting)
      checks = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          treefmtEval = treefmt-nix.lib.evalModule pkgs {
            projectRootFile = "flake.nix";
            programs.nixfmt.enable = true;
            settings.formatter.mdformat = {
              command = lib.getExe self.packages.${system}.mdformat;
              includes = [ "*.md" ];
            };
          };
        in
        {
          formatting = treefmtEval.config.build.check self;
          nix-unit =
            pkgs.runCommand "nix-unit-tests"
              {
                nativeBuildInputs = [ nix-unit.packages.${system}.default ];
              }
              ''
                export HOME=$TMPDIR
                nix-unit --expr 'import ${self}/tests { lib = import ${nixpkgs}/lib; }'
                touch $out
              '';
        }
      );

      # Formatter
      formatter = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          treefmtEval = treefmt-nix.lib.evalModule pkgs {
            projectRootFile = "flake.nix";
            programs.nixfmt.enable = true;
            settings.formatter.mdformat = {
              command = lib.getExe self.packages.${system}.mdformat;
              includes = [ "*.md" ];
            };
          };
        in
        treefmtEval.config.build.wrapper
      );

      # Main entry point: create a docgen instance for a project
      # Usage: docgen.mkDocgen { inherit pkgs; manifest = ./docs.nix; srcDir = ./src; siteDir = ./docs; }
      mkDocgen =
        {
          pkgs,
          lib ? pkgs.lib,
          # Documentation manifest (path or attrset)
          manifest,
          # Source directory containing .nix files
          srcDir,
          # Site directory containing mdbook structure (book.toml, src/, etc.)
          siteDir ? null,
          # Optional: extra files to copy into site (e.g., { "README.md" = ./README.md; })
          extraFiles ? { },
          # Optional: custom nixdoc package
          nixdocPkg ? nixdoc.packages.${pkgs.system}.default,
          # Optional: custom mdformat package
          mdformatPkg ? self.packages.${pkgs.system}.mdformat,
          # Optional: custom mdbook package
          mdbookPkg ? self.packages.${pkgs.system}.mdbook,
          # Optional: options JSON file for options.md generation
          optionsJson ? null,
          # Optional: prefix for function anchors (e.g., "imp")
          anchorPrefix ? "",
          # Optional: project name for derivation naming
          name ? "docs",
          # Optional: subdirectory within site/src for generated reference docs
          # Set to "" to place directly in site/src, or e.g. "api" for site/src/api/
          referenceDir ? "reference",
          # Optional: relative paths for local dev scripts (serve/build)
          # These are the paths used when running `nix run .#docs` from project root
          localPaths ? {
            site = "./docs";
            src = "./src";
          },
          # Optional: custom output file names
          outputFiles ? {
            files = "files.md";
            methods = "methods.md";
            options = "options.md";
          },
        }:
        import ./src/mkDocgen.nix {
          inherit
            lib
            pkgs
            manifest
            srcDir
            siteDir
            extraFiles
            nixdocPkg
            mdformatPkg
            mdbookPkg
            optionsJson
            anchorPrefix
            name
            referenceDir
            localPaths
            outputFiles
            ;
          docgenLib = coreLib;
        };
    };
}
