# vimloating

A Rust text editor on a floating surface in 3D space, inspired by Kashikishi. A perspective grid, a straight-on text panel, and smoothly interpolated camera motion give you a spatial workspace. Rendering uses **OpenGL through Macroquad / Miniquad**.

## Run

Install a current stable Rust toolchain and run:

```sh
cargo run --release
```

Open an existing UTF-8 file, or start a new file at a given path:

```sh
cargo run --release -- notes.rs
```

The app runs as a native desktop window on Windows, Linux, and macOS. It needs an OpenGL-capable graphics driver. Linux additionally needs the usual X11/OpenGL development packages; on Debian/Ubuntu these are `libx11-dev`, `libxi-dev`, and `libgl1-mesa-dev`.

Running without a filename opens an editable introduction buffer. Save it under a name with `:w notes.rs`, or open another file with `:e path`.

## Move through space

| Input | Action |
| --- | --- |
| Mouse wheel / trackpad scroll | Smoothly zoom in and out |
| Right-button drag | Pan when started outside the editor; orbit from the surface when flat-only mode is disabled |
| Middle-button drag | Pan the camera |
| Left click on text | Position the cursor, including while tilted or zoomed |
| `F2` | Smoothly return to the home view |
| `F1` | Show/hide the camera-controls overlay |
| `F3` | Toggle the perspective floor grid (off at startup) |
| `F4` | Toggle flat-only mode (on by default); panning remains available |

The text is rendered into an offscreen OpenGL texture sized to the window's physical pixel width and recreated when the viewport or display scale changes. This keeps the text surface aligned with the available display resolution before it is placed on the world-space mesh. Flat-only mode is on by default: it keeps the surface facing you, hides the scene frame and HUD, and still allows panning. The floor grid starts hidden and can be shown with `F3`. Turn flat-only mode off with `F4` to orbit. Wheel zoom is deliberately gentle, and camera motion uses frame-rate-independent exponential smoothing. Mouse positioning uses a ray/plane intersection, so it continues to work when the surface is rotated.

## Vim controls

Start in **Normal** mode. `Esc` returns to Normal from any mode.

| Input | Action |
| --- | --- |
| `h` `j` `k` `l` / arrow keys | Move left / down / up / right |
| `w` `b` `e` | Next word / previous word / word end |
| `0` `^` `$` | Line start / first nonblank / line end |
| `gg` / `G` | First / last line |
| `12G` or `:12` | Go to line 12 |
| `i` / `a` | Insert before / after the cursor |
| `I` / `A` | Insert at first nonblank / line end |
| `o` / `O` | Open a line below / above, preserving indentation |
| `x` / `D` | Delete character / delete to end of line |
| `dd` / `yy` | Delete / yank a line |
| `p` / `P` | Paste after / before from the internal register |
| `v`, then movement | Select a characterwise region |
| `d` / `x` / `y` in Visual mode | Delete / delete / yank the selection |
| `u` / `Ctrl+R` | Undo / redo |
| `/pattern`, `Enter` | Find text, with highlights and wraparound |
| `n` / `N` | Next / previous match |
| `Ctrl+D` / `Ctrl+U` | Move half a page down / up |
| `Ctrl+J` / `Ctrl+K` | Animate through five lines down / up |
| `PageDown` / `PageUp` | Move a page down / up |
| `Ctrl+S` | Save to the current filename |
| `Ctrl+V` in Insert mode | Paste system clipboard text |
| `Ctrl+W` / `Ctrl+Backspace` in Insert mode | Delete the previous word |
| `Ctrl+-` / `Ctrl+=` | Decrease / increase font size |

Counts work with movements and common operations, such as `5j`, `3w`, `2dd`, `4yy`, and `2p`. Each Insert session is one undo step. In Insert mode, Enter auto-indents, Tab inserts four spaces, and Backspace/Delete can join adjacent lines. After 15 seconds without keyboard, pointer, mouse-button, or scroll activity, Insert mode automatically returns to Normal. The buffer scrolls vertically and horizontally to keep the cursor visible.

Word wrap is on by default. Press Enter twice quickly in Normal mode to toggle it. `Ctrl+H` / `Ctrl+L` scroll the text horizontally; using either shortcut turns wrapping off so long lines can be scrolled. Font size can be adjusted from 12–42 px with the Ctrl+-/= shortcuts.

### Commands

Type `:`, enter a command, and press Enter. Paths can contain spaces; enter them directly without quotes.

| Command | Action |
| --- | --- |
| `:w` | Save |
| `:w path/to/file` | Save under a filename |
| `:e path/to/file` | Open a file |
| `:e! path/to/file` | Open a file, discarding unsaved changes |
| `:q` | Quit if there are no unsaved changes |
| `:q!` | Quit and discard changes |
| `:wq` / `:x` | Save and quit |
| `:noh` | Clear search highlighting |
| `:help` | Show a compact bindings reminder |

Closing the window also checks for unsaved changes; use `:wq` or `:q!` when needed.

## Scope

This is a working single-buffer prototype with a useful subset of Vim controls. It includes lightweight comment/string coloring, character-indexed Unicode editing, a bounded undo history, and UTF-8 file I/O. It does not implement Vim's full operator/motion grammar, plugins, LSP, or multiple floating documents.

Files are normalized to LF line endings when opened. Existing tab characters are preserved and displayed as a single arrow cell. Text uses an installed monospace font (Consolas, Menlo, DejaVu Sans Mono, or Liberation Mono), falling back to Macroquad's built-in font; available glyphs depend on the font.

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

For a native OpenGL rendering smoke test, capture a frame and exit:

```sh
cargo run -- --screenshot preview.png
```

- `src/editor.rs`: text buffer, Vim state machine, search, history, file commands, and tests.
- `src/main.rs`: window/input loop, offscreen text renderer, 3D scene, camera, and mouse picking.
