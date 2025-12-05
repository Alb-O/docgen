# Changelog

## Current

Forked from [nix-community/nixdoc](https://github.com/nix-community/nixdoc) v3.0.4.

Changes from upstream:

- Removed legacy doc-comment format support (only RFC145 `/** ... */` comments are supported)
- Renamed binary from `nixdoc` to `docgen`
- Added `options` subcommand for rendering NixOS-style module options to markdown
- Added `file-doc` command to extract file-level documentation
- Added `--export` flag to document specific let bindings
- Added `--shift-headings` argument to file-doc command
- Improved identifier resolution for let-in patterns
