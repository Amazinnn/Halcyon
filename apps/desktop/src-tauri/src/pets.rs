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

/// Parse image dimensions from file headers only (PNG IHDR / WebP VP8X/VP8L),
/// no decoding dependency. Returns (width, height).
pub fn sheet_dimensions(path: &Path) -> Result<(u32, u32), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("读取精灵图失败: {e}"))?;
    if bytes.len() >= 24 && &bytes[..8] == b"\x89PNG\r\n\x1a\n" {
        // IHDR: width at 16..20, height at 20..24 (big-endian)
        let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        return Ok((w, h));
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        let fourcc = &bytes[12..16];
        if fourcc == b"VP8X" {
            if bytes.len() < 26 {
                return Err("不支持的 WebP 变体".into());
            }
            // VP8X: canvas width-1 at bytes 20..22, height-1 at bytes 23..25 (24-bit LE)
            let w = 1 + u32::from(bytes[20]) + (u32::from(bytes[21]) << 8) + (u32::from(bytes[22]) << 16);
            let h = 1 + u32::from(bytes[23]) + (u32::from(bytes[24]) << 8) + (u32::from(bytes[25]) << 16);
            return Ok((w, h));
        }
        if fourcc == b"VP8 " {
            // VP8 lossy: 14-byte frame tag; width at offset 26..28 (14-bit LE), height 28..30
            let w = u16::from_le_bytes([bytes[26], bytes[27]]) & 0x3FFF;
            let h = u16::from_le_bytes([bytes[28], bytes[29]]) & 0x3FFF;
            return Ok((u32::from(w), u32::from(h)));
        }
        if fourcc == b"VP8L" {
            // VP8L lossless: 1-byte signature 0x2f then 14-bit width-1 / 14-bit height-1
            if bytes.len() >= 25 && bytes[20] == 0x2f {
                let bits = u32::from_le_bytes([bytes[21], bytes[22], bytes[23], bytes[24]]);
                let w = 1 + (bits & 0x3FFF);
                let h = 1 + ((bits >> 14) & 0x3FFF);
                return Ok((w, h));
            }
        }
        return Err("不支持的 WebP 变体".into());
    }
    Err("精灵图必须是 PNG 或 WebP".into())
}

/// Reject spritesheets with an opaque background (C1, ADR-0010): sample the
/// four corners and the four edge midpoints; if the 5x5 average alpha of any
/// sample exceeds 8, the background is considered non-transparent.
pub fn check_transparent_background(path: &Path) -> Result<(), String> {
    let img = image::open(path).map_err(|e| format!("spritesheet ????: {e}"))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    if w == 0 || h == 0 {
        return Err("spritesheet ????".into());
    }
    let pts: [(u32, u32); 8] = [
        (0, 0),
        (w - 1, 0),
        (0, h - 1),
        (w - 1, h - 1),
        (w / 2, 0),
        (w / 2, h - 1),
        (0, h / 2),
        (w - 1, h / 2),
    ];
    for (px, py) in pts {
        let mut sum: u32 = 0;
        let mut n: u32 = 0;
        for dy in -2i64..=2 {
            for dx in -2i64..=2 {
                let x = px as i64 + dx;
                let y = py as i64 + dy;
                if x < 0 || y < 0 || x >= w as i64 || y >= h as i64 {
                    continue;
                }
                let px = rgba.get_pixel(x as u32, y as u32);
                sum += px.0[3] as u32;
                n += 1;
            }
        }
        let avg = if n > 0 { sum / n } else { 255 };
        if avg > 8 {
            return Err("???????spritesheet ??/???????".into());
        }
    }
    Ok(())
}

/// Remove an imported pack by id (safe id only).
pub fn remove(data_dir: &Path, id: &str) -> Result<(), String> {
    if !is_valid_id(id) {
        return Err("宠物包 id 非法".into());
    }
    let dir = pets_root(data_dir).join(id);
    if !dir.is_dir() {
        return Err(format!("宠物包不存在: {id}"));
    }
    std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())
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
    let sheet = sheet_path(dir).ok_or_else(|| "宠物包缺少 spritesheet.webp 或 spritesheet.png".to_string())?;
    let (w, h) = sheet_dimensions(&sheet)?;
    if w != ATLAS_W || h != ATLAS_H {
        return Err(format!(
            "spritesheet 尺寸不符：需要 {ATLAS_W}x{ATLAS_H}，实际 {w}x{h}"
        ));
    }
    check_transparent_background(&sheet)?;
    Ok(m)
}

