# Terminal Animator File Format V1

Status: draft

The canonical Terminal Animator file format is a UTF-8 TOML file with extension
`.tanim.toml`.

Raw ANSI output is not the canonical format. ANSI is an export format because it
mixes artwork with terminal control sequences, cursor movement, and terminal
assumptions that are not stable source data.

## Design Goals

- Lossless for terminal-cell art and frame animation.
- Easy to parse from Rust.
- Reasonably readable in a text editor.
- Stable enough for source control.
- Simple for terminal applications to import.
- Explicit about transparency, color, frame timing, and layout.

## Stable Interop Requirements

The following rules are compatibility-critical. Do not change them within schema
version 1:

- Top-left origin, zero-based coordinates, `x` rightward, `y` downward.
- Width and height are terminal cell counts.
- Omitted cells are transparent.
- `ch = " "` is an explicit painted space.
- `runs` are expanded before `cells`.
- Later writes override earlier writes at the same coordinate.
- Style references resolve by `styles.id`.
- Frame durations are milliseconds.
- Layout metadata uses `min_width`, `min_height`, `anchor`, and `overflow`.
- `kind = "image"` has exactly one frame, and `kind = "animation"` has one or
  more frames.
- Missing layout fields are normalized with the defaults in this document.
- Resource limits are enforced before frame expansion.
- Stored characters must be valid V1 terminal characters.

Any breaking change to these rules requires a new `schema_version` and a
migration.

## Versioning

`schema_version` is a positive integer. It is independent from the application
version.

Rules:

- New files are written with the current schema version.
- Clean older files may be migrated on save.
- Malformed files should not be overwritten unless the user explicitly chooses
  to save a repaired or fresh version.
- Unknown future schema versions should open read-only unless the app knows how
  to migrate them.

## Coordinate System

- Origin is the top-left cell.
- `x` increases to the right.
- `y` increases downward.
- Coordinates are zero-based.
- `width` and `height` are measured in terminal cells.
- Version 1 requires every stored character to pass the V1 character validity
  rules.

## Transparency

The format is sparse.

- Omitted cells are transparent.
- A cell with `ch = " "` is an explicit painted space.
- Explicit spaces can carry background color.
- Transparent cells do not erase lower layers in V1 because V1 has no layer
  stack.

## Minimal Example

```toml
schema_version = 1

[asset]
name = "tiny-star"
kind = "image"
width = 7
height = 3
default_frame_duration_ms = 250
loop = true

[layout]
min_width = 7
min_height = 3
anchor = "center"
overflow = "clip"

[[styles]]
id = "star"
fg = "#E0B952"
bg = "transparent"
attrs = ["bold"]

[[frames]]
id = "frame-1"

[[frames.cells]]
x = 3
y = 0
ch = "*"
style = "star"

[[frames.cells]]
x = 2
y = 1
ch = "*"
style = "star"

[[frames.cells]]
x = 4
y = 1
ch = "*"
style = "star"

[[frames.cells]]
x = 3
y = 2
ch = "*"
style = "star"
```

## Top-Level Fields

```toml
schema_version = 1
```

Required.

## Asset Table

```toml
[asset]
name = "sleigh"
kind = "animation"
width = 48
height = 16
default_frame_duration_ms = 300
loop = true
```

Fields:

- `name`: required string. Human-readable asset name.
- `kind`: required string, either `"image"` or `"animation"`.
- `width`: required positive integer.
- `height`: required positive integer.
- `default_frame_duration_ms`: required positive integer.
- `loop`: required boolean.

Frame count rules:

- `kind = "image"` requires exactly one frame.
- `kind = "animation"` requires one or more frames.
- A one-frame animation is valid.

## Layout Table

```toml
[layout]
min_width = 24
min_height = 8
anchor = "bottom_center"
overflow = "clip"
```

Fields:

- `min_width`: optional positive integer. Smallest useful render width.
- `min_height`: optional positive integer. Smallest useful render height.
- `anchor`: optional string. One of `top_left`, `top_center`, `top_right`,
  `center_left`, `center`, `center_right`, `bottom_left`, `bottom_center`, or
  `bottom_right`.
- `overflow`: optional string. One of `clip` or `hide`.

Canonical writers should always emit `[layout]` with all fields populated.
Readers should accept a missing `[layout]` table or missing layout fields and
normalize them before validation or rendering:

- `min_width`: defaults to `asset.width`.
- `min_height`: defaults to `asset.height`.
- `anchor`: defaults to `center`.
- `overflow`: defaults to `clip`.

Host applications can use the normalized layout to filter assets by render size
and place them in the available region.

## Styles

```toml
[[styles]]
id = "fire-orange"
fg = "#FF9D2E"
bg = "transparent"
attrs = ["bold"]
role = "accent"
```

Fields:

- `id`: required string. Unique within the file.
- `fg`: required `#RRGGBB` color. Foreground transparency is not valid in V1.
- `bg`: optional color or `"transparent"`. Defaults to `"transparent"`.
- `attrs`: optional list of text attributes.
- `role`: optional string for semantic grouping.

Supported attributes:

- `bold`
- `dim`
- `italic`
- `underline`
- `reverse`

Color values:

- `#RRGGBB`
- `"transparent"` for background only

Hex colors must contain exactly six ASCII hexadecimal digits after `#`.

## Frames

```toml
[[frames]]
id = "glide-01"
duration_ms = 300
```

Fields:

