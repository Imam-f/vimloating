use super::Editor;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn containing_directory(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    }
}

impl Editor {
    pub fn open_path(path: PathBuf) -> Result<Self, String> {
        let metadata = fs::metadata(&path).map_err(|err| err.to_string())?;
        if metadata.is_dir() {
            let path = fs::canonicalize(&path).unwrap_or(path);
            let mut directories = Vec::new();
            let mut files = Vec::new();
            for entry in fs::read_dir(&path).map_err(|err| err.to_string())? {
                let entry = entry.map_err(|err| err.to_string())?;
                let entry_path = entry.path();
                if entry.file_type().map_err(|err| err.to_string())?.is_dir() {
                    directories.push(entry_path);
                } else {
                    files.push(entry_path);
                }
            }
            let sort_entries = |entries: &mut Vec<PathBuf>| {
                entries.sort_by_key(|entry| {
                    entry
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase()
                });
            };
            sort_entries(&mut directories);
            sort_entries(&mut files);

            let parent = path.parent().filter(|parent| *parent != path);
            let has_parent = parent.is_some();
            let mut entries = Vec::new();
            if let Some(parent) = parent {
                entries.push(parent.to_path_buf());
            }
            entries.extend(directories);
            entries.extend(files);
            let listing = entries
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                    let name = if has_parent && index == 0 {
                        "..".to_owned()
                    } else {
                        entry
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned()
                    };
                    if entry.is_dir() {
                        format!("{name}/")
                    } else {
                        name
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            let mut editor = Self::new(&listing, Some(path.clone()));
            editor.directory_entries = Some(entries);
            editor.message = format!(
                "Directory: {} · Enter opens, - goes to parent",
                path.display()
            );
            Ok(editor)
        } else {
            let text = fs::read_to_string(&path).map_err(|err| err.to_string())?;
            Ok(Self::new(&text, Some(path)))
        }
    }

    pub fn save(&mut self, path: Option<&str>) -> bool {
        if self.is_directory_browser() {
            self.message = "Cannot write a directory listing".into();
            return false;
        }
        let destination = path
            .filter(|s| !s.is_empty())
            .map(super::paths::expand_home)
            .or_else(|| self.path.clone());
        let Some(destination) = destination else {
            self.message = "No filename · use :w path/to/file".into();
            return false;
        };
        let text = self.text();
        match fs::write(&destination, &text) {
            Ok(()) => {
                self.path = Some(destination);
                self.saved = text;
                self.message = format!("Written {} · {} lines", self.name(), self.lines.len());
                true
            }
            Err(err) => {
                self.message = format!("Write failed: {err}");
                false
            }
        }
    }
}