pub const ATLAS_COLS: usize = 8;
pub const ATLAS_ROWS: usize = 9;
pub const CELL_W: usize = 192;
pub const CELL_H: usize = 208;
const ATLAS_W: u32 = (ATLAS_COLS * CELL_W) as u32;
const ATLAS_H: u32 = (ATLAS_ROWS * CELL_H) as u32;

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

    /// Minimal valid PNG header for ATLAS_W x ATLAS_H.
    fn png_header(w: u32, h: u32) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        v.extend_from_slice(b"\x00\x00\x00\x0dIHDR");
        v.extend_from_slice(&w.to_be_bytes());
        v.extend_from_slice(&h.to_be_bytes());
        v.extend_from_slice(&[8u8, 6, 0, 0, 0]);
        v
    }

    fn write_pack(dir: &Path, id: &str, name: &str, desc: &str) {
        std::fs::create_dir_all(dir).unwrap();
        let manifest = serde_json::json!({
            "id": id,
            "displayName": name,
            "description": desc,
            "spritesheetPath": "spritesheet.png"
        });
        std::fs::write(dir.join("pet.json"), serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        let mut img = image::RgbaImage::from_pixel(1536, 1872, image::Rgba([0, 0, 0, 0]));
        img.save(dir.join("spritesheet.png")).unwrap();
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
        assert!(copied.join("spritesheet.png").exists());
        assert!(info.spritesheet_path.ends_with("spritesheet.png"));
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
        std::fs::write(
            src.join("pet.json"),
            r#"{"id":"x","displayName":"X","spritesheetPath":"spritesheet.png"}"#,
        )
        .unwrap();
        std::fs::write(src.join("spritesheet.png"), png_header(1536, 1872)).unwrap();
        assert!(import(&src, &tmp.join("data")).is_err());

        // invalid id (path traversal)
        let src2 = tmp.join("src2");
        std::fs::create_dir_all(&src2).unwrap();
        std::fs::write(
            src2.join("pet.json"),
            r#"{"id":"../evil","displayName":"X","description":"d","spritesheetPath":"spritesheet.png"}"#,
        )
        .unwrap();
        std::fs::write(src2.join("spritesheet.png"), png_header(1536, 1872)).unwrap();
        assert!(import(&src2, &tmp.join("data")).is_err());

        // missing spritesheet
        let src3 = tmp.join("src3");
        std::fs::create_dir_all(&src3).unwrap();
        std::fs::write(
            src3.join("pet.json"),
            r#"{"id":"x","displayName":"X","description":"d","spritesheetPath":"spritesheet.png"}"#,
        )
        .unwrap();
        assert!(import(&src3, &tmp.join("data")).is_err());

        // wrong spritesheet dimensions
        let src4 = tmp.join("src4");
        std::fs::create_dir_all(&src4).unwrap();
        std::fs::write(
            src4.join("pet.json"),
            r#"{"id":"x","displayName":"X","description":"d","spritesheetPath":"spritesheet.png"}"#,
        )
        .unwrap();
        std::fs::write(src4.join("spritesheet.png"), png_header(64, 64)).unwrap();
        assert!(import(&src4, &tmp.join("data")).is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sheet_dimensions_parses_png_and_webp() {
        let tmp = temp_dir("dims");
        std::fs::create_dir_all(&tmp).unwrap();
        let p = tmp.join("a.png");
        std::fs::write(&p, png_header(1536, 1872)).unwrap();
        assert_eq!(sheet_dimensions(&p).unwrap(), (1536, 1872));

        let w = tmp.join("b.webp");
        // VP8X: RIFF size(4) WEBP(4) VP8X(4) flags(1) reserved(3) w-1(3) h-1(3)
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&20u32.to_le_bytes());
        v.extend_from_slice(b"WEBP");
        v.extend_from_slice(b"VP8X");
        v.extend_from_slice(&[0u8; 10]);
        v[20] = 0xFF; // w-1 low
        v[21] = 0x05; // w-1 mid (0x5FF = 1535)
        v[22] = 0x00;
        v[23] = 0x4F; // h-1 = 0x74F = 1871
        v[24] = 0x07;
        v[25] = 0x00;
        std::fs::write(&w, &v).unwrap();
        assert_eq!(sheet_dimensions(&w).unwrap(), (1536, 1872));

        let bad = tmp.join("c.png");
        std::fs::write(&bad, b"not an image").unwrap();
        assert!(sheet_dimensions(&bad).is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn transparent_background_validation() {
        use image::{Rgba, RgbaImage};
        let tmp = temp_dir("alpha");
        std::fs::create_dir_all(&tmp).unwrap();

        // fully transparent sheet -> import ok
        let src = tmp.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("pet.json"),
            r#"{"id":"a","displayName":"A","description":"d","spritesheetPath":"spritesheet.png"}"#,
        )
        .unwrap();
        let mut clear = RgbaImage::from_pixel(1536, 1872, Rgba([0, 0, 0, 0]));
        clear.save(src.join("spritesheet.png")).unwrap();
        assert!(import(&src, &tmp.join("data")).is_ok());

        // opaque corner -> import rejected
        let src2 = tmp.join("src2");
        std::fs::create_dir_all(&src2).unwrap();
        std::fs::write(
            src2.join("pet.json"),
            r#"{"id":"b","displayName":"B","description":"d","spritesheetPath":"spritesheet.png"}"#,
        )
        .unwrap();
        let mut opaque = RgbaImage::from_pixel(1536, 1872, Rgba([0, 0, 0, 0]));
        opaque.put_pixel(0, 0, Rgba([255, 0, 255, 255]));
        opaque.save(src2.join("spritesheet.png")).unwrap();
        let err = import(&src2, &tmp.join("data")).unwrap_err();
        assert!(err.contains("??"), "unexpected error: {err}");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn remove_deletes_imported_pack() {
        let tmp = temp_dir("remove");
        let src = tmp.join("src");
        let data = tmp.join("data");
        write_pack(&src, "r.pet", "R", "remove me");
        import(&src, &data).unwrap();
        assert!(info_for(&data, "r.pet").is_ok());
        remove(&data, "r.pet").unwrap();
        assert!(info_for(&data, "r.pet").is_err());
        assert!(remove(&data, "../evil").is_err());
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
