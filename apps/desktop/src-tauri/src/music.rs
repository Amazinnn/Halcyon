//! v1.9 local music player: scan a user-chosen folder for MP3/FLAC/M4A, read
//! text tags + embedded cover via lofty, and serve playback through the Tauri
//! asset protocol (HTML5 <audio> + convertFileSrc). The asset protocol scope
//! is extended at runtime because the default scope only covers $APPDATA/**.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use lofty::file::TaggedFileExt;
use lofty::tag::{Accessor, Tag};
use serde::Serialize;

pub const AUDIO_EXTS: [&str; 3] = ["mp3", "flac", "m4a"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub path: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn stem_of(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn tag_text(
    tags: &[Tag],
    f: fn(&Tag) -> Option<Cow<'_, str>>,
) -> Option<String> {
    tags.iter().find_map(|t| f(t)).map(|c| c.into_owned())
}

/// Scan the top level of `dir` (non-recursive) for supported audio files,
/// sorted by file name. Text metadata comes from lofty and falls back to the
/// file stem when parsing fails.
pub fn list_tracks(dir: &Path) -> Vec<Track> {
    let mut files: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_file() && is_audio(&p) {
                files.push(p);
            }
        }
    }
    files.sort();
    files.into_iter().map(|p| track_for(&p)).collect()
}

fn track_for(path: &Path) -> Track {
    let path_str = path.to_string_lossy().into_owned();
    let fallback = stem_of(path);
    let (title, artist, album) = match lofty::read_from_path(path) {
        Ok(file) => {
            let tags = file.tags();
            (
                tag_text(tags, |t| Accessor::title(t)).unwrap_or(fallback),
                tag_text(tags, |t| Accessor::artist(t)),
                tag_text(tags, |t| Accessor::album(t)),
            )
        }
        Err(_) => (fallback, None, None),
    };
    Track {
        path: path_str,
        title,
        artist,
        album,
    }
}

/// Embedded cover art as a `data:` URI for the first picture found in any
/// tag, or None.
pub fn cover_data_uri(path: &str) -> Option<String> {
    let file = lofty::read_from_path(path).ok()?;
    for tag in file.tags() {
        if let Some(pic) = tag.pictures().first() {
            let mime = pic.mime_type().map(|m| m.as_str());
            if let Some(mime) = mime {
                if !mime.is_empty() {
                    use base64::Engine;
                    let b64 = base64::engine::general_purpose::STANDARD.encode(pic.data());
                    return Some(format!("data:{mime};base64,{b64}"));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> PathBuf {
        let seq = TEST_SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "focus-music-test-{}-{}",
            std::process::id(),
            seq
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn list_filters_and_sorts_top_level() {
        let dir = temp_dir();
        std::fs::write(dir.join("b.mp3"), b"x").unwrap();
        std::fs::write(dir.join("a.flac"), b"x").unwrap();
        std::fs::write(dir.join("c.m4a"), b"x").unwrap();
        std::fs::write(dir.join("d.txt"), b"x").unwrap();
        let sub = dir.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("e.mp3"), b"x").unwrap();

        let tracks = list_tracks(&dir);
        assert_eq!(tracks.len(), 3);
        // garbage bytes are not valid audio -> filename stem fallback
        assert_eq!(tracks[0].title, "a");
        assert_eq!(tracks[1].title, "b");
        assert_eq!(tracks[2].title, "c");
        assert!(tracks.iter().all(|t| t.artist.is_none() && t.album.is_none()));
    }

    #[test]
    fn cover_none_for_garbage() {
        let dir = temp_dir();
        let p = dir.join("x.mp3");
        std::fs::write(&p, b"not audio").unwrap();
        assert!(cover_data_uri(&p.to_string_lossy()).is_none());
    }
}