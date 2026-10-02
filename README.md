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
| Left click on text | Position the cursor, then highlight and underline the clicked character, including while tilted or zoomed |
| `F2` | Smoothly return to the home view |
| `F1` | Show/hide the camera-controls overlay |
| `F3` | Toggle the perspective floor grid (off at startup) |
| `F4` | Toggle flat-only/orbit in 3D; from 2D-only, enter orbit mode |
| `F5` | Toggle 2D-only mode (on by default); fills the viewport and disables camera movement |

The text is rendered into an offscreen OpenGL texture at 2× the window's physical pixel width (with a 3200-pixel minimum), recreated when the viewport or display scale changes. In 2D-only mode (enabled by default), that texture fills the viewport and camera movement is disabled; press `F5` to toggle it off or on. `F4` keeps its flat-only/orbit behavior in 3D, and exits 2D-only directly into orbit mode. Flat-only mode keeps the surface facing you and hides the scene frame and HUD. The floor grid starts hidden and can be shown with `F3`. Wheel zoom is deliberately gentle, and camera motion uses frame-rate-independent exponential smoothing. Mouse positioning uses a ray/plane intersection, so it continues to work when the surface is rotated.

## Vim controls

Start in **Normal** mode. `Esc` returns to Normal from any mode.

| Input | Action |
| --- | --- |
| `h` `j` `k` `l` / arrow keys | Move left / down / up / right |
| `Ctrl+N` / `Ctrl+P` | Next / previous line in Normal or Visual mode; next / previous matching history entry in command and search prompts |
| `Home` / `End` | Move to line start / line end |
| `w` `b` `e` | Next word / previous word / word end |
| `f{char}` / `F{char}` | Find next / previous matching character on the line; pressing `f` / `F` highlights and underlines a suggested letter in every following / previous word |
| `t{char}` / `T{char}` | Move just before / after the next / previous matching character; `t` / `T` show the same word-target hints |
| `;` / `,` | Repeat the last character find in the same / opposite direction |
| `{` / `}` | Previous / next paragraph |
| `(` / `)` | Previous / next sentence |
| `0` `^` `$` | Line start / first nonblank / line end |
| `H` / `M` / `L` | First nonblank on the top / middle / bottom visible line |
| `gg` / `G` | First / last line |
| `m{letter}` / `` `{letter} `` / `'{letter}` | Set a buffer-local mark / jump to its position / jump to its first nonblank |
| `12G` or `:12` | Go to line 12 |
| `i` / `a` | Insert before / after the cursor |
| `I` / `A` | Insert at first nonblank / line end |
| `o` / `O` | Open a line below / above, preserving indentation |
| `x` / `X` / `D` | Delete under cursor / before cursor / to end of line |
| `r{char}` | Replace characters under the cursor (with counts) or throughout a Visual selection; Enter replaces with a line break |
| `dd` / `yy` | Delete / yank a line |
| `p` / `P` | Paste after / before from the internal register |
| `v` / `V`, then movement | Select characters / whole lines |
| `Ctrl+A` / `Ctrl+X` in Normal mode | Increment / decrement the decimal number at or after the cursor |
| `o` in Visual mode | Switch the active end of the selection |
| `d` / `x` / `y` in Visual mode | Delete / delete / yank the selection; line selections paste as lines |
| `<` / `>` | Unindent / indent the current line, or every selected line in Visual mode |
| `Alt+H` / `Alt+L` | Unindent / indent the current line, or every selected line in Visual mode |
| `Alt+J` / `Alt+K` | Move the current line (or the selected lines) down / up, re-indenting to match |
| `zz` | Center the cursor line in the view |
| `zc` / `zo` / `za` / `zM` / `zR` | Close / open / toggle an indentation fold / close all / open all; edits open folds |
| `u` / `Ctrl+R` | Undo / redo |
| `.` | Repeat the last change |
| `Ctrl+C` | Return to Normal; from a history window, transfer its selected line to the prompt |
| `q:` / `q/` | Edit command / search history with Vim keys; Enter executes the selected line, Ctrl+C transfers it to the prompt, Esc closes |
| `/pattern` / `?pattern`, `Enter` | Search forward / backward with live highlights and wraparound |
| `n` / `N` | Repeat in the last search direction / opposite direction |
| `Ctrl+6` | Toggle to the last active buffer |
| `Ctrl+E` / `Ctrl+Y` | Scroll down / up one display line |
| `Ctrl+D` / `Ctrl+U` | Scroll down / up half a page, then center the view |
| `Ctrl+F` / `Ctrl+B` | Scroll down / up one page, then center the view |
| `Ctrl+J` / `Ctrl+K` | Animate through five lines down / up |
| `PageDown` / `PageUp` | Move a page down / up |
| `Ctrl+S` | Save to the current filename |
| `Ctrl+V` in Insert mode | Paste system clipboard text |
| `Ctrl+W` / `Ctrl+Backspace` in Insert mode | Delete the previous word |
| `Ctrl+-` / `Ctrl+=` | Decrease / increase font size |

