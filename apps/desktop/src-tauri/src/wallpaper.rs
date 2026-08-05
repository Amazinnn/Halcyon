//! Wallpaper import: copies a user-selected image into app_data_dir/wallpapers
//! so the app only ever loads files from its own managed directory.

use std::path::Path;

const EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

pub fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn import(src: &str, data_dir: &Path) -> Result<String, String> {
    let src_path = Path::new(src);
    if !is_supported(src_path) {
        return Err("仅支持 png / jpg / jpeg / webp 图片".to_string());
    }
    let wall_dir = data_dir.join("wallpapers");
    std::fs::create_dir_all(&wall_dir).map_err(|e| e.to_string())?;
    let ext = src_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase();
    let name = format!(
        "wallpaper-{}.{}",
        chrono::Utc::now().timestamp_millis(),
        ext
    );
    let dest = wall_dir.join(&name);
    std::fs::copy(src_path, &dest).map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().into_owned())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_copies_supported_image() {
        let tmp = std::env::temp_dir().join(format!("focus-wall-test-{}", chrono::Utc::now().timestamp_millis()));
        std::fs::create_dir_all(&tmp).unwrap();
        let src = tmp.join("sample.png");
        std::fs::write(&src, b"\x89PNG\r\n\x1a\nfake").unwrap();
        let out = import(src.to_str().unwrap(), &tmp).unwrap();
        let dest = std::path::Path::new(&out);
        assert!(dest.exists());
        assert_eq!(dest.extension().unwrap().to_str().unwrap(), "png");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn import_rejects_unsupported() {
        let tmp = std::env::temp_dir().join(format!("focus-wall-test2-{}", chrono::Utc::now().timestamp_millis()));
        std::fs::create_dir_all(&tmp).unwrap();
        let src = tmp.join("sample.exe");
        std::fs::write(&src, b"MZ").unwrap();
        assert!(import(src.to_str().unwrap(), &tmp).is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}