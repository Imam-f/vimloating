pub const WELCOME: &str = r#"// vimloating — terminal editor

// i: insert · Esc: normal · h j k l: move · w b e: words
// j/k: wrapped rows · gj/gk: original lines
// v / V: select · y / d: yank / delete · p: paste
// u / Ctrl+R: undo / redo · / or ?: search · n / N: repeat
// :w notes.rs: save · :e path: open · :Explore: browse
// :ls: buffers · :bn / :bp: switch · :q: quit · :q!: discard
// :theme everforest / solarized-blue: colors
// Enter Enter: toggle wrap · F1: terminal controls

"#;

pub(super) const HELP: &str = r#"vimloating terminal controls

i / a / I / A / o / O   Insert text
Esc / Ctrl+C           Return to Normal mode
h / l / arrows        Move; counts supported
j / k                 Move down / up through wrapped rows
gj / gk               Move down / up through original lines
w b e · gg G · f F t T  Vim motions and character-find hints
ge                    End of previous word; counts supported
gf / gF               Open path[:line[:column]] under cursor
Insert: Ctrl+X Ctrl+F  Complete file / directory path
Insert: Ctrl+X Ctrl+L  Complete whole line
Insert: Ctrl+N / P     Complete word / cycle popup matches
Hold Ctrl+N / P       Repeat completion / motion / history
Normal/Visual: Backspace Move back one character; hold to repeat
Insert: Backspace     Delete before cursor / join lines; hold to repeat
Insert: Delete        Delete under cursor; hold to repeat
Command: Tab          Complete command / path
v / V · y d x · p P    Select, yank, delete, paste
u / Ctrl+R             Undo / redo
/ or ? · n / N         Search forward/backward and repeat
:w [path] · Ctrl+S     Save
:e path · :Explore     Open file / directory browser
:ls · :b id · :bn :bp  List / switch buffers
:Files / :Gfiles       Fuzzy-pick files (rg / Git)
:Gfiles?               Pick files listed by Git status
:Buffer / :Blines      Pick a buffer / line from any buffer
:Lines / :Marks        Pick active-buffer lines / marked lines
:History / :History:   Recent files / command history
:History/              Search history
:Command / :Help       Pick an editor command / README topic
Ctrl+6 / Ctrl+^        Last active buffer (terminal dependent)
<leader><leader>       Switch to the recent buffer (leader is Space)
<leader>e              Browse the current file's directory
:!command · :.!command Shell output / filter current line
:q / :q! / :wq         Quit / discard / save and quit
Enter Enter            Toggle word wrap in Normal mode
Ctrl+E / Y             Scroll one display line
Ctrl+D / U · F / B     Half-page / full-page movement
Ctrl+H / L             Horizontal scroll (disables wrap)
Ctrl+J / K             Scroll five display lines (terminal dependent)
Alt+H / L              Unindent / indent current or selected lines
Alt+J / K              Move current or selected lines
Normal: Ctrl+A / X     Increment / decrement number; counts supported
Terminal paste         Paste in Insert mode
Output: j/k, PgUp/Down Scroll captured output / buffer list
F1 / Esc               Close this help

Font size and clipboard shortcuts are controlled by your terminal."#;
