# Host Application Integration

Status: draft

This document describes how terminal applications should consume assets created
by Terminal Animator.

## Integration Principle

Host applications should import structured `.tanim.toml` files, not ANSI
exports.

ANSI output is useful for previews and sharing, but it is not a good internal
asset format. It bakes together content, style, cursor movement, and terminal
behavior. A structured file lets a renderer choose the right frame, clip safely,
anchor the art, and render only the cells that fit.

## Recommended Runtime Model

A host application can load each asset into a compact in-memory model:

```rust
pub struct TerminalAsset {
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub default_frame_duration_ms: u64,
    pub loop_animation: bool,
    pub layout: TerminalAssetLayout,
    pub styles: Vec<TerminalStyle>,
    pub frames: Vec<TerminalFrame>,
}

pub struct TerminalFrame {
    pub id: Option<String>,
    pub duration_ms: u64,
    pub cells: Vec<TerminalCell>,
}

pub struct TerminalCell {
    pub x: i32,
    pub y: i32,
    pub ch: char,
    pub style_index: usize,
}
```

The file format stores non-negative coordinates inside the asset bounds. The
runtime model may use signed coordinates because anchor offsets and clipping can
produce negative intermediate positions. The importer should expand `runs` and
`cells` once at load time. Rendering then only iterates over pre-expanded cells.

## Scene Selection

Animated decorative scenes should be chosen when the scene is about to render,
not when the app starts.

Reason:
Pane size can change. If a user starts in a small terminal and later expands it,
the renderer should be able to choose a richer scene. If a user shrinks the
terminal, the renderer should stop choosing scenes that no longer fit.

Selection algorithm:

1. Build the candidate list for the current context, such as an empty grid slot
   or a persistent-output pane.
2. Filter out assets whose `layout.min_width` or `layout.min_height` exceeds the
   current render area.
3. Filter out assets whose tags or context do not match, if tags are added
   later.
4. Pick randomly from the remaining candidates.
5. If no candidates fit, render nothing or render a tiny fallback.

For testing, a host application may support a private environment variable such
as `TERMINAL_ANIMATOR_FORCE_SCENE=sleigh`, but this should not need to become a
user-facing setting.

## Anchoring

Terminal Animator assets include a layout anchor. Host applications should honor
it when possible.

Common anchors:

- `bottom_center` for snowmen, fireplaces, trees, presents, and skating scenes.
- `center` for floating effects.
- `top_center` for sky scenes.

If the asset is larger than the render area:

- `overflow = "clip"` means draw the visible part.
- `overflow = "hide"` means skip the scene.

## Frame Timing

Frame selection should be based on elapsed time:

```text
elapsed_ms % total_animation_duration_ms
```

Then walk frames until the accumulated duration contains the elapsed point.

For non-looping animations, clamp to the last frame once elapsed time exceeds
the total duration.

## Rendering To Ratatui

Suggested rendering flow:

1. Determine the render area.
2. Pick the current frame by elapsed time.
3. Compute anchor offset.
4. For each cell in the frame:
   - Add anchor offset.
   - Skip if outside the target area.
   - Apply character and style to the target buffer cell.

Conceptual code:

```rust
let area_left = i32::from(area.x);
let area_top = i32::from(area.y);
let area_right = area_left + i32::from(area.width);
let area_bottom = area_top + i32::from(area.height);

for cell in frame.cells.iter() {
    let x = area_left + offset_x + cell.x;
    let y = area_top + offset_y + cell.y;

    if x < area_left || y < area_top {
        continue;
    }

    if x >= area_right || y >= area_bottom {
        continue;
    }

    let style = asset.styles[cell.style_index].to_ratatui_style();
    buf[(x as u16, y as u16)]
        .set_symbol(&cell.ch.to_string())
        .set_style(style);
}
```

Actual code should avoid allocating a `String` per cell every frame. The
expanded runtime model can store a one-character string if profiling shows the
allocation matters.

## Where Assets Should Live

Suggested layout inside a host application:

```text
assets/
  terminal-scenes/
    fireplace.tanim.toml
    snowman.tanim.toml
    tree.tanim.toml
    sleigh.tanim.toml
```

The renderer can use `include_str!` for built-in assets:

```rust
const FIREPLACE: &str = include_str!("../assets/terminal-scenes/fireplace.tanim.toml");
```

This keeps built-in scenes available without runtime file lookup.

## Output Panes With Persistent Logs

Some host applications render decorative scenes below persistent output. The
scene should never clear or rewrite that output.

Recommended behavior:

- Reserve only the lower portion of the visible pane for the scene.
- Keep at least two or three lines of final process output visible.
- If there is not enough room for both output and a valid scene, skip the scene.
- Do not add artificial output lines to the terminal history just to display the
  decoration.

The decoration should be a render overlay, not terminal output.

## Empty Grid Slots

Empty grid slots can use the whole pane. They are the best place for larger or
more elaborate scenes.

The scene should be picked from all assets that fit the current slot size. It
should not always default to one scene.

## Compatibility Checklist

Before embedding a Terminal Animator file:

- Validate `schema_version`.
- Normalize layout defaults.
- Enforce resource limits before expanding frames.
- Validate image and animation frame counts.
- Validate enum values, style attributes, and colors.
- Validate all style references.
- Expand runs to cells.
- Reject characters that fail V1 character validity rules.
- Reject out-of-bounds cells.
- Cache expanded frames.
- Use signed coordinate math for anchor offsets and clipping.
- Render with clipping.
- Filter by layout minimums at render time.

## Why Background Color Matters

Some terminal art uses solid block characters for bodies and colored foreground
characters for lights, sparks, or faces. If a colored detail replaces the whole
cell without preserving the background, the art can look like it has holes.

The `.tanim.toml` style model supports both foreground and background color so a
detail can be drawn as a foreground symbol over a colored background cell.

## Future Helper Crate

If the importer grows beyond a small module, consider a shared crate:

```text
terminal_animator_format
```

Responsibilities:

- Parse `.tanim.toml`.
- Validate schema.
- Expand runs.
- Resolve styles.
- Provide a renderer-neutral asset model.

`terminal_animator` and host applications could both depend on this crate,
avoiding format drift.
