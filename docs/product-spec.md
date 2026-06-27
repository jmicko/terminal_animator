# Terminal Animator Product Spec

Status: draft

## Purpose

Terminal Animator is an MS Paint-like editor for terminal cells. A user should
be able to choose a character, choose colors, click cells on a canvas, animate
frames, and export the result into a terminal application without hand-writing
ASCII art or Rust scene code.

## Goals

- Make terminal drawing mouse-friendly.
- Support both still images and frame-based animations.
- Make animation iteration fast by copying the previous frame into the next one.
- Provide onion-skin and previous-frame comparison tools.
- Support selection, moving, copying, and stamping rectangular regions.
- Save files in a structured format that terminal applications can read
  directly.
- Export plain text and ANSI versions for sharing outside the editor.
- Keep the editor usable for someone who is not comfortable hand-editing text
  art or config files.

## Scope Terms

Schema V1 is the file-format contract. Phase 1 is the first implementation
milestone. The schema can describe animation before the editor implements every
animation workflow.

## Non-Goals For Initial Implementation

- Full vector drawing.
- Pixel-perfect image import.
- Arbitrary terminal escape sequence editing.
- Audio, video, or high-frame-rate animation.
- Multi-asset project packages.
- Networked collaboration.

## Core Concepts

Asset:
The image or animation being edited. Version 1 stores one asset per
`.tanim.toml` file.

Canvas:
A fixed-width and fixed-height grid of terminal cells.

Cell:
One terminal display cell. It may contain one displayed character plus style
metadata. Empty transparent cells are omitted from the saved frame.

Frame:
A snapshot of the canvas at one point in the animation. A still image is an
asset with one frame.

Style:
A named foreground color, optional background color, and optional text
attributes.

Stamp:
A reusable rectangular selection that can be pasted repeatedly.

Onion Skin:
A read-only visual overlay of previous or next frames while editing the current
frame.

## Stable Interop Requirements

These details are part of the external asset contract and should not be changed
casually:

- `.tanim.toml` is the canonical source format.
- Coordinates are zero-based terminal cells, with origin at the top left.
- Omitted cells are transparent.
- Explicit space cells are real painted cells.
- Runs are applied before cells, and later writes override earlier writes.
- Frame timing is stored in milliseconds.
- Styles are referenced by stable string IDs.
- Layout metadata describes minimum useful size, anchor, and overflow behavior.
- `kind = "image"` has exactly one frame, and `kind = "animation"` has one or
  more frames.
- Version 1 characters must pass the strict V1 character validity rules.
- Resource limits are enforced before loading or expanding frames.

If any of these rules need to change, make it a new schema version and provide a
migration path.

## Primary Workflows

### Create A Still Image

1. User opens Terminal Animator.
2. User chooses `New Image`.
3. User chooses width and height.
4. User paints on the canvas with character and color tools.
5. User saves as `.tanim.toml`.
6. User may export as plain text or ANSI.

### Create An Animation

1. User opens Terminal Animator.
2. User chooses `New Animation`.
3. User chooses width, height, default frame duration, and loop behavior.
4. User paints the first frame.
5. User presses `Next Frame`.
6. The new frame starts as a copy of the current frame.
7. User edits only the cells that changed.
8. User previews the animation in the preview pane.
9. User saves as `.tanim.toml`.

### Draw A Terminal Scene For Runtime Use

1. User opens or creates a `.tanim.toml` scene.
2. User sets layout metadata, such as minimum size and anchor.
3. User draws the scene with colors and transparent space.
4. User previews against multiple terminal sizes.
5. User exports or copies the `.tanim.toml` into another terminal application.

## UI Layout

The default workbench should have five regions.

Top Bar:
File actions, asset name, unsaved indicator, animation playback state, and
current frame position. Animation controls should only appear for animation
assets.

Tool And Palette Area:
Paint tools, character palette, foreground and background color swatches,
attribute toggles, and saved stamps.

Canvas:
The editable terminal-cell grid. Mouse clicks and drags act on this region.

Preview:
A live animated render of the asset. The preview should not steal focus from the
canvas unless clicked intentionally.

Timeline:
Frame thumbnails or frame labels, duration controls, insert/delete/duplicate
actions, and playback controls.

Status Area:
Persistent footer controls plus temporary messages. Temporary messages should
appear above the footer rather than replacing command hints.

## Small Terminal Layout

When the terminal is too small for the full workbench:

