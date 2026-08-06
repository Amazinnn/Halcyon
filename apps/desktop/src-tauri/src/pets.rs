//! Pet Pack management (ADR-0009): imports OpenAI hatch-pet artifacts
//! (`pet.json` + `spritesheet.webp`, fixed 8x9 / 192x208 contract) into the
//! app data dir `pet-packs/<id>/`, validates the manifest, and resolves the
//! currently active pack. Rendering/playback lives in the frontend.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const PETS_DIR: &str = "pet-packs";
const SHEET_EXTENSIONS: &[&str] = &["webp", "png"];

/// Raw `pet.json` shape from the hatch-pet contract (camelCase fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetManifest {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub spritesheet_path: String,
}

/// Imported/resolved pack as seen by the frontend; spritesheet_path is an
/// absolute path the renderer can pass through `convertFileSrc`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetInfo {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub spritesheet_path: String,
}

/// A hatch-pet id is a safe directory name (no path separators / traversal).
pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && id != "."
        && id != ".."
}

fn sheet_path(dir: &Path) -> Option<PathBuf> {
    for ext in SHEET_EXTENSIONS {
        let p = dir.join(format!("spritesheet.{ext}"));
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

/// Load and validate `pet.json` in `dir`.
pub fn load_manifest(dir: &Path) -> Result<PetManifest, String> {
    let manifest_path = dir.join("pet.json");
    let text = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("pet.json 读取失败: {e}"))?;
    let m: PetManifest =
        serde_json::from_str(&text).map_err(|e| format!("pet.json 解析失败: {e}"))?;
    if !is_valid_id(&m.id) {
        return Err("pet.json id 非法（需字母/数字/-/_/.，≤64 字符）".into());
    }
    if m.display_name.trim().is_empty() {
        return Err("pet.json 缺少 displayName".into());
    }
    if m.description.trim().is_empty() {
        return Err("pet.json 缺少 description".into());
    }
    if m.spritesheet_path.trim().is_empty() {
        return Err("pet.json 缺少 spritesheetPath".into());
    }
    if sheet_path(dir).is_none() {
        return Err("宠物包缺少 spritesheet.webp 或 spritesheet.png".into());
    }
    Ok(m)
}

fn to_info(m: &PetManifest, dir: &Path) -> Result<PetInfo, String> {
    let sheet = sheet_path(dir).ok_or_else(|| "宠物包缺少 spritesheet".to_string())?;
    Ok(PetInfo {
        id: m.id.clone(),
        display_name: m.display_name.clone(),
        description: m.description.clone(),
        spritesheet_path: sheet.to_string_lossy().into_owned(),
    })
}

fn pets_root(data_dir: &Path) -> PathBuf {
    data_dir.join(PETS_DIR)
}

/// Recursively copy `src` into `dest` (both dirs), creating `dest` as needed.
fn copy_dir(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let target = dest.join(entry.file_name());
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        if ty.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else if ty.is_file() {
            std::fs::copy(entry.path(), &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Validate `src_dir` (must contain pet.json + spritesheet) and copy it into
/// `data_dir/pet-packs/<id>/` (replacing any previous pack with the same id).
pub fn import(src_dir: &Path, data_dir: &Path) -> Result<PetInfo, String> {
    if !src_dir.is_dir() {
        return Err("所选路径不是文件夹".into());
    }
    let m = load_manifest(src_dir)?;
    let root = pets_root(data_dir);
    let dest = root.join(&m.id);
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    if dest.exists() {
        std::fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
    }
    copy_dir(src_dir, &dest)?;
    // Re-read from the copied location so spritesheet_path points at app data.
    let copied = load_manifest(&dest)?;
    to_info(&copied, &dest)
}

/// List all imported packs (id/displayName/description + sheet path).
pub fn list(data_dir: &Path) -> Result<Vec<PetInfo>, String> {
    let root = pets_root(data_dir);
    if !root.is_dir() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        if let Ok(m) = load_manifest(&dir) {
            if let Ok(info) = to_info(&m, &dir) {
                out.push(info);
            }
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// Resolve one pack by id; errors when missing or invalid.
pub fn info_for(data_dir: &Path, id: &str) -> Result<PetInfo, String> {
    if !is_valid_id(id) {
        return Err("宠物包 id 非法".into());
    }
    let dir = pets_root(data_dir).join(id);
    if !dir.is_dir() {
        return Err(format!("宠物包不存在: {id}"));
    }
    let m = load_manifest(&dir)?;
    to_info(&m, &dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "focus-pets-test-{}-{}",
            tag,
            chrono::Utc::now().timestamp_millis()
        ))
    }

    fn write_pack(dir: &Path, id: &str, name: &str, desc: &str) {
        std::fs::create_dir_all(dir).unwrap();
        let manifest = serde_json::json!({
            "id": id,
            "displayName": name,
            "description": desc,
            "spritesheetPath": "spritesheet.webp"
        });
        std::fs::write(dir.join("pet.json"), serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        std::fs::write(dir.join("spritesheet.webp"), b"fake-webp").unwrap();
    }

    #[test]
    fn valid_manifest_passes_and_imports() {
        let tmp = temp_dir("valid");
        let src = tmp.join("src");
        let data = tmp.join("data");
        write_pack(&src, "my.pet", "My Pet", "A test pet");
        let info = import(&src, &data).unwrap();
        assert_eq!(info.id, "my.pet");
        assert_eq!(info.display_name, "My Pet");
        let copied = data.join(PETS_DIR).join("my.pet");
        assert!(copied.join("pet.json").exists());
        assert!(copied.join("spritesheet.webp").exists());
        assert!(info.spritesheet_path.ends_with("spritesheet.webp"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn import_rejects_invalid_manifest() {
        let tmp = temp_dir("invalid");
        let src = tmp.join("src");
        std::fs::create_dir_all(&src).unwrap();
        // missing description
        std::fs::write(
            src.join("pet.json"),
            r#"{"id":"x","displayName":"X","spritesheetPath":"spritesheet.webp"}"#,
        )
        .unwrap();
        std::fs::write(src.join("spritesheet.webp"), b"fake").unwrap();
        assert!(import(&src, &tmp.join("data")).is_err());

        // invalid id (path traversal)
        let src2 = tmp.join("src2");
        std::fs::create_dir_all(&src2).unwrap();
        std::fs::write(
            src2.join("pet.json"),
            r#"{"id":"../evil","displayName":"X","description":"d","spritesheetPath":"spritesheet.webp"}"#,
        )
        .unwrap();
        std::fs::write(src2.join("spritesheet.webp"), b"fake").unwrap();
        assert!(import(&src2, &tmp.join("data")).is_err());

        // missing spritesheet
        let src3 = tmp.join("src3");
        std::fs::create_dir_all(&src3).unwrap();
        std::fs::write(
            src3.join("pet.json"),
            r#"{"id":"x","displayName":"X","description":"d","spritesheetPath":"spritesheet.webp"}"#,
        )
        .unwrap();
        assert!(import(&src3, &tmp.join("data")).is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn list_and_info_for_resolve_imported_packs() {
        let tmp = temp_dir("list");
        let src = tmp.join("src");
        let data = tmp.join("data");
        write_pack(&src, "a.pet", "A", "first");
        import(&src, &data).unwrap();
        let src2 = tmp.join("src2");
        write_pack(&src2, "b.pet", "B", "second");
        import(&src2, &data).unwrap();
        let all = list(&data).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, "a.pet");
        assert_eq!(all[1].id, "b.pet");
        let b = info_for(&data, "b.pet").unwrap();
        assert_eq!(b.display_name, "B");
        assert!(info_for(&data, "missing").is_err());
        assert!(info_for(&data, "../evil").is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn import_replaces_existing_pack_same_id() {
        let tmp = temp_dir("replace");
        let src = tmp.join("src");
        let data = tmp.join("data");
        write_pack(&src, "same.pet", "V1", "old");
        import(&src, &data).unwrap();
        write_pack(&src, "same.pet", "V2", "new");
        import(&src, &data).unwrap();
        let info = info_for(&data, "same.pet").unwrap();
        assert_eq!(info.display_name, "V2");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}