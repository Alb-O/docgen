# Core docgen library functions
#
# Pure functions for generating documentation shell commands.
# These don't depend on pkgs, only on lib.
{ lib }:
let
  inherit (lib) concatStrings concatMapStrings genList;

  # Helper to normalize file entries (string or attrset)
  normalizeFileEntry = entry: if builtins.isString entry then { name = entry; } else entry;

  # Escape backticks and other shell-sensitive chars for echo
  escapeForShell =
    s:
    builtins.replaceStrings
      [
        "`"
        "\""
        "$"
      ]
      [
        "\\`"
        "\\\""
        "\\$"
      ]
      s;

  # Generate markdown heading with given level (1-6)
  mkHeading = level: text: concatStrings (genList (_: "#") level) + " ${text}";

  /**
    Convert a NixOS-style options module to JSON format for nixdoc.

    Takes an options module (a function or attrset with `options` attribute)
    and returns a JSON string in the format expected by `nixdoc options`.

    # Arguments

    - `optionsModule` (module): A NixOS-style options module
    - `prefix` (string): Optional prefix to filter options by (e.g., "imp.")

    # Returns

    JSON string suitable for nixdoc's `options` subcommand.

    # Example

    ```nix
    optionsToJson {
      optionsModule = { lib, ... }: {
        options.myApp.enable = lib.mkEnableOption "myApp";
      };
    }
    => '{"myApp.enable": {"description": "...", "type": "boolean", ...}}'
    ```
  */
  optionsToJson =
    {
      optionsModule,
      prefix ? null,
    }:
    let
      # Evaluate the module to get structured options
      evaluated = lib.evalModules {
        modules = [ optionsModule ];
      };

      # Convert to documentation list format
      rawOpts = lib.optionAttrSetToDocList evaluated.options;

      # Filter by prefix if specified, and exclude hidden/internal options
      filteredOpts = lib.filter (
        opt:
        (opt.visible or true)
        && !(opt.internal or false)
        && (if prefix != null then lib.hasPrefix prefix opt.name else true)
      ) rawOpts;

      # Convert to the attrset format nixdoc expects
      optionsAttrset = builtins.listToAttrs (
        map (o: {
          name = o.name;
          value = removeAttrs o [
            "name"
            "visible"
            "internal"
          ];
        }) filteredOpts
      );
    in
    builtins.toJSON optionsAttrset;

  # Generate shell commands for a single file in files.md
  mkFileCommands =
    {
      nixdocVar,
      srcDirVar,
      fileLevel,
      contentShift,
    }:
    entry:
    let
      normalized = normalizeFileEntry entry;
      filename = normalized.name;
      fallback = normalized.fallback or null;
      escapedFallback = if fallback != null then escapeForShell fallback else null;
      heading = mkHeading fileLevel filename;
    in
    ''
      echo "${heading}"
      echo ""
    ''
    + (
      if escapedFallback != null then
        ''
          echo "${escapedFallback}"
          echo ""
        ''
      else
        ''
          ''$${nixdocVar} file-doc --file "''$${srcDirVar}/${filename}" --shift-headings ${toString contentShift} || true
          echo ""
        ''
    );

  # Generate shell commands for a section in files.md
  mkSectionCommands =
    {
      nixdocVar,
      srcDirVar,
      sectionLevel,
      fileLevel,
      contentShift,
    }:
    section:
    let
      heading = mkHeading sectionLevel section.name;
      fileCmd = mkFileCommands {
        inherit
          nixdocVar
          srcDirVar
          fileLevel
          contentShift
          ;
      };
    in
    ''
      echo "${heading}"
      echo ""
    ''
    + concatMapStrings fileCmd section.files;

  # Generate all shell commands for files.md
  generateFilesCommands =
    {
      filesConfig,
      nixdocVar ? "NIXDOC",
      srcDirVar ? "SRC_DIR",
    }:
    let
      titleLevel = filesConfig.titleLevel or 1;
      sectionLevel = titleLevel + 1;
      fileLevel = titleLevel + 2;
      contentShift = fileLevel;

      sectionCmd = mkSectionCommands {
        inherit
          nixdocVar
          srcDirVar
          sectionLevel
          fileLevel
          contentShift
          ;
      };

      titleHeading = mkHeading titleLevel filesConfig.title;
    in
    ''
      echo "${titleHeading}"
      echo ""
      echo "<!-- Auto-generated - do not edit -->"
      echo ""
    ''
    + concatMapStrings sectionCmd filesConfig.sections;

  # Generate shell commands for a section in methods.md
  mkMethodSectionCommands =
    {
      nixdocVar,
      srcDirVar,
      sectionLevel,
      prefix,
    }:
    section:
    let
      hasHeading = section ? heading && section.heading != null;
      hasExports = section ? exports && section.exports != null;
      exportArg = if hasExports then "--export ${lib.concatStringsSep "," section.exports}" else "";
      heading = if hasHeading then mkHeading sectionLevel section.heading else "";
      prefixArg = if prefix != "" then "--prefix \"${prefix}\"" else "";
    in
    (
      if hasHeading then
        ''
          echo ""
          echo "${heading}"
          echo ""
        ''
      else
        ""
    )
    + ''
      ''$${nixdocVar} \
        --file "''$${srcDirVar}/${section.file}" \
        --category "" \
        --description "" \
        ${prefixArg} \
        --anchor-prefix "" \
        ${exportArg}
    '';

  # Generate all shell commands for methods.md
  generateMethodsCommands =
    {
      methodsConfig,
      nixdocVar ? "NIXDOC",
      srcDirVar ? "SRC_DIR",
      prefix ? "",
    }:
    let
      titleLevel = methodsConfig.titleLevel or 1;
      sectionLevel = titleLevel + 1;

      sectionCmd = mkMethodSectionCommands {
        inherit
          nixdocVar
          srcDirVar
          sectionLevel
          prefix
          ;
      };

      titleHeading = mkHeading titleLevel methodsConfig.title;
    in
    ''
      echo "${titleHeading}"
      echo ""
      echo "<!-- Auto-generated - do not edit -->"
      echo ""
    ''
    + concatMapStrings sectionCmd methodsConfig.sections;

  # Generate shell commands for options.md
  generateOptionsCommands =
    {
      optionsConfig,
      nixdocVar ? "NIXDOC",
      optionsJsonVar ? "OPTIONS_JSON",
    }:
    let
      title = optionsConfig.title or "Module Options";
      anchorPrefix = optionsConfig.anchorPrefix or "opt-";
    in
    ''
      ''$${nixdocVar} options \
        --file "''$${optionsJsonVar}" \
        --title "${title}" \
        --anchor-prefix "${anchorPrefix}"
    '';

in
{
  inherit
    normalizeFileEntry
    escapeForShell
    mkHeading
    optionsToJson
    mkFileCommands
    mkSectionCommands
    mkMethodSectionCommands
    generateFilesCommands
    generateMethodsCommands
    generateOptionsCommands
    ;
}
