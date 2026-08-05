//! v1.3 file-shortcut zone: infer type/name/id from a path and keep the
//! ordered shortcut list in Settings.shortcuts. All writes go through the
//! atomic settings.json save.

use crate::settings::{Shortcut, ShortcutType};
use std::path::Path;

/// Infer the shortcut kind from a path: existing directories are folders;
/// executables/links (.exe/.lnk/.bat/.cmd/.com) are applications; everything
/// else is a plain file.
pub fn infer_type(path: &Path) -> ShortcutType {
    if path.is_dir() {
        return ShortcutType::Folder;
    }
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
    {
        Some(e) if matches!(e.as_str(), "exe" | "lnk" | "bat" | "cmd" | "com") => {
            ShortcutType::Application
        }
        _ => ShortcutType::File,
    }
}

/// Display name = file name (fallback: full path).
pub fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Best-effort unique id: timestamp + current list length.
pub fn new_id(existing: &[Shortcut]) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("sc-{ts}-{}", existing.len())
}

/// Renumber `order` fields to match vector position.
pub fn renumber(list: &mut [Shortcut]) {
    for (i, s) in list.iter_mut().enumerate() {
        s.order = i;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_is_application() {
        assert_eq!(infer_type(Path::new("C:/Tools/app.exe")), ShortcutType::Application);
    }

    #[test]
    fn lnk_and_bat_are_applications() {
        assert_eq!(infer_type(Path::new("C:/x/Shortcut.lnk")), ShortcutType::Application);
        assert_eq!(infer_type(Path::new("C:/x/run.bat")), ShortcutType::Application);
    }

    #[test]
    fn data_file_is_file() {
        assert_eq!(infer_type(Path::new("C:/x/report.pdf")), ShortcutType::File);
    }

    #[test]
    fn existing_dir_is_folder() {
        assert_eq!(infer_type(std::env::temp_dir().as_path()), ShortcutType::Folder);
    }

    #[test]
    fn renumber_normalizes_order() {
        let mut list = vec![
            Shortcut { id: "a".into(), name: "a".into(), kind: ShortcutType::File, target: "t".into(), order: 5 },
            Shortcut { id: "b".into(), name: "b".into(), kind: ShortcutType::File, target: "t".into(), order: 2 },
        ];
        renumber(&mut list);
        assert_eq!((list[0].order, list[1].order), (0, 1));
    }

    #[test]
    fn display_name_uses_basename() {
        assert_eq!(display_name(Path::new("C:/a/b/notes.md")), "notes.md");
    }
}
