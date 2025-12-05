# Schema definitions for docgen manifests
#
# This file defines the expected structure and types for documentation manifests.
# Consumers should follow this schema when creating their manifest files.
{ lib }:
let
  inherit (lib) types mkOption;

  # A file entry can be either a string (filename) or an attrset with options
  fileEntryType = types.either types.str (
    types.submodule {
      options = {
        name = mkOption {
          type = types.str;
          description = "The filename relative to srcDir";
        };
        fallback = mkOption {
          type = types.nullOr types.str;
          default = null;
          description = "Fallback text if the file has no doc comment";
        };
      };
    }
  );

  # A section in files.md
  filesSectionType = types.submodule {
    options = {
      name = mkOption {
        type = types.str;
        description = "Section heading name";
      };
      files = mkOption {
        type = types.listOf fileEntryType;
        description = "List of files in this section";
      };
    };
  };

  # A section in methods.md
  methodsSectionType = types.submodule {
    options = {
      file = mkOption {
        type = types.str;
        description = "The source file to extract function docs from";
      };
      heading = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Optional section heading (if null, no heading is rendered)";
      };
      exports = mkOption {
        type = types.nullOr (types.listOf types.str);
        default = null;
        description = "Optional list of function names to export (if null, exports all)";
      };
    };
  };

  # The files configuration
  filesConfigType = types.submodule {
    options = {
      title = mkOption {
        type = types.str;
        default = "File Reference";
        description = "The document title";
      };
      titleLevel = mkOption {
        type = types.ints.between 1 6;
        default = 1;
        description = ''
          Heading level for the title (1-6).
          Sections are titleLevel + 1, files are titleLevel + 2.
          Content headings are shifted by titleLevel + 2.
        '';
      };
      sections = mkOption {
        type = types.listOf filesSectionType;
        description = "Sections organizing the files";
      };
    };
  };

  # The methods configuration
  methodsConfigType = types.submodule {
    options = {
      title = mkOption {
        type = types.str;
        default = "API Methods";
        description = "The document title";
      };
      titleLevel = mkOption {
        type = types.ints.between 1 6;
        default = 1;
        description = ''
          Heading level for the title (1-6).
          Section headings are titleLevel + 1.
        '';
      };
      sections = mkOption {
        type = types.listOf methodsSectionType;
        description = "Sections organizing the method documentation";
      };
    };
  };

  # The options configuration
  optionsConfigType = types.submodule {
    options = {
      title = mkOption {
        type = types.str;
        default = "Module Options";
        description = "The document title";
      };
      anchorPrefix = mkOption {
        type = types.str;
        default = "opt-";
        description = "Prefix for option anchors";
      };
    };
  };

  # Top-level manifest type
  manifestType = types.submodule {
    options = {
      files = mkOption {
        type = types.nullOr filesConfigType;
        default = null;
        description = "Configuration for files.md generation";
      };
      methods = mkOption {
        type = types.nullOr methodsConfigType;
        default = null;
        description = "Configuration for methods.md generation";
      };
      options = mkOption {
        type = types.nullOr optionsConfigType;
        default = null;
        description = "Configuration for options.md generation";
      };
    };
  };

in
{
  # Type definitions for use in option declarations
  types = {
    inherit
      fileEntryType
      filesSectionType
      methodsSectionType
      filesConfigType
      methodsConfigType
      optionsConfigType
      manifestType
      ;
  };

  # Default values
  defaults = {
    files = {
      title = "File Reference";
      titleLevel = 1;
    };
    methods = {
      title = "API Methods";
      titleLevel = 1;
    };
    options = {
      title = "Module Options";
      anchorPrefix = "opt-";
    };
  };

  # Example manifest for documentation
  example = {
    files = {
      title = "File Reference";
      titleLevel = 1;
      sections = [
        {
          name = "Core";
          files = [
            "default.nix"
            "api.nix"
            {
              name = "lib.nix";
              fallback = "Internal utility functions.";
            }
          ];
        }
      ];
    };
    methods = {
      title = "API Methods";
      titleLevel = 1;
      sections = [
        { file = "api.nix"; }
        {
          heading = "Utilities";
          file = "lib.nix";
          exports = [
            "helperFn"
            "utilFn"
          ];
        }
      ];
    };
    options = {
      title = "Module Options";
      anchorPrefix = "opt-";
    };
  };
}