- `id`: optional string. Unique when present.
- `duration_ms`: optional positive integer. Defaults to
  `asset.default_frame_duration_ms`.

Frames may contain `runs` and `cells`.

In TOML, `[[frames.runs]]` and `[[frames.cells]]` attach to the most recent
`[[frames]]` table. Canonical writers must therefore emit each complete frame as
a contiguous block:

1. `[[frames]]`
2. That frame's `[[frames.runs]]`
3. That frame's `[[frames.cells]]`
4. The next `[[frames]]`

Do not emit all frame headers first and then all runs or cells later; that would
attach the nested arrays to the wrong frame.

## Runs

Runs are useful for row fragments that share one style.

```toml
[[frames.runs]]
x = 10
y = 8
text = "/____\\"
style = "sleigh-red"
```

Fields:

- `x`: required integer.
- `y`: required integer.
- `text`: required string.
- `style`: required style ID.

Each character in `text` must be a valid V1 terminal character. Runs are
expanded from left to right. Runs may contain spaces; those spaces are explicit
painted cells.

## Cells

Cells are useful for accents and mixed-style details.

```toml
[[frames.cells]]
x = 16
y = 6
ch = "*"
style = "gold"
```

Fields:

- `x`: required integer.
- `y`: required integer.
- `ch`: required string containing exactly one valid V1 terminal character.
- `style`: required style ID.

## Character Validity

V1 is intentionally strict. Every stored character in `cells.ch` and every
character in `runs.text` must be:

- Exactly one Unicode scalar value.
- Display width 1 in a terminal cell.
- Not a control character.
- Not a tab, newline, carriage return, escape, or delete character.
- Not a combining mark, zero-width mark, or variation selector.
- Not part of a multi-scalar grapheme cluster.

The implementation authority for display width is Rust's `unicode-width` crate.
A V1 character is display-width-valid only when `UnicodeWidthChar::width(c)`
returns `Some(1)`, after the explicit exclusions above have been applied. The
validator should also use Unicode general categories to reject marks, control
characters, format characters, surrogate code points, and unassigned code
points.

This means V1 supports practical ASCII, box drawing, block drawing, punctuation,
and many single-cell symbols, but rejects emoji, combining accents, ZWJ
sequences, and double-width CJK characters. A future schema version can add
explicit wide-grapheme support if the editor needs it.

## Composition Rules

For each frame:

1. Start with a transparent canvas of `asset.width` by `asset.height`.
2. Apply `runs` in file order.
3. Apply `cells` in file order.
4. Later writes override earlier writes at the same coordinate.

This means an editor can save broad shapes as runs and small details as cells
without needing a separate layer system.

## Deterministic Save Order

Writers should preserve stable output:

1. `schema_version`
2. `[asset]`
3. `[layout]`
4. `[[styles]]`
5. For each frame in animation order:
   - `[[frames]]`
   - that frame's `[[frames.runs]]`
   - that frame's `[[frames.cells]]`

Style order should follow palette order. Frame order is animation order.
Within each frame, run order and cell order are composition order.

## Validation Rules

Errors:

- Missing required top-level fields.
- Unsupported schema version.
- Non-positive width, height, or duration.
- Invalid `asset.kind`.
- Invalid `layout.anchor`.
- Invalid `layout.overflow`.
- Invalid style attribute.
- Invalid hex color.
- `styles.fg = "transparent"`.
- `kind = "image"` with anything other than exactly one frame.
- `kind = "animation"` with zero frames.
- Duplicate style IDs.
- Duplicate frame IDs.
- Unknown style reference.
- Cell or run coordinate outside the canvas.
- Run text extends outside the canvas.
- Character fails V1 character validity rules.
- Resource limits are exceeded.

Warnings:

- Unused style.
- Empty animation frame.
- Layout minimum larger than asset size.
- Very fast frame duration, such as below 80 ms.

## Resource Limits

Parsers should enforce hard limits before allocating expanded frame buffers.
These limits are part of safe file handling, not artistic intent. Implementations
may choose stricter limits, but should not exceed these defaults without an
explicit user-controlled override:

- Maximum width: 500 cells.
- Maximum height: 300 cells.
- Maximum area per frame: 150,000 cells.
- Maximum frames: 1,000.
- Maximum styles: 256.
- Maximum runs per frame: 10,000.
- Maximum explicit cells per frame: 150,000.
- Maximum expanded writes per frame, including runs and cells before override
  collapse: `max(150,000, asset.width * asset.height * 4)`, capped at 600,000.
- Maximum expanded cells across all frames: 5,000,000.

Run text must also fit within the canvas row after expansion. A single run
therefore cannot expand beyond the remaining width of its row, and cannot exceed
500 characters under the default width limit.

`expanded writes per frame` counts all run-expanded cells and explicit cells
before later writes override earlier writes. `expanded cells across all frames`
counts final non-transparent cells after composition.

## Export Formats

Plain text:
Exports frame contents without style metadata. Transparent cells become spaces.

ANSI:
Exports styled terminal output. Animation export may emit escape sequences and
timing hints, but it is not intended as source data.

Rust helper:
A future exporter may generate Rust code that embeds a `.tanim.toml` file with
`include_str!` and calls a shared renderer. Generated Rust should not replace
the `.tanim.toml` source.

## Future Schema Candidates

Possible V2 features:

- Multiple layers.
- Wide grapheme support.
- Per-frame canvas size changes.
- Named hitboxes or anchors.
- Per-cell metadata.
- Asset packages with multiple sprites.
- Palette inheritance across files.
