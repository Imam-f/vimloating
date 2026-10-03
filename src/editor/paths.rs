use super::{BufferAction, Editor, Pos};
use std::{
    fs,
    ops::Range,
    path::{Path, PathBuf},
};

pub(super) fn expand_home(path: &str) -> PathBuf {
    if path == "~" || path.starts_with("~/") || path.starts_with("~\\") {
        #[cfg(windows)]
        let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
        #[cfg(not(windows))]
        let home = std::env::var_os("HOME");
        if let Some(home) = home {
            return PathBuf::from(home).join(path.get(2..).unwrap_or(""));
        }
    }
    PathBuf::from(path)
}

pub(super) fn path_completions(prefix: &str, base: &Path) -> Vec<String> {
    let (directory_prefix, name_prefix) = match prefix.rfind(['/', '\\']) {
        Some(index) => (&prefix[..=index], &prefix[index + 1..]),
        None => ("", prefix),
    };
    let directory = base.join(expand_home(directory_prefix));
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let name_prefix = name_prefix.to_lowercase();
    let mut matches = Vec::new();
    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if !name.to_lowercase().starts_with(&name_prefix) {
            continue;
        }
        let mut candidate = format!("{directory_prefix}{name}");
        if entry.path().is_dir() {
            candidate.push(if directory_prefix.contains('\\') {
                '\\'
            } else {
                '/'
            });
        }
        matches.push(candidate);
    }
    matches.sort_by_cached_key(|candidate| (candidate.to_lowercase(), candidate.clone()));
    matches
}

fn boundary(ch: char) -> bool {
    ch.is_whitespace()
        || matches!(
            ch,
            '"' | '\''
                | '`'
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | '<'
                | '>'
                | ','
                | ';'
                | '='
                | '|'
        )
}

// Quoted paths can contain spaces. All positions are character indices.
pub(super) fn path_range(line: &[char], col: usize) -> Option<Range<usize>> {
    let mut scan = 0;
    while scan < line.len() {
        if matches!(line[scan], '"' | '\'' | '`') {
            let quote = line[scan];
            let start = scan + 1;
            let end = (start..line.len())
                .find(|&i| line[i] == quote)
                .unwrap_or(line.len());
            let mut address_end = end.saturating_add(1);
            if line.get(address_end) == Some(&':') {
                while line
                    .get(address_end)
                    .is_some_and(|ch| ch.is_ascii_digit() || *ch == ':')
                {
                    address_end += 1;
                }
            }
            if (start <= col && col < end) || (end < col && col < address_end) {
                return Some(start..end);
            }
            scan = end.saturating_add(1);
        } else {
            scan += 1;
        }
    }
    if line.get(col).is_none_or(|&ch| boundary(ch)) {
        return None;
    }
    let mut start = col;
    while start > 0 && !boundary(line[start - 1]) {
        start -= 1;
    }
    let mut end = col + 1;
    while end < line.len() && !boundary(line[end]) {
        end += 1;
    }
    Some(start..end)
}

fn split_address(address: &str) -> (&str, Option<Pos>) {
    let Some((prefix, last)) = address.rsplit_once(':') else {
        return (address, None);
    };
    let Ok(last) = last.parse::<usize>() else {
        return (address, None);
    };
    if let Some((path, line)) = prefix.rsplit_once(':')
        && let Ok(line) = line.parse::<usize>()
    {
        return (
            path,
            Some(Pos {
                row: line.saturating_sub(1),
                col: last.saturating_sub(1),
            }),
        );
    }
    // A Windows drive prefix (C:123) is not a line address.
    if prefix.len() == 1 && prefix.as_bytes()[0].is_ascii_alphabetic() {
        return (address, None);
    }
    (
        prefix,
        Some(Pos {
            row: last.saturating_sub(1),
            col: 0,
        }),
    )
}

impl Editor {
    pub(super) fn path_directory(&self) -> PathBuf {
        self.path
            .as_deref()
            .map(|path| {
                if self.is_directory_browser() {
                    path
                } else {
                    path.parent()
                        .filter(|parent| !parent.as_os_str().is_empty())
                        .unwrap_or_else(|| Path::new("."))
                }
            })
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    }

    pub fn open_cursor_path(&mut self) {
        if self.is_directory_browser() {
            self.open_directory_entry();
            return;
        }
        let line = &self.lines[self.cursor.row];
        let Some(range) = path_range(line, self.cursor.col) else {
            self.message = "No file path under cursor".into();
            return;
        };
        let mut address: String = line[range.clone()].iter().collect();
        // A diagnostic can put its address after a quoted filename.
        if line
            .get(range.end)
            .is_some_and(|ch| matches!(ch, '"' | '\'' | '`'))
            && line.get(range.end + 1) == Some(&':')
        {
            address.extend(
                line[range.end + 1..]
                    .iter()
                    .take_while(|ch| ch.is_ascii_digit() || **ch == ':'),
            );
        }
        let (filename, position) = split_address(&address);
        if filename.is_empty() {
            self.message = "No file path under cursor".into();
            return;
        }
        let expanded = expand_home(filename);
        let relative = self.path_directory().join(&expanded);
        let path = if relative.exists() {
            relative
        } else {
            expanded
        };
        if !path.exists() {
            self.message = format!("File not found: {filename}");
            return;
        }
        self.buffer_action = Some(BufferAction::OpenAddress { path, position });
    }
}
