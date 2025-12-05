{
  description = "Documentation generator for Nix projects";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nix-unit.url = "github:nix-community/nix-unit";
    nix-unit.inputs.nixpkgs.follows = "nixpkgs";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      nix-unit,
      treefmt-nix,
    }:
    let
      lib = nixpkgs.lib;

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      forAllSystems = f: lib.genAttrs systems (system: f system);

      # Core Nix library
      coreLib = import ./nix/lib.nix { inherit lib; };

      # Schema types
      schema = import ./nix/schema.nix { inherit lib; };

      # Rust CLI (crate is named nixdoc internally, package exposed as docgen)
      mkDocgenCli =
        { rustPlatform, ... }:
        let
          cargo = lib.importTOML ./rs/Cargo.toml;
        in
        rustPlatform.buildRustPackage {
          pname = "docgen";
          version = cargo.package.version;
          src = ./rs;
          cargoLock.lockFile = ./rs/Cargo.lock;
          postInstall = ''
            mv $out/bin/nixdoc $out/bin/docgen
          '';
        };
    in
    {
      inherit schema;

      lib = coreLib;

      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.callPackage mkDocgenCli { };

          mdformat = pkgs.mdformat.withPlugins (
            ps: with ps; [
              mdformat-gfm
              mdformat-frontmatter
              mdformat-footnote
            ]
          );

          mdbook = pkgs.mdbook;
        }
      );

      checks = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          treefmtEval = treefmt-nix.lib.evalModule pkgs {
            projectRootFile = "flake.nix";
            programs.nixfmt.enable = true;
            programs.rustfmt.enable = true;
            settings.formatter.mdformat = {
              command = lib.getExe self.packages.${system}.mdformat;
              includes = [ "*.md" ];
            };
          };
        in
        {
          formatting = treefmtEval.config.build.check self;

          docgen = self.packages.${system}.default.overrideAttrs (prev: {
            doCheck = true;
            postCheck = prev.postCheck or "" + ''
              ${pkgs.clippy}/bin/cargo-clippy --no-deps -- -D warnings
            '';
          });

          nix-unit =
            pkgs.runCommand "nix-unit-tests"
              {
                nativeBuildInputs = [ nix-unit.packages.${system}.default ];
              }
              ''
                export HOME=$TMPDIR
                nix-unit --expr 'import ${self}/nix/tests { lib = import ${nixpkgs}/lib; }'
                touch $out
              '';
        }
      );

      formatter = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          treefmtEval = treefmt-nix.lib.evalModule pkgs {
            projectRootFile = "flake.nix";
            programs.nixfmt.enable = true;
            programs.rustfmt.enable = true;
            settings.formatter.mdformat = {
              command = lib.getExe self.packages.${system}.mdformat;
              includes = [ "*.md" ];
            };
          };
        in
        treefmtEval.config.build.wrapper
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              cargo
              cargo-insta
              clippy
              rustfmt
              rustc
            ];
          };
        }
      );

      mkDocgen =
        {
          pkgs,
          lib ? pkgs.lib,
          manifest,
          srcDir,
          siteDir ? null,
          extraFiles ? { },
          docgenPkg ? self.packages.${pkgs.system}.default,
          mdformatPkg ? self.packages.${pkgs.system}.mdformat,
          mdbookPkg ? self.packages.${pkgs.system}.mdbook,
          optionsJson ? null,
          anchorPrefix ? "",
          name ? "docs",
          referenceDir ? "reference",
          localPaths ? {
            site = "./docs";
            src = "./src";
          },
          outputFiles ? {
            files = "files.md";
            methods = "methods.md";
            options = "options.md";
          },
        }:
        import ./nix/mkDocgen.nix {
          inherit
            lib
            pkgs
            manifest
            srcDir
            siteDir
            extraFiles
            mdformatPkg
            mdbookPkg
            optionsJson
            anchorPrefix
            name
            referenceDir
            localPaths
            outputFiles
            ;
          nixdocPkg = docgenPkg;
          docgenLib = coreLib;
        };
    };
}