- Canvas remains the primary view.
- Tool palette, preview, and timeline become tabs.
- Footer commands wrap by command group, not by word.
- No text should be clipped without a visible cue.
- Mouse actions should still work on the canvas.

## Tools

Pencil:
Paints the selected character and style. Dragging paints continuously.

Eraser:
Clears cells back to transparent. A separate "paint blank" action should exist
for explicit space cells.

Eyedropper:
Picks the character and style from a cell.

Fill:
Flood-fills a contiguous region. Matching can be by character, style, or both.

Selection:
Drag to select a rectangle. The selected rectangle can be moved, copied, cut,
deleted, or saved as a stamp.

Stamp:
Paints a saved rectangular region onto the canvas. Transparent cells in the
stamp remain transparent by default.

Text:
Types a string into the canvas using the current style.

Line And Rectangle:
Useful phase-two tools. They should be planned but do not need to block V1.

## Selection Behavior

Selection starts with a mouse drag on the canvas. A selected rectangle should
show a clear outline without destroying the art underneath.

Supported operations:

- Move selection.
- Copy selection.
- Cut selection.
- Delete selection.
- Duplicate selection in place.
- Save selection as stamp.
- Paste from stamp.
- Nudge selection with arrow keys.

When moving a selection, the editor should show a preview before committing.
Esc cancels the move. Enter commits.

## Animation Behavior

Each frame has a duration in milliseconds. Assets also have a default frame
duration used when a frame does not override it.

New Frame:
Creates a new frame after the current frame by copying the current frame.

Duplicate Frame:
Creates an exact copy of the current frame.

Blank Frame:
Creates a transparent frame.

Previous-Frame Toggle:
A hold-to-preview key temporarily swaps the canvas view to the previous frame.
This allows quick comparison by repeatedly pressing or holding one key.

Onion Skin:
The canvas can show previous and next frames as faint overlays. The user can
choose how many frames are visible, with a default of one previous frame.

Preview:
The preview pane plays the full animation using real frame durations. It should
also support play, pause, step forward, step backward, and restart.

## Character Palette

The editor should ship with practical default character groups:

- ASCII drawing: `-`, `_`, `/`, `\`, `|`, `+`, `.`, `,`, `'`, `"`, `*`.
- Blocks and shading.
- Box drawing.
- Weather and sparkle symbols when supported by the terminal font.
- Recent characters.
- User favorites.

Version 1 should reject or warn on characters that occupy more than one terminal
cell. Wide graphemes make coordinates ambiguous and should be deferred until a
future schema version.

V1 should reject control characters, tabs, newlines, combining marks,
zero-width marks, variation selectors, multi-scalar grapheme clusters, emoji
sequences, and double-width characters. The initial character palette should
only expose characters that pass this validation.

## Color And Style Palette

The editor should support:

- Foreground color.
- Optional background color.
- Bold, dim, italic, underline, and reverse attributes.
- Transparent background.
- Reusable named styles.
- Recent styles.
- Project palette styles.
- Mouse-friendly color picking with an expanded palette and RGB mixer.
- Recent custom colors for colors chosen outside the default palette.

Colors are stored as sRGB hex values. Renderers can map those colors to the
nearest available terminal color if truecolor is unavailable.

## File Browser And Preferences

Open and Save As should use a mouse-friendly file browser rather than requiring
the user to type paths from memory.

Expected behavior:

- Default to the last used directory, then the user's home directory, then the
  current working directory.
- Store lightweight preferences in a local config file. A database is not
  needed.
- Hide dot-prefixed folders and files by default, with a visible toggle to show
  them.
- Save As should let the user edit the base file name without deleting the
  `.tanim.toml` suffix by hand.
- Save As should append `.tanim.toml` automatically when the user types a base
  name.
- Save As should warn before overwriting an existing file.
- Recent custom colors may also live in the same config file.

## Mouse Controls

Default mouse behavior:

- Left click paints with the active tool.
- Left drag paints or selects, depending on tool.
- Right click opens a context menu for the clicked canvas cell or selection.
- Wheel scrolls panels when the pointer is over a scrollable panel.
- Click a swatch to select a color.
- Click a character to select it.
- Click timeline frames to select frames.

The context menu should never be required for core workflows; it is a shortcut.

## Keyboard Controls

Suggested defaults:

