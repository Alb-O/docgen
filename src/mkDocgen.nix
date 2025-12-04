# Create a docgen instance for a project
#
# This is the main entry point for consumers. It takes a manifest
# and returns scripts and derivations for generating documentation.
{
  lib,
  pkgs,
  manifest,
  srcDir,
  siteDir,
  extraFiles,
  nixdocPkg,
  mdformatPkg,
  mdbookPkg,
  optionsJson,
  anchorPrefix,
  name,
  referenceDir,
  localPaths,
  outputFiles,
  docgenLib,
}:
let
  # Load manifest if it's a path
  loadedManifest = if builtins.isPath manifest then import manifest else manifest;

  # Generate the shell commands based on manifest
  filesCommands =
    if loadedManifest ? files && loadedManifest.files != null then
      docgenLib.generateFilesCommands {
        filesConfig = loadedManifest.files;
      }
    else
      null;

  methodsCommands =
    if loadedManifest ? methods && loadedManifest.methods != null then
      docgenLib.generateMethodsCommands {
        methodsConfig = loadedManifest.methods;
        prefix = anchorPrefix;
      }
    else
      null;

  optionsCommands =
    if loadedManifest ? options && loadedManifest.options != null && optionsJson != null then
      docgenLib.generateOptionsCommands { optionsConfig = loadedManifest.options; }
    else
      null;

  # Output file names (with defaults)
  filesOutput = outputFiles.files or "files.md";
  methodsOutput = outputFiles.methods or "methods.md";
  optionsOutput = outputFiles.options or "options.md";

  # Create the generate script for API reference docs
  # Takes $SRC_DIR, $OUT_DIR, and optionally $OPTIONS_JSON as arguments
  generateDocsScript = pkgs.writeShellScript "docgen-generate" ''
    set -e
    SRC_DIR="$1"
    OUT_DIR="$2"
    OPTIONS_JSON="''${3:-}"

    NIXDOC="${lib.getExe' nixdocPkg "nixdoc"}"
    MDFORMAT="${lib.getExe mdformatPkg}"

    ${
      if methodsCommands != null then
        ''
          # Generate ${methodsOutput}
          {
            ${methodsCommands}
          } > "$OUT_DIR/${methodsOutput}"
          $MDFORMAT "$OUT_DIR/${methodsOutput}"
        ''
      else
        ""
    }

    ${
      if optionsCommands != null then
        ''
          # Generate ${optionsOutput}
          if [ -n "$OPTIONS_JSON" ]; then
            ${optionsCommands} > "$OUT_DIR/${optionsOutput}"
            $MDFORMAT "$OUT_DIR/${optionsOutput}"
          fi
        ''
      else
        ""
    }

    ${
      if filesCommands != null then
        ''
          # Generate ${filesOutput}
          {
            ${filesCommands}
          } > "$OUT_DIR/${filesOutput}"
          $MDFORMAT "$OUT_DIR/${filesOutput}"
        ''
      else
        ""
    }
  '';

  # Generate API reference derivation
  apiReference =
    pkgs.runCommand "${name}-api-reference"
      {
        nativeBuildInputs = [
          nixdocPkg
          mdformatPkg
        ];
      }
      ''
        mkdir -p $out
        ${generateDocsScript} ${srcDir} $out ${if optionsJson != null then optionsJson else ""}
      '';

  # Computed reference path (empty string means directly in src/)
  refPath = if referenceDir == "" then "" else "${referenceDir}/";

  # Build site with generated reference (only if siteDir is provided)
  siteWithGeneratedDocs =
    if siteDir != null then
      pkgs.runCommand "${name}-site-src" { } (
        ''
          cp -r ${siteDir} $out
          chmod -R +w $out
        ''
        + (lib.optionalString (referenceDir != "") "mkdir -p $out/src/${referenceDir}\n")
        + (
          if methodsCommands != null then
            "cp ${apiReference}/${methodsOutput} $out/src/${refPath}${methodsOutput}\n"
          else
            ""
        )
        + (
          if optionsCommands != null && optionsJson != null then
            "cp ${apiReference}/${optionsOutput} $out/src/${refPath}${optionsOutput}\n"
          else
            ""
        )
        + (
          if filesCommands != null then
            "cp ${apiReference}/${filesOutput} $out/src/${refPath}${filesOutput}\n"
          else
            ""
        )
        + lib.concatStringsSep "\n" (
          lib.mapAttrsToList (dest: src: "cp ${src} $out/src/${dest}") extraFiles
        )
      )
    else
      null;

  # Build the final docs site
  docs =
    if siteDir != null then
      pkgs.stdenvNoCC.mkDerivation {
        name = "${name}-site";
        src = siteWithGeneratedDocs;
        nativeBuildInputs = [ mdbookPkg ];
        buildPhase = ''
          runHook preBuild
          mdbook build --dest-dir $out
          runHook postBuild
        '';
        dontInstall = true;
      }
    else
      null;

  # Local paths for serve/build scripts
  localSiteDir = localPaths.site or "./site";
  localSrcDir = localPaths.src or "./src";
  localRefDir = if referenceDir == "" then "" else "/${referenceDir}";

  # Script to serve docs locally with live reload
  # Paths are baked in from mkDocgen configuration
  serveDocsScript =
    if siteDir != null then
      pkgs.writeShellScript "docgen-serve" ''
        set -e
        SITE_DIR="${localSiteDir}"
        SRC_DIR="${localSrcDir}"
        REF_DIR="$SITE_DIR/src${localRefDir}"
        OPTIONS_JSON="${if optionsJson != null then optionsJson else ""}"

        MDBOOK="${lib.getExe mdbookPkg}"

        cleanup() { kill $pid 2>/dev/null; }
        trap cleanup EXIT INT TERM

        if [ ! -d "$SITE_DIR" ]; then
          echo "Error: Site directory '$SITE_DIR' not found. Run from the project root."
          exit 1
        fi

        echo "Generating API reference..."
        mkdir -p "$REF_DIR"
        ${generateDocsScript} "$SRC_DIR" "$REF_DIR" "$OPTIONS_JSON"

        ${lib.concatStringsSep "\n" (
          lib.mapAttrsToList (dest: src: ''cp "${src}" "$SITE_DIR/src/${dest}"'') extraFiles
        )}

        echo "Starting mdbook server..."
        $MDBOOK serve "$SITE_DIR" &
        pid=$!
        sleep 1
        echo "Documentation available at http://localhost:3000"
        wait $pid
      ''
    else
      null;

  # Script to build docs locally
  # Paths are baked in from mkDocgen configuration
  buildDocsScript =
    if siteDir != null then
      pkgs.writeShellScript "docgen-build" ''
        set -e
        SITE_DIR="${localSiteDir}"
        SRC_DIR="${localSrcDir}"
        REF_DIR="$SITE_DIR/src${localRefDir}"
        OPTIONS_JSON="${if optionsJson != null then optionsJson else ""}"

        MDBOOK="${lib.getExe mdbookPkg}"

        if [ ! -d "$SITE_DIR" ]; then
          echo "Error: Site directory '$SITE_DIR' not found. Run from the project root."
          exit 1
        fi

        echo "Generating API reference..."
        mkdir -p "$REF_DIR"
        ${generateDocsScript} "$SRC_DIR" "$REF_DIR" "$OPTIONS_JSON"

        ${lib.concatStringsSep "\n" (
          lib.mapAttrsToList (dest: src: ''cp "${src}" "$SITE_DIR/src/${dest}"'') extraFiles
        )}

        $MDBOOK build "$SITE_DIR"
        echo "Documentation built in '$SITE_DIR/book' directory."
      ''
    else
      null;

in
{
  inherit
    generateDocsScript
    serveDocsScript
    buildDocsScript
    apiReference
    docs
    loadedManifest
    ;

  # Expose the packages for consumers
  packages = {
    nixdoc = nixdocPkg;
    mdformat = mdformatPkg;
    mdbook = mdbookPkg;
  };

  # Helper to get the generated commands (for debugging/inspection)
  commands = {
    files = filesCommands;
    methods = methodsCommands;
    options = optionsCommands;
  };
}
