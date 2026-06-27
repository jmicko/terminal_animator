# Terminal Animator

Terminal Animator is a little editor for drawing terminal art by hand.

It's not that hard to grasp, basically paint in the terminal. But ascii art.
Kind of. It's easier to click characters into position with a mouse than to type
them out by hand. So you use a mouse in a terminal and it's all good.

The main file format is `.tanim.toml`. Plain text and ANSI are export formats
for sharing or previewing.

## Install

For now, build it from source:

```sh
cargo install --path .
```

Then run it:

```sh
terminal_animator
```

You can also run it without installing:

```sh
cargo run
```

## Getting Started

Open the app and choose `New Image`, `New Animation`, or `Open File`.

Most things are clickable:

- Click and drag on the canvas to draw.
- Click the current tool to switch tools.
- Click colors, characters, and style options in the sidebar.
- Click `Save`, `Save As`, `Export`, or `Quit` in the top bar.
- Use the file browser to open or save `.tanim.toml` files.

Useful keyboard shortcuts:

- `Ctrl-S`: save.
- `Ctrl-Shift-S`: save as.
- `Ctrl-Z` / `Ctrl-Y`: undo and redo.
- `Esc`: cancel the thing you are doing.
- Mouse wheel or PageUp/PageDown: scroll bigger pickers and file lists.

Save As lets you type just the name. The app adds `.tanim.toml` for you.

## State of the Project

Image editing is the main focus, and works fairly well. Don't get too
excited here. I built this to help with a different project. I'm only going
to keep improving it for as long as I need it.

## More Detail

- [Product spec](product-spec.md)
- [File format v1](file-format-v1.md)
- [Host integration](host-integration.md)

## License

Apache-2.0. See [LICENSE](../LICENSE).