- `Ctrl-S`: save.
- `Ctrl-Shift-S`: save as.
- `Ctrl-Z`: undo the previous edit operation.
- `Ctrl-Y`: redo the next edit operation.
- `Space`: play or pause preview when focus is not in a text field.
- `N`: next frame.
- `P`: previous frame.
- `D`: duplicate frame.
- `B`: blank frame.
- `O`: toggle onion skin.
- Hold `[` or another dedicated key: temporarily show previous frame.
- `V`: selection tool.
- `I`: eyedropper.
- `E`: eraser.
- `Esc`: cancel current transient operation.
- Arrow keys: move cursor or nudge selected region.
- Shift plus arrow keys: expand selection.

Final bindings can change after hands-on testing.

## Validation

The editor should validate before saving:

- Asset width and height are positive.
- Asset width, height, frame count, style count, run count, and expanded cell
  count are within resource limits.
- Every cell coordinate is inside bounds.
- Every referenced style exists.
- Every frame has a positive duration or inherits a positive default duration.
- Every stored character passes V1 character validity rules.
- Enum values are valid.
- Hex colors are valid.
- Foreground colors are not transparent.
- Style attributes are valid.
- Style IDs are unique.
- Frame IDs are unique when present.
- `kind = "image"` has exactly one frame.
- `kind = "animation"` has at least one frame.

Warnings should not block saving unless they would make the file unreadable.
Errors should block saving and point to the specific frame, cell, or style that
needs attention.

## Recovery And Safety

- Save should be atomic: write a temporary file, then rename.
- Keep a timestamped backup before overwriting a file from an older schema.
- Malformed files should offer `Open read-only`, `Start fresh`, and `Cancel`.
- Autosave can be added later, but explicit saves should be enough for V1.
- The app should never silently discard frames.

## Phase 1 CLI And Save Workflow

The first implementation should support simple file-oriented startup behavior:

- `terminal_animator`: starts on a welcome screen with `New image` and `Open
  file` actions.
- `terminal_animator path/to/art.tanim.toml`: opens an existing file when it
  exists.
- `terminal_animator path/to/new-art.tanim.toml`: offers to create a new image
  that will save to that path.
- `terminal_animator --new WIDTHxHEIGHT path/to/art.tanim.toml`: creates a new
  image with the given dimensions and target save path.
- `terminal_animator --export text input.tanim.toml output.txt`: exports without
  opening the interactive editor.
- `terminal_animator --export ansi input.tanim.toml output.ansi`: exports ANSI
  output without opening the interactive editor.

Save behavior:

- `Ctrl-S` saves to the current file path when one exists.
- `Ctrl-S` on an untitled image opens a save-as prompt.
- `Ctrl-Shift-S` always opens a save-as prompt.
- Save-as should warn before overwriting an existing file.
- Closing with unsaved changes should offer `Save`, `Discard`, and `Cancel`.
- If a loaded file validates with warnings, saving should write the normalized
  canonical form.
- If a loaded file has validation errors, saving should remain blocked until
  the errors are fixed.

## Export Targets

Canonical:
`.tanim.toml`, lossless project file.

Plain Text:
Current frame or all frames without color or timing.

ANSI:
Current frame or full animation as terminal escape output. This is for display
or sharing, not for source control as the canonical project file.

Runtime Asset:
The same `.tanim.toml` file, optionally copied into another application's asset
folder. A future exporter may generate a Rust include helper, but the data file
should remain the source of truth.

## Suggested Implementation Phases

Phase 1:
Image-only editor with one frame, pencil, eraser, eyedropper, style palette,
save/load `.tanim.toml`, file validation, and basic per-stroke undo/redo.

Phase 1 acceptance criteria:

- Create a new image with fixed width and height.
- Paint and erase cells with mouse input.
- Pick a cell's character and style with eyedropper.
- Create, edit, and reuse named styles.
- Undo and redo at least one full paint stroke at a time.
- Save a valid `.tanim.toml` image file.
- Load the saved image without data loss.
- Reject invalid files with actionable validation errors.
- Export the current image as plain text.

Phase 1 implementation order:

1. Define the file model.
2. Build the parser and validator.
3. Build save/load round trips.
4. Build the canvas renderer.
5. Add mouse painting tools.
6. Add undo/redo.
7. Add plain text export.

Phase 2:
Animation foundation: timeline, frame duplication, preview playback, frame
durations, onion skin, and previous-frame toggle.

Phase 3:
Selection, move, copy, cut, paste, stamps, and text tool.

Phase 4:
ANSI export, runtime integration helpers, size preview, recovery flows, and
validation polish.

Phase 5:
Optional line/rectangle/fill polish, autosave, recent projects, and asset
libraries.
