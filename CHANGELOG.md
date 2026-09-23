# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-09-26

### Added

- Initial Editio release: minimal terminal viewing and editing, syntax highlighting,
  multi-cursors, familiar shortcuts, mouse selection and clipboard support.
- Markdown previews with Mermaid diagrams, tables, collapsible sections and
  heading/link navigation; CSV/TSV tables and pretty-printed JSON.
- Unified diffs, two-file comparison, piped input and explicit format selection.
- Recovery drafts for named and untitled documents without overwriting originals.
- Unified Find with in-file search, bounded filename/content searches, tags,
  saved globs, and shared recently viewed/modified histories.
- Git-aware discovery, explicit ignore overrides, link opening/completion and
  safe document switching with Save / Discard / Cancel.
- Configurable indentation, EditorConfig support and undoable indentation conversion.
- Terminal-derived colors and the bundled Tokyo Night Omarchy theme.
- Local release builds for Linux, macOS and Windows on x86-64 and ARM64;
  GitHub downloads, mise, same-repository Homebrew tap and Linux/macOS installer.

### Fixed

- Preserve viewport position across view/edit transitions and wrapped navigation.
- Keep file and preference saves robust across macOS metadata variations.
- Use consistent status/footer colors across terminals; hide Windows canonical
  path prefixes in Find labels while preserving filesystem paths.
- Reload recent histories across instances and sort by the corresponding Editio
  activity; rank matches before applying the 50-result cap.
- Cycle search results with the keyboard while clamping mouse-wheel navigation.
