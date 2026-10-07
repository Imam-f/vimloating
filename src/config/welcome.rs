pub const WELCOME: &str = r#"// vimloating — a little room for your thoughts

fn main() {
    let idea = "Text doesn't have to live in a window.";
    println!("{idea}");
}

// Start exploring
//   i              enter insert mode
//   Esc            return to normal mode
//   h j k l        move left, down, up, right
//   j / k          move through wrapped rows; gj / gk move original lines
//   w b e          move by word
//   ge             end of the previous word; counts supported
//   f/t + char     find / till; F/T backward; ; / , repeat
//   .              repeat the last change
//   > / <          indent / unindent current line or visual selection
//   Alt+L / H      indent / unindent current line or visual selection
//   Alt+J / K      move line(s) down / up; match previous nonblank indentation
//   gg / G         first / last line
//   dd / yy / p    delete / yank / paste a line
//   u / Ctrl-R     undo / redo
//   v              select text, then y or d
//   Ctrl+A / Ctrl+X increment / decrement the next decimal number
//   /text, ?text   search forward / backward; n / N repeat / reverse
//   :w notes.rs    save this buffer
//   :e notes.rs    open a UTF-8 file
//   :Explore       browse files; Enter opens, - goes to the parent
//   :!git status   run a shell command (Esc closes its output)
//   :.!pwd         replace the current line with command output
//   :theme everforest / solarized-blue  change the colors
//   :ls             list buffers · :b 2 switches · :bn / :bp cycles · :bd deletes
//   :Files          fuzzy-pick files with rg · :Gfiles / :Gfiles? use Git
//   :Buffer         pick buffer · :Blines / :Lines pick lines · :Marks pick marks
//   :History        recent files · :History: commands · :History/ searches
//   :Command / :Help pick an editor command / README topic
//   Ctrl+6          switch to the last active buffer
//   <leader><leader> switch to the recent buffer · <leader>e browse files (leader: Space)
//   gf / gF         open the path[:line[:column]] under the cursor
//   Ctrl+X Ctrl+F   complete a file path in Insert mode; Ctrl+N / P cycle
//   Ctrl+N / P      complete words in Insert mode; Ctrl+X Ctrl+L completes lines
//   Completion      popup shows matches; hold Ctrl+N / P to cycle
//   Backspace      move back in Normal/Visual mode; hold to repeat
//   Insert: BS     delete before cursor; join lines at line start; hold to repeat
//   Insert: Del    delete under cursor; hold to repeat
//   Tab             complete command names and file paths in the command prompt

// Space is yours
//   Wheel          gentle zoom
//   Right drag     orbit (F4 toggles 2D-only/orbit)
//   Middle drag    pan the camera
//   F2             return to the straight-on view
//   F3             toggle the floor grid
//   F4             toggle flat-only / orbit (exits 2D into orbit)
//   F5             toggle 2D-only mode on/off
//   Ctrl+- / =     change font size
//   Ctrl+W / Ctrl+Backspace delete previous word in Insert mode
//   Ctrl+V          paste system clipboard text in Insert mode
//   Ctrl+J / K     animate five-line movement
//   Ctrl+E / Y     scroll one display line
//   Ctrl+D / U     scroll half a page; Ctrl+F / B one page
//   Idle 15s       return to Normal mode
//   Enter Enter    toggle word wrap in Normal mode
//   Ctrl+H / L     scroll horizontally (turns wrap off)
//   F1             toggle the controls overlay

// Make something worth keeping.
"#;
