# Terminal Animator Docs

Terminal Animator is a mouse-first terminal art and animation editor. The
initial design target is simple: make it easy to draw terminal scenes by hand,
preview them as animations, and export them into terminal applications without
rewriting the art in Rust.

## Documents

- [Product spec](product-spec.md): user workflows, UI layout, tools, animation
  behavior, and validation expectations.
- [File format v1](file-format-v1.md): canonical `.tanim.toml` project format,
  validation rules, export formats, and migration policy.
- [Host integration](host-integration.md): how terminal applications should
  consume Terminal Animator assets safely and efficiently.

## Key Decisions

- The canonical save format is `.tanim.toml`, not raw ANSI escape output.
- ANSI and plain text are export formats only.
- Coordinates are zero-based terminal cell positions, with `x` increasing right
  and `y` increasing down.
- Version 1 stores one image or animation asset per file.
- Frames are sparse: omitted cells are transparent, and explicit space cells are
  real painted cells.
- The format supports foreground color, background color, and text attributes so
  block art can avoid "holes" where colored details replace a colored body.
- Schema version 1 is the file-format contract; implementation phases are
  separate milestones and should start narrower than the full format.

## Current Implementation Highlights

- Mouse-first editor with pencil, eraser, eyedropper, and flood fill tools.
- Clickable foreground/background color targets, swatches, character palette,
  style attributes, expanded palette, and RGB mixer controls.
- Rectangle selection with copy, cut, delete, move, and reusable stamp paste
  workflows.
- Still image and basic animation editing with frame navigation, duplicate,
  blank frame, and previous-frame onion skin.
- Save/load for canonical `.tanim.toml` files with save-as overwrite
  confirmation.
- Plain text and ANSI export from the interactive app and CLI:
  `terminal_animator --export text input.tanim.toml output.txt`
  or `terminal_animator --export ansi input.tanim.toml output.ansi`.

## Compatibility Contract

The file-format details above are intentional integration points. Do not change
coordinate semantics, sparse transparency behavior, frame timing, style
resolution, normalized layout metadata, frame count rules, resource limits, or
composition order without a schema migration.
