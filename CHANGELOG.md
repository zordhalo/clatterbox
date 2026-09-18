# Changelog

All notable changes to Clatterbox are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
uses [Semantic Versioning](https://semver.org/).

## [0.1.1] - 2026-09-18

### Fixed
- macOS: the app bundle is now ad-hoc signed (`signingIdentity: "-"`). Unsigned universal
  bundles can fail on Apple Silicon with "Clatterbox is damaged and can't be opened".
- README: macOS first-run steps now cover macOS 15, which removed the right-click → Open
  bypass. Use System Settings → Privacy & Security → Open Anyway, or `xattr`.

## [0.1.0] - 2026-09-18

### Added
- Initial implementation: tray app, keyboard hook (Windows Raw Input, macOS CGEventTap, Linux
  X11/evdev), cpal-based mixer with pan/pitch/gain variation, procedural synth packs
  (`synth/thock`, `synth/click`), sample-based packs with load-time derivation of missing
  key-up and special-key sets, settings window, pack import.
- Two bundled CC0 sample packs: `builtin/classic` ("Classic Office") and `builtin/tactile`
  ("Tactile").
- CI (Windows/macOS/Linux) and unsigned draft release workflow.

## [0.1.0] - Unreleased

First public release. See "Added" above.
