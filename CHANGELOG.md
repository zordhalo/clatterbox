# Changelog

All notable changes to Clatterbox are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

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
