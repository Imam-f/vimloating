# GUI and TUI differences

This comparison covers the frontend implementations reviewed on October 5, 2026:
the native desktop GUI (`vimloating`) and terminal TUI (`vimloating-tui`). Most
editing features use the same editor core. The differences are in presentation,
input handling, startup, and frontend behavior.

The findings below come from source inspection and existing tests, without
interactive UI checks. Terminal-specific behavior can depend on the terminal,
its settings, and the events it reports.

## Features and interface

| Area | GUI | TUI |
| --- | --- | --- |
| Runtime | Native Macroquad/OpenGL window; default build | Crossterm/Ratatui terminal; requires the `tui` feature |
| Environment | Requires graphics support | Requires interactive stdin/stdout |
| Views | Full-window 2D, floating flat surface, and orbit mode | Terminal grid |
| Camera | Zoom, pan, orbit, smooth motion, and cursor-following camera | None |
| Mouse | Click to position cursor and highlight the character; wheel/drag camera controls | No application mouse handling or mouse capture |
| Function keys | F2 home, F3 floor, F4 flat/orbit, F5 2D | No corresponding actions |
| F1 help | Camera overlay, visible only in orbit mode; editing continues | Full-screen, scrollable terminal help; editing keys are intercepted |
| Font | Loads a system monospace font, with fallback; Ctrl+-/= adjusts 12–42 px | Terminal controls font and size |
| Resize | Changes rendering resolution and scales the board; logical text grid depends on font size | Changes available text rows, columns, and wrapping |
| Clipboard paste | Ctrl+V in Insert mode reads system clipboard | Terminal paste events; no direct clipboard access |
| Prompt paste | Ctrl+V has no prompt-paste handler | Bracketed paste inserts non-control characters into command/search prompts |
| Newline normalization on paste | Normalizes CRLF; standalone CR is ignored | Normalizes CRLF and standalone CR to LF |
| Shell output / buffer list | Overlay panel; long output is truncated and cannot scroll | Full content area; scroll with j/k, arrows, page keys, Home/End |
| Configuration | Uses `theme` and `2d_only` | Uses `theme`; ignores `2d_only` |
| Startup buffer | Longer introduction with sample Rust and spatial controls | Short terminal-specific introduction |
| Screenshot CLI | Supports `--screenshot`, captures a frame, then exits | No screenshot option |
| Empty CLI path | Explicitly rejected | No explicit rejection |

Sources:

- [GUI startup and event loop](../src/desktop/mod.rs)
- [GUI camera](../src/desktop/view.rs)
- [GUI scene and help overlay](../src/desktop/render/scene.rs)
- [GUI font loading](../src/desktop/render/text.rs)
- [GUI CLI](../src/desktop/cli.rs)
- [TUI startup and event loop](../src/tui/mod.rs)
- [TUI CLI](../src/bin/vimloating-tui.rs)
- [TUI terminal session](../src/tui/session.rs)
- [Welcome text and terminal help](../src/tui/help.rs)
- [GUI welcome text](../src/config/welcome.rs)
- [Build features and dependencies](../Cargo.toml)

## Keyboard and navigation

| Input / behavior | GUI | TUI |
| --- | --- | --- |
| Ctrl+J/K in Normal/Visual | Animates cursor movement through five **source lines** | Immediately scrolls five **display rows**; moves cursor only when needed to keep it visible |
| Ctrl+J in Insert | Animated downward cursor movement | Auto-indented newline |
| Ctrl+K in Insert | Animated upward cursor movement | Scrolls five display rows upward |
| Ctrl+H in Insert | Horizontal scroll; disables wrapping | Backspace |
| Ctrl+H in command/search prompt | Horizontal-scroll handler | Deletes previous prompt character |
| Ctrl+I/M | No dedicated bindings | Ctrl+I inserts four spaces in Insert; Ctrl+M inserts newline or submits prompt |
| Ctrl+J/M in history window | Ctrl+J moves within the history editor; Ctrl+M has no dedicated binding | Executes selected history line |
| PageUp/PageDown | Moves by source lines; normal cursor reveal | Moves by display rows and centers cursor |
| Last-buffer shortcut | Handles Ctrl+6 | Handles Ctrl+6 and Ctrl+^ |
| Key repeat | Custom 350 ms delay / 60 ms interval for supported keys; special continuous Ctrl+J/K animation | Follows incoming terminal/OS press and repeat events |
| Shift+F/T | Repairs uppercase find commands when character events are missing or lowercase | Uses character supplied by terminal |
| Alt-modified input | Recognized Alt+H/J/K/L actions; otherwise character input can continue | Consumes all Alt-modified events; only H/J/K/L act in Normal/Visual |
| Double Enter wrap toggle | Previous Enter timestamp survives intervening keys | Ordinary non-Enter input resets timestamp; history handling also resets it |
| Insert idle activity | Held keys, pointer movement, mouse buttons, and wheel reset timeout | Received terminal events, including resize/release events, reset timeout |
| Insert idle in history editor | Timeout checks parent mode, so nested Insert mode is not timed out | Timeout checks nested history editor and returns it to Normal |

