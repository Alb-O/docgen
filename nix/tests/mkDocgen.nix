# Tests for mkDocgen entry point
# Since mkDocgen requires pkgs, we test the structural aspects
# and verify the commands structure is correctly passed through
{
  lib,
  docgenLib,
  ...
}:
let
  # Mock manifest for testing
  sampleManifest = {
    files = {
      title = "File Reference";
      titleLevel = 1;
      sections = [
        {
          name = "Core";
          files = [
            "default.nix"
            "api.nix"
          ];
        }
      ];
    };
    methods = {
      title = "API Methods";
      titleLevel = 1;
      sections = [
        {
          file = "api.nix";
          heading = "Core API";
        }
      ];
    };
    options = {
      title = "Options";
      anchorPrefix = "opt-";
    };
  };

  # Minimal manifest with only files
  filesOnlyManifest = {
    files = {
      title = "Files";
      sections = [
        {
          name = "Main";
          files = [ "main.nix" ];
        }
      ];
    };
  };

  # Manifest with only methods
  methodsOnlyManifest = {
    methods = {
      title = "Methods";
      sections = [
        { file = "lib.nix"; }
      ];
    };
  };
in
{
  # Test manifest loading (path simulation)
  mkDocgen."test manifest attrset is used directly" = {
    expr =
      let
        loaded = if builtins.isPath sampleManifest then import sampleManifest else sampleManifest;
      in
      loaded.files.title;
    expected = "File Reference";
  };

  # Test that filesCommands are generated when files config present
  mkDocgen."test files config generates commands" = {
    expr =
      let
        cmd = docgenLib.generateFilesCommands {
          filesConfig = sampleManifest.files;
        };
      in
      cmd != null && lib.hasInfix "File Reference" cmd;
    expected = true;
  };

  # Test that methodsCommands are generated when methods config present
  mkDocgen."test methods config generates commands" = {
    expr =
      let
        cmd = docgenLib.generateMethodsCommands {
          methodsConfig = sampleManifest.methods;
        };
      in
      cmd != null && lib.hasInfix "API Methods" cmd;
    expected = true;
  };

  # Test that optionsCommands are generated when options config present
  mkDocgen."test options config generates commands" = {
    expr =
      let
        cmd = docgenLib.generateOptionsCommands {
          optionsConfig = sampleManifest.options;
        };
      in
      cmd != null && lib.hasInfix "Options" cmd;
    expected = true;
  };

  # Test files-only manifest produces only files commands
  mkDocgen."test files-only manifest works" = {
    expr =
      let
        filesCmd = docgenLib.generateFilesCommands {
          filesConfig = filesOnlyManifest.files;
        };
      in
      lib.hasInfix "# Files" filesCmd && lib.hasInfix "main.nix" filesCmd;
    expected = true;
  };

  # Test methods-only manifest produces only methods commands
  mkDocgen."test methods-only manifest works" = {
    expr =
      let
        methodsCmd = docgenLib.generateMethodsCommands {
          methodsConfig = methodsOnlyManifest.methods;
        };
      in
      lib.hasInfix "# Methods" methodsCmd && lib.hasInfix "lib.nix" methodsCmd;
    expected = true;
  };

  # Test default output file names
  mkDocgen."test default output files" = {
    expr =
      let
        outputFiles = {
          files = "files.md";
          methods = "methods.md";
          options = "options.md";
        };
        filesOutput = outputFiles.files or "files.md";
        methodsOutput = outputFiles.methods or "methods.md";
        optionsOutput = outputFiles.options or "options.md";
      in
      filesOutput == "files.md" && methodsOutput == "methods.md" && optionsOutput == "options.md";
    expected = true;
  };

  # Test custom output file names
  mkDocgen."test custom output files" = {
    expr =
      let
        outputFiles = {
          files = "api-files.md";
          methods = "api-methods.md";
          options = "api-options.md";
        };
        filesOutput = outputFiles.files or "files.md";
      in
      filesOutput == "api-files.md";
    expected = true;
  };

  # Test referenceDir path computation
  mkDocgen."test empty referenceDir produces empty path" = {
    expr =
      let
        referenceDir = "";
        refPath = if referenceDir == "" then "" else "${referenceDir}/";
      in
      refPath == "";
    expected = true;
  };

  mkDocgen."test non-empty referenceDir produces path with slash" = {
    expr =
      let
        referenceDir = "api";
        refPath = if referenceDir == "" then "" else "${referenceDir}/";
      in
      refPath == "api/";
    expected = true;
  };

  # Test localPaths defaults
  mkDocgen."test localPaths defaults" = {
    expr =
      let
        localPaths = { };
        localSiteDir = localPaths.site or "./docs";
        localSrcDir = localPaths.src or "./src";
      in
      localSiteDir == "./docs" && localSrcDir == "./src";
    expected = true;
  };

  # Test localPaths custom values
  mkDocgen."test localPaths custom values" = {
    expr =
      let
        localPaths = {
          site = "./documentation";
          src = "./source";
        };
        localSiteDir = localPaths.site or "./docs";
        localSrcDir = localPaths.src or "./src";
      in
      localSiteDir == "./documentation" && localSrcDir == "./source";
    expected = true;
  };

  # Test anchorPrefix is passed to methods commands
  mkDocgen."test anchorPrefix in methods commands" = {
    expr =
      let
        cmd = docgenLib.generateMethodsCommands {
          methodsConfig = sampleManifest.methods;
          prefix = "mylib";
        };
      in
      lib.hasInfix ''--prefix "mylib"'' cmd;
    expected = true;
  };

  # Test manifest with sections containing exports
  mkDocgen."test methods with exports filter" = {
    expr =
      let
        methodsWithExports = {
          title = "Methods";
          sections = [
            {
              file = "api.nix";
              exports = [
                "foo"
                "bar"
                "baz"
              ];
            }
          ];
        };
        cmd = docgenLib.generateMethodsCommands {
          methodsConfig = methodsWithExports;
        };
      in
      lib.hasInfix "--export foo,bar,baz" cmd;
    expected = true;
  };

  # Test manifest with multiple sections
  mkDocgen."test multiple sections in files config" = {
    expr =
      let
        multiSectionManifest = {
          title = "Reference";
          sections = [
            {
              name = "Core";
              files = [ "core.nix" ];
            }
            {
              name = "Utils";
              files = [ "utils.nix" ];
            }
            {
              name = "Extras";
              files = [ "extras.nix" ];
            }
          ];
        };
        cmd = docgenLib.generateFilesCommands {
          filesConfig = multiSectionManifest;
        };
      in
      lib.hasInfix "## Core" cmd && lib.hasInfix "## Utils" cmd && lib.hasInfix "## Extras" cmd;
    expected = true;
  };

  # Test files with fallback descriptions
  mkDocgen."test files with fallback descriptions" = {
    expr =
      let
        filesWithFallback = {
          title = "Files";
          sections = [
            {
              name = "Internal";
              files = [
                {
                  name = "internal.nix";
                  fallback = "Internal implementation details.";
                }
                {
                  name = "private.nix";
                  fallback = "Private utilities.";
                }
              ];
            }
          ];
        };
        cmd = docgenLib.generateFilesCommands {
          filesConfig = filesWithFallback;
        };
      in
      lib.hasInfix "Internal implementation details." cmd
      && lib.hasInfix "Private utilities." cmd
      # Should use echo for fallback, not nixdoc
      && lib.hasInfix ''echo "Internal implementation details."'' cmd;
    expected = true;
  };

  # Test null handling for optional configs
  mkDocgen."test null files config check" = {
    expr =
      let
        manifest = {
          methods = {
            title = "Methods";
            sections = [ ];
          };
        };
        filesConfig = manifest.files or null;
      in
      filesConfig == null;
    expected = true;
  };

  mkDocgen."test present files config check" = {
    expr =
      let
        manifest = {
          files = {
            title = "Files";
            sections = [ ];
          };
        };
        filesConfig = manifest.files or null;
      in
      filesConfig != null && filesConfig.title == "Files";
    expected = true;
  };
}