Counts work with movements and common operations, such as `5j`, `3w`, `2dd`, `4yy`, and `2p`. Each Insert session is one undo step. In Insert mode, Enter auto-indents, Tab inserts four spaces, and Backspace/Delete can join adjacent lines. After 15 seconds without keyboard, pointer, mouse-button, or scroll activity, Insert mode automatically returns to Normal. Vertical scroll keys keep the cursor visible, and can scroll past the end until only the final display line remains visible.

Character-find hints prefer the letter in each word requiring the fewest `;` repeats to reach; ties favor letters that occur less often in that word.

Word wrap is on by default. Press Enter twice quickly in Normal mode to toggle it. `Ctrl+H` / `Ctrl+L` scroll the text horizontally; using either shortcut turns wrapping off so long lines can be scrolled. Font size can be adjusted from 12–42 px with the Ctrl+-/= shortcuts.

### Commands

Type `:`, enter a command, and press Enter. Press Tab to complete command names or file paths; repeated Tab cycles through matches. Paths can contain spaces; enter them directly without quotes.

| Command | Action |
| --- | --- |
| `:w` | Save |
| `:w path/to/file` | Save under a filename |
| `:e path/to/file` | Open a file in another buffer; keep unsaved edits in the current buffer |
| `:e! path/to/file` | Replace the current buffer, discarding unsaved changes |
| `:Explore` / `:Ex [directory]` | Browse a directory; use `j`/`k`, Enter to open, and `-` for the parent |
| `:!command` | Run a command in the current file's directory and show captured output; press Esc to return |
| `:.!command` | Feed the current line to a command and replace it with the command's output |
| `:theme everforest` | Switch to the Everforest dark palette |
| `:theme solarized-blue` | Switch to the blue Solarized Dark palette |
| `:theme default` | Restore the Vimfloating palette |
| `:ls` / `:buffers` | Show the buffer list; press Esc to return |
| `:b {id or name}` / `:buffer` | Switch to a buffer by its list number or filename |
| `:bn` / `:bp` / `:bnext` / `:bprevious` | Switch to the next / previous buffer; modified buffers stay open in memory |
| `:bd` / `:bdelete` / `:b delete` | Delete the current buffer; add `!` to discard unsaved changes |
| `:q` | Quit if there are no unsaved changes |
| `:q!` | Quit and discard changes |
| `:wq` / `:x` | Save and quit |
| `:noh` / `:nohlsearch` | Clear search highlighting |
| `:help` | Show a compact bindings reminder |

Closing the window also checks for unsaved changes; use `:wq` or `:q!` when needed.

## Scope

This is a working multi-buffer prototype with a useful subset of Vim controls. It includes lightweight comment/string coloring, character-indexed Unicode editing, a bounded undo history, and UTF-8 file I/O. It does not implement Vim's full operator/motion grammar, plugins, LSP, or multiple floating documents.

Files are normalized to LF line endings when opened. Existing tab characters are preserved and displayed as a single arrow cell. Text uses an installed monospace font (Consolas, Menlo, DejaVu Sans Mono, or Liberation Mono), falling back to Macroquad's built-in font; available glyphs depend on the font.

## TODO

- Change (`c`) command
- Move between block delim
- Select text block
- Selection based command mode
- Visual feedback when yanking
- Inside/around text objects
- Insert mode control
- Upper/lower case
- Next/Prev word under cursor
- Double colon number
- Outline
- More Vim options
- Fuzzy finder / command palette
- 3D control with keyboard
- CLI command
- TUI mode
- LSP support
- Completion
- Tree-sitter support
- Full screen mode
- Walk around mode
- Popup mode
- Full canvas support
- Arrow/rope connector

## Done

- Marks (buffer-local letter marks)
- Indentation folds (replaceable fold provider for future Tree-sitter support)
- Ctrl+N / Ctrl+P line movement and command/search history
- Command mode shortcuts: Ctrl+C, q:, q/
- Replace with r

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

- `src/editor.rs` and `src/editor/`: buffer state, editing, Vim motions, file commands, viewport layout, and tests.
- `src/config.rs`: window, renderer, and editor constants plus the welcome buffer.
- `src/view.rs`: camera, zoom, orbit, pan, and surface picking.
- `src/render.rs`: offscreen editor rendering and 3D scene.
- `src/input.rs`: keyboard shortcuts and animated vertical movement.
- `src/main.rs`: app setup and event loop.