Terminal encoding can additionally make shortcuts indistinguishable—for example,
Ctrl+J versus Enter and Ctrl+H versus Backspace. A terminal may also reserve
Ctrl+S. These limitations are separate from the frontend handlers above.

Sources:

- [GUI input](../src/desktop/input/mod.rs)
- [GUI vertical animation](../src/desktop/input/motion.rs)
- [GUI key repeat](../src/desktop/input/repeat.rs)
- [TUI input and paste handling](../src/tui/input.rs)
- [GUI idle and wrap handling](../src/desktop/mod.rs)
- [TUI idle handling](../src/tui/mod.rs)
- [Shared viewport operations](../src/editor/viewport.rs)
- [Shared history handling](../src/editor/history.rs)

## Rendering

| Area | GUI | TUI |
| --- | --- | --- |
| Unicode | Attempts to draw original glyphs in fixed character positions; appearance depends on font | Replaces wide, zero-width, and control characters with `�`; original file contents remain intact |
| Tabs | Smaller, muted `→` | Normal-cell `→`, using surrounding syntax color |
| Comment coloring | Recognizes leading `//`; implementation also skips whitespace between the slashes | Recognizes leading `#` or adjacent `//` |
| Punctuation coloring | Separate color for `{ } ( ) [ ] ; : , . !` | Ordinary text color |
| Current-line highlight | Only cursor's wrapped segment | Every visible segment of cursor's source line |
| Linewise selection | Softer background across the row, with stronger color over text | Stronger background over text; softer over empty cells |
| Fold label | `+-- 3 lines: root`; counts header and trims its text | `root … 2 lines`; counts hidden lines and preserves header whitespace |
| Gutter | Fixed allocation and divider | Width grows with document line-count digits |
| Status information | Font size, wrap, cursor position, total lines; colored mode badge | Filename/path, dirty marker, wrap, cursor position; uniform status background |
| Shell mode label | `SHELL` | `OUTPUT` |
| Insert cursor | Drawn bar with animated opacity and theme Insert color | Terminal blinking-bar cursor |
| Prompt cursor | Literal `\|` appended to prompt | Actual terminal cursor |
| Long prompt/message | Shows last 112 characters | Prompt shows tail sized to terminal; messages show beginning |
| Completion popup | Pixel border and title row | Terminal box border with embedded title; selected candidate is bold |
| Search rendering | KMP matching and interval-based highlight coverage | Direct substring comparison and per-cell marking |
| Long-line syntax work | Caps prefix scanning at ten screen widths | Scans full prefix for comments/quote state |
| Redrawing | Continuously renders each frame | Redraws on events, selected state changes, and yank animation |

Sources:

- [GUI buffer rendering](../src/desktop/render/buffer.rs)
- [GUI layout](../src/desktop/render/layout.rs)
- [GUI search highlighting](../src/desktop/render/search.rs)
- [GUI completion popup](../src/desktop/render/completion.rs)
- [GUI scene and texture rendering](../src/desktop/render/scene.rs)
- [TUI rendering](../src/tui/render/mod.rs)
- [TUI completion popup](../src/tui/render/completion.rs)
- [Shared theme palette](../src/config/theme.rs)

## Shared editing features

Both frontends use the [shared editor core](../src/editor/mod.rs), which provides:

- Vim motions and counts.
- Operators, text objects, and case changes.
- Insert, Visual, linewise, and rectangular block modes.
- Marks and indentation folds.
- Undo/redo and last-change repeat.
- Literal searches, whole-word searches, and character-find hints.
- Command/search history and editable history windows.
- Word, whole-line, path, and command completion.
- Buffers, file commands, and directory browsing.
- Shell execution and current-line filtering.
- Themes and unsaved-change protection.

Shared functionality can still behave differently where a frontend supplies
different input or viewport operations, as documented above.

## Validation

The existing suites passed during the comparison:

| Command | Result |
| --- | --- |
| `cargo test --all-features` | 176 tests passed: 147 library tests, 26 desktop unit tests, and 3 desktop CLI integration tests |
| `cargo test --no-default-features --features tui` | 147 library tests passed |

Relevant frontend coverage is in [desktop tests](../tests/unit/desktop),
[TUI tests](../tests/unit/tui), and
[desktop CLI integration tests](../tests/integration/desktop_cli.rs).
Passing tests do not establish complete frontend parity or replace interactive
UI checks. No implementation changes were made as part of this comparison.
