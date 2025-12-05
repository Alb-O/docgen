# Tests for docgenLib functions
{
  lib,
  docgenLib,
  ...
}:
{
  # normalizeFileEntry tests
  normalizeFileEntry."test string entry becomes attrset with name" = {
    expr = docgenLib.normalizeFileEntry "foo.nix";
    expected = {
      name = "foo.nix";
    };
  };

  normalizeFileEntry."test attrset entry passes through" = {
    expr = docgenLib.normalizeFileEntry {
      name = "bar.nix";
      fallback = "Description";
    };
    expected = {
      name = "bar.nix";
      fallback = "Description";
    };
  };

  # escapeForShell tests
  escapeForShell."test escapes backticks" = {
    expr = docgenLib.escapeForShell "hello `world`";
    expected = "hello \\`world\\`";
  };

  escapeForShell."test escapes double quotes" = {
    expr = docgenLib.escapeForShell ''hello "world"'';
    expected = ''hello \"world\"'';
  };

  escapeForShell."test escapes dollar signs" = {
    expr = docgenLib.escapeForShell "hello $world";
    expected = "hello \\$world";
  };

  escapeForShell."test escapes multiple special chars" = {
    expr = docgenLib.escapeForShell "`echo \"$HOME\"`";
    expected = "\\`echo \\\"\\$HOME\\\"\\`";
  };

  escapeForShell."test plain string unchanged" = {
    expr = docgenLib.escapeForShell "hello world";
    expected = "hello world";
  };

  # mkHeading tests
  mkHeading."test level 1 heading" = {
    expr = docgenLib.mkHeading 1 "Title";
    expected = "# Title";
  };

  mkHeading."test level 2 heading" = {
    expr = docgenLib.mkHeading 2 "Section";
    expected = "## Section";
  };

  mkHeading."test level 3 heading" = {
    expr = docgenLib.mkHeading 3 "Subsection";
    expected = "### Subsection";
  };

  mkHeading."test level 6 heading" = {
    expr = docgenLib.mkHeading 6 "Deep";
    expected = "###### Deep";
  };

  # generateFilesCommands tests
  generateFilesCommands."test generates title and sections" = {
    expr =
      let
        cmd = docgenLib.generateFilesCommands {
          filesConfig = {
            title = "File Reference";
            titleLevel = 1;
            sections = [
              {
                name = "Core";
                files = [ "default.nix" ];
              }
            ];
          };
        };
      in
      lib.hasInfix ''echo "# File Reference"'' cmd && lib.hasInfix ''echo "## Core"'' cmd;
    expected = true;
  };

  generateFilesCommands."test uses custom title level" = {
    expr =
      let
        cmd = docgenLib.generateFilesCommands {
          filesConfig = {
            title = "Files";
            titleLevel = 2;
            sections = [
              {
                name = "Section";
                files = [ "test.nix" ];
              }
            ];
          };
        };
      in
      # Title at level 2, section at level 3, file at level 4
      lib.hasInfix ''echo "## Files"'' cmd && lib.hasInfix ''echo "### Section"'' cmd;
    expected = true;
  };

  generateFilesCommands."test file with fallback uses echo" = {
    expr =
      let
        cmd = docgenLib.generateFilesCommands {
          filesConfig = {
            title = "Files";
            sections = [
              {
                name = "Core";
                files = [
                  {
                    name = "lib.nix";
                    fallback = "Internal utilities.";
                  }
                ];
              }
            ];
          };
        };
      in
      lib.hasInfix ''echo "Internal utilities."'' cmd;
    expected = true;
  };

  generateFilesCommands."test file without fallback uses nixdoc" = {
    expr =
      let
        cmd = docgenLib.generateFilesCommands {
          filesConfig = {
            title = "Files";
            sections = [
              {
                name = "Core";
                files = [ "api.nix" ];
              }
            ];
          };
        };
      in
      lib.hasInfix ''$NIXDOC file-doc --file "$SRC_DIR/api.nix"'' cmd;
    expected = true;
  };

  # generateMethodsCommands tests
  generateMethodsCommands."test generates title" = {
    expr =
      let
        cmd = docgenLib.generateMethodsCommands {
          methodsConfig = {
            title = "API Methods";
            titleLevel = 1;
            sections = [
              { file = "api.nix"; }
            ];
          };
        };
      in
      lib.hasInfix ''echo "# API Methods"'' cmd;
    expected = true;
  };

  generateMethodsCommands."test section with heading" = {
    expr =
      let
        cmd = docgenLib.generateMethodsCommands {
          methodsConfig = {
            title = "Methods";
            sections = [
              {
                file = "utils.nix";
                heading = "Utilities";
              }
            ];
          };
        };
      in
      lib.hasInfix ''echo "## Utilities"'' cmd;
    expected = true;
  };

  generateMethodsCommands."test section without heading has no echo" = {
    expr =
      let
        cmd = docgenLib.generateMethodsCommands {
          methodsConfig = {
            title = "Methods";
            sections = [
              { file = "api.nix"; }
            ];
          };
        };
      in
      # Should have the nixdoc call but no section heading echo
      lib.hasInfix ''$NIXDOC'' cmd
      && !(lib.hasInfix ''
        echo ""
        echo "##'' cmd);
    expected = true;
  };

  generateMethodsCommands."test exports filter" = {
    expr =
      let
        cmd = docgenLib.generateMethodsCommands {
          methodsConfig = {
            title = "Methods";
            sections = [
              {
                file = "lib.nix";
                exports = [
                  "foo"
                  "bar"
                ];
              }
            ];
          };
        };
      in
      lib.hasInfix "--export foo,bar" cmd;
    expected = true;
  };

  generateMethodsCommands."test anchor prefix" = {
    expr =
      let
        cmd = docgenLib.generateMethodsCommands {
          methodsConfig = {
            title = "Methods";
            sections = [
              { file = "api.nix"; }
            ];
          };
          prefix = "mylib";
        };
      in
      lib.hasInfix ''--prefix "mylib"'' cmd;
    expected = true;
  };

  # generateOptionsCommands tests
  generateOptionsCommands."test generates options command" = {
    expr =
      let
        cmd = docgenLib.generateOptionsCommands {
          optionsConfig = {
            title = "Module Options";
            anchorPrefix = "opt-";
          };
        };
      in
      lib.hasInfix ''$NIXDOC options'' cmd
      && lib.hasInfix ''--title "Module Options"'' cmd
      && lib.hasInfix ''--anchor-prefix "opt-"'' cmd;
    expected = true;
  };

  generateOptionsCommands."test uses custom anchor prefix" = {
    expr =
      let
        cmd = docgenLib.generateOptionsCommands {
          optionsConfig = {
            title = "Options";
            anchorPrefix = "cfg-";
          };
        };
      in
      lib.hasInfix ''--anchor-prefix "cfg-"'' cmd;
    expected = true;
  };

  # mkFileCommands tests
  mkFileCommands."test generates heading for file" = {
    expr =
      let
        cmd = docgenLib.mkFileCommands {
          nixdocVar = "NIXDOC";
          srcDirVar = "SRC_DIR";
          fileLevel = 3;
          contentShift = 3;
        } "test.nix";
      in
      lib.hasInfix ''echo "### test.nix"'' cmd;
    expected = true;
  };

  mkFileCommands."test uses fallback when provided" = {
    expr =
      let
        cmd =
          docgenLib.mkFileCommands
            {
              nixdocVar = "NIXDOC";
              srcDirVar = "SRC_DIR";
              fileLevel = 3;
              contentShift = 3;
            }
            {
              name = "internal.nix";
              fallback = "Internal module.";
            };
      in
      lib.hasInfix ''echo "Internal module."'' cmd && !(lib.hasInfix "$NIXDOC" cmd);
    expected = true;
  };

  # mkSectionCommands tests
  mkSectionCommands."test generates section heading" = {
    expr =
      let
        cmd =
          docgenLib.mkSectionCommands
            {
              nixdocVar = "NIXDOC";
              srcDirVar = "SRC_DIR";
              sectionLevel = 2;
              fileLevel = 3;
              contentShift = 3;
            }
            {
              name = "Core Files";
              files = [ "api.nix" ];
            };
      in
      lib.hasInfix ''echo "## Core Files"'' cmd;
    expected = true;
  };

  # mkMethodSectionCommands tests
  mkMethodSectionCommands."test without heading" = {
    expr =
      let
        cmd = docgenLib.mkMethodSectionCommands {
          nixdocVar = "NIXDOC";
          srcDirVar = "SRC_DIR";
          sectionLevel = 2;
          prefix = "";
        } { file = "api.nix"; };
      in
      lib.hasInfix ''--file "$SRC_DIR/api.nix"'' cmd && !(lib.hasInfix "echo" cmd);
    expected = true;
  };

  mkMethodSectionCommands."test with heading" = {
    expr =
      let
        cmd =
          docgenLib.mkMethodSectionCommands
            {
              nixdocVar = "NIXDOC";
              srcDirVar = "SRC_DIR";
              sectionLevel = 2;
              prefix = "";
            }
            {
              file = "utils.nix";
              heading = "Utilities";
            };
      in
      lib.hasInfix ''echo "## Utilities"'' cmd;
    expected = true;
  };

  # optionsToJson tests
  optionsToJson."test converts simple options module to JSON" = {
    expr =
      let
        testModule =
          { lib, ... }:
          {
            options.test.enable = lib.mkEnableOption "test feature";
          };
        json = docgenLib.optionsToJson { optionsModule = testModule; };
        parsed = builtins.fromJSON json;
      in
      parsed ? "test.enable" && parsed."test.enable" ? description;
    expected = true;
  };

  optionsToJson."test filters by prefix" = {
    expr =
      let
        testModule =
          { lib, ... }:
          {
            options.foo.enable = lib.mkEnableOption "foo";
            options.bar.enable = lib.mkEnableOption "bar";
          };
        json = docgenLib.optionsToJson {
          optionsModule = testModule;
          prefix = "foo.";
        };
        parsed = builtins.fromJSON json;
      in
      parsed ? "foo.enable" && !(parsed ? "bar.enable");
    expected = true;
  };

  optionsToJson."test excludes internal options" = {
    expr =
      let
        testModule =
          { lib, ... }:
          {
            options.visible = lib.mkOption {
              type = lib.types.bool;
              default = false;
              description = "Visible option";
            };
            options.hidden = lib.mkOption {
              type = lib.types.bool;
              default = false;
              description = "Hidden option";
              internal = true;
            };
          };
        json = docgenLib.optionsToJson { optionsModule = testModule; };
        parsed = builtins.fromJSON json;
      in
      parsed ? "visible" && !(parsed ? "hidden");
    expected = true;
  };

  optionsToJson."test includes option type" = {
    expr =
      let
        testModule =
          { lib, ... }:
          {
            options.myString = lib.mkOption {
              type = lib.types.str;
              default = "hello";
              description = "A string option";
            };
          };
        json = docgenLib.optionsToJson { optionsModule = testModule; };
        parsed = builtins.fromJSON json;
      in
      parsed."myString" ? type;
    expected = true;
  };
}
