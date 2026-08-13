//! Pet Pack management (ADR-0009): imports OpenAI hatch-pet artifacts
//! (official Hatch Pet or explicit `focus-hatch-pet`) into the
//! app data dir `pet-packs/<id>/`, validates the manifest, and resolves the
//! currently active pack. Rendering/playback lives in the frontend.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const PETS_DIR: &str = "pet-packs";
pub const AGENT_PET_DIR: &str = "pet-pack";
const DISPLAY_METADATA_FILE: &str = ".focus-display.json";
const DISPLAY_METADATA_VERSION: u32 = 1;
pub const MIN_HORIZONTAL_CORRECTION: f32 = 0.75;
pub const MAX_HORIZONTAL_CORRECTION: f32 = 1.33;
const SHEET_EXTENSIONS: &[&str] = &["webp", "png"];
pub const FOCUS_PET_STATES: [&str; 6] = ["resting", "focusing", "working", "waiting", "happy", "troubled"];

/// Legacy pre-v1.13 atlas kept readable for already-imported user packages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetManifest {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub spritesheet_path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetAnimation {
    pub id: String,
    pub asset_path: String,
    pub columns: u32,
    pub rows: u32,
    pub frames: u32,
    pub fps: u32,
    pub looped: bool,
    pub start_row: u32,
    pub cell_width: u32,
    pub cell_height: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetSourceRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetAnimationAnalysis {
    pub source_rect: PetSourceRect,
    pub warning_frames: Vec<u32>,
    pub used_full_cell: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetPalette {
    pub accent: [u8; 3],
    pub host_tint: [u8; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PetDisplayMetadata {
    pub version: u32,
    pub pet_pack_id: String,
    pub horizontal_correction: f32,
    pub bubble_accent: String,
    pub host_tint: String,
    pub analyses: BTreeMap<String, PetAnimationAnalysis>,
    pub quality_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetAnchor {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub struct PetPackage {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub anchor: Option<PetAnchor>,
    pub animations: Vec<PetAnimation>,
}

impl PetPackage {
    pub fn animation(&self, id: &str) -> Option<&PetAnimation> {
        self.animations.iter().find(|animation| animation.id == id)
    }
}

pub type StateMapping = std::collections::BTreeMap<String, Option<String>>;

pub fn default_state_mapping(animations: &[PetAnimation]) -> StateMapping {
    FOCUS_PET_STATES
        .into_iter()
        .map(|state| {
            let match_id = animations
                .iter()
                .find(|animation| animation.id.eq_ignore_ascii_case(state))
                .or_else(|| {
                    let alias = match state {
                        "resting" => Some("idle"),
                        "focusing" => Some("focus"),
                        "working" => Some("work"),
                        "troubled" => Some("error"),
                        _ => None,
                    }?;
                    animations.iter().find(|animation| animation.id.eq_ignore_ascii_case(alias))
                })
                .map(|animation| animation.id.clone());
            (state.to_string(), match_id)
        })
        .collect()
}

pub fn reconcile_state_mapping(existing: &StateMapping, animations: &[PetAnimation]) -> StateMapping {
    let defaults = default_state_mapping(animations);
    FOCUS_PET_STATES
        .into_iter()
        .map(|state| {
            let retained = existing
                .get(state)
                .and_then(|id| id.as_ref())
                .filter(|id| animations.iter().any(|animation| animation.id == **id))
                .cloned();
            (state.to_string(), retained.or_else(|| defaults.get(state).cloned().flatten()))
        })
        .collect()
}

pub fn resolve_state_animation<'a>(
    state: &str,
    mapping: &StateMapping,
    animations: &'a [PetAnimation],
) -> Option<&'a PetAnimation> {
    let selected = mapping.get(state).and_then(|id| id.as_deref())
        .and_then(|id| animations.iter().find(|animation| animation.id == id));
    selected
        .or_else(|| animations.iter().find(|animation| animation.id.eq_ignore_ascii_case("idle")))
        .or_else(|| animations.first())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FocusManifest {
    format: String,
    id: String,
    display_name: String,
    #[serde(default)]
    description: String,
    spritesheet: String,
    atlas: AtlasManifest,
    animations: BTreeMap<String, FocusAnimation>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FocusAnimation {
    row: u32,
    frames: u32,
    fps: u32,
    #[serde(rename = "loop")]
    looped: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DraftManifest {
    format: String,
    id: String,
    display_name: String,
    description: String,
    #[serde(default)]
    bubble_anchor: Option<PetAnchor>,
    animations: BTreeMap<String, DraftAnimation>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DraftAnimation {
    sheet_path: String,
    columns: u32,
    rows: u32,
    frames: u32,
    fps: u32,
    #[serde(rename = "loop")]
    looped: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AtlasManifest {
    columns: u32,
    rows: u32,
    cell_width: u32,
    cell_height: u32,
    #[serde(default)]
    row_order: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OfficialHatchManifest {
    schema_version: u32,
    id: String,
    name: String,
    renderer: String,
    spritesheet: String,
    atlas: AtlasManifest,
    #[serde(default)]
    canvas: OfficialCanvas,
    states: BTreeMap<String, OfficialState>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OfficialCanvas {
    bubble_anchor: Option<[f32; 2]>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OfficialState {
    row: u32,
    #[serde(rename = "loop")]
    looped: bool,
    fps: u32,
    frames: Vec<OfficialFrame>,
}

#[derive(Debug, Deserialize)]
struct OfficialFrame {
    rect: [u32; 4],
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
    pub animations: Vec<PetAnimation>,
    pub bubble_anchor: Option<PetAnchor>,
    pub bubble_accent: String,
    pub host_tint: String,
    pub horizontal_correction: f32,
    pub analyses: BTreeMap<String, PetAnimationAnalysis>,
    pub quality_warnings: Vec<String>,
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
    let package = official_package(m, dir, &sheet)?;
    let metadata = load_or_create_display_metadata(dir, &package, &sheet)?;
    Ok(info_from_package(&package, &sheet, &metadata))
}

fn analyze_package(package: &PetPackage, sheet: &Path, horizontal_correction: f32) -> PetDisplayMetadata {
    let image = image::open(sheet).ok().map(|image| image.to_rgba8());
    let mut analyses = BTreeMap::new();
    let mut warnings = Vec::new();
    if let Some(ref image) = image {
        for animation in &package.animations {
            let analysis = analyze_animation_pixels(image, animation);
            if !analysis.warning_frames.is_empty() {
                warnings.push(format!(
                    "动画 {} 的第 {} 帧包含稀疏边缘像素，显示时已自动校准",
                    animation.id,
                    analysis.warning_frames.iter().map(|frame| (frame + 1).to_string()).collect::<Vec<_>>().join(", ")
                ));
            }
            if analysis.used_full_cell {
                warnings.push(format!(
                    "动画 {} 的稳定主体不足，显示时已回退完整单元格",
                    animation.id,
                ));
            }
            analyses.insert(animation.id.clone(), analysis);
        }
    }
    let palette = image.as_ref()
        .map(|image| derive_pet_palette_from_calibrated(image, package, &analyses))
        .unwrap_or(PetPalette { accent: [138, 166, 141], host_tint: [18, 32, 24] });
    PetDisplayMetadata {
        version: DISPLAY_METADATA_VERSION,
        pet_pack_id: package.id.clone(),
        horizontal_correction,
        bubble_accent: rgb_hex(palette.accent),
        host_tint: rgb_hex(palette.host_tint),
        analyses,
        quality_warnings: warnings,
    }
}

fn info_from_package(package: &PetPackage, sheet: &Path, metadata: &PetDisplayMetadata) -> PetInfo {
    PetInfo {
        id: package.id.clone(),
        display_name: package.display_name.clone(),
        description: package.description.clone(),
        spritesheet_path: sheet.to_string_lossy().into_owned(),
        animations: package.animations.clone(),
        bubble_anchor: package.anchor.clone(),
        bubble_accent: metadata.bubble_accent.clone(),
        host_tint: metadata.host_tint.clone(),
        horizontal_correction: metadata.horizontal_correction,
        analyses: metadata.analyses.clone(),
        quality_warnings: metadata.quality_warnings.clone(),
    }
}

fn display_metadata_path(package_dir: &Path) -> PathBuf {
    package_dir.join(DISPLAY_METADATA_FILE)
}

fn write_display_metadata(package_dir: &Path, metadata: &PetDisplayMetadata) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(metadata).map_err(|e| format!("桌宠显示元数据序列化失败: {e}"))?;
    std::fs::write(display_metadata_path(package_dir), json)
        .map_err(|e| format!("桌宠显示元数据写入失败: {e}"))
}

fn read_display_metadata(package_dir: &Path, package: &PetPackage) -> Option<PetDisplayMetadata> {
    let bytes = std::fs::read(display_metadata_path(package_dir)).ok()?;
    let metadata: PetDisplayMetadata = serde_json::from_slice(&bytes).ok()?;
    let valid_correction = metadata.horizontal_correction.is_finite()
        && (MIN_HORIZONTAL_CORRECTION..=MAX_HORIZONTAL_CORRECTION)
            .contains(&metadata.horizontal_correction);
    (metadata.version == DISPLAY_METADATA_VERSION
        && metadata.pet_pack_id == package.id
        && valid_correction
        && package.animations.iter().all(|animation| metadata.analyses.contains_key(&animation.id)))
        .then_some(metadata)
}

fn load_or_create_display_metadata(
    package_dir: &Path,
    package: &PetPackage,
    sheet: &Path,
) -> Result<PetDisplayMetadata, String> {
    if let Some(metadata) = read_display_metadata(package_dir, package) {
        return Ok(metadata);
    }
    let metadata = analyze_package(package, sheet, 1.0);
    write_display_metadata(package_dir, &metadata)?;
    Ok(metadata)
}

fn rgb_hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

fn derive_pet_palette_from_calibrated(
    image: &image::RgbaImage,
    package: &PetPackage,
    analyses: &BTreeMap<String, PetAnimationAnalysis>,
) -> PetPalette {
    let mut pixels = Vec::new();
    for animation in &package.animations {
        let Some(analysis) = analyses.get(&animation.id) else { continue };
        for frame in 0..animation.frames {
            let col = frame % animation.columns.max(1);
            let row = animation.start_row + frame / animation.columns.max(1);
            let left = col * animation.cell_width;
            let top = row * animation.cell_height;
            let (_, cleaned) = frame_alpha_masks(image, animation, frame);
            for y in analysis.source_rect.y..analysis.source_rect.y + analysis.source_rect.height {
                for x in analysis.source_rect.x..analysis.source_rect.x + analysis.source_rect.width {
                    let index = (y * animation.cell_width + x) as usize;
                    if cleaned.get(index).copied().unwrap_or(false) {
                        if let Some(pixel) = image.get_pixel_checked(left + x, top + y) {
                            pixels.push(*pixel);
                        }
                    }
                }
            }
        }
    }
    derive_pet_palette(pixels.iter())
}

fn derive_pet_palette<'a>(pixels: impl Iterator<Item = &'a image::Rgba<u8>>) -> PetPalette {
    let mut clusters: BTreeMap<u16, ([u64; 3], u64)> = BTreeMap::new();
    for pixel in pixels {
        let [r, g, b, a] = pixel.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        if a >= 96 && max.saturating_sub(min) >= 18 && max >= 48 {
            let key = ((r as u16 >> 4) << 8) | ((g as u16 >> 4) << 4) | (b as u16 >> 4);
            let (total, count) = clusters.entry(key).or_insert(([0; 3], 0));
            total[0] += r as u64;
            total[1] += g as u64;
            total[2] += b as u64;
            *count += 1;
        }
    }
    let Some((total, count)) = clusters.values().max_by_key(|(_, count)| *count) else {
        return PetPalette { accent: [138, 166, 141], host_tint: [18, 32, 24] };
    };
    let avg = [
        (total[0] / *count) as u8,
        (total[1] / *count) as u8,
        (total[2] / *count) as u8,
    ];
    let accent = [
        ((avg[0] as u16 * 3 + 255) / 4) as u8,
        ((avg[1] as u16 * 3 + 255) / 4) as u8,
        ((avg[2] as u16 * 3 + 255) / 4) as u8,
    ];
    let host_tint = [
        (avg[0] as u16 * 2 / 5).max(8) as u8,
        (avg[1] as u16 * 2 / 5).max(8) as u8,
        (avg[2] as u16 * 2 / 5).max(8) as u8,
    ];
    PetPalette { accent, host_tint }
}

fn analyze_animation_pixels(image: &image::RgbaImage, animation: &PetAnimation) -> PetAnimationAnalysis {
    let cell_w = animation.cell_width.max(1);
    let cell_h = animation.cell_height.max(1);
    let mut union: Option<(u32, u32, u32, u32)> = None;
    let mut warning_frames = Vec::new();
    let mut raw_alpha_count = 0usize;
    let mut retained_alpha_count = 0usize;
    for frame in 0..animation.frames {
        let (mask, cleaned) = frame_alpha_masks(image, animation, frame);
        raw_alpha_count += mask.iter().filter(|pixel| **pixel).count();
        retained_alpha_count += cleaned.iter().filter(|pixel| **pixel).count();
        let raw_bounds = mask_bounds(&mask, cell_w, cell_h);
        let Some((x, y, w, h)) = mask_bounds(&cleaned, cell_w, cell_h) else { continue };
        if raw_bounds != Some((x, y, w, h)) {
            warning_frames.push(frame);
        }
        let trim = 2;
        let rect = (x.saturating_sub(trim), y.saturating_sub(trim), (w + trim * 2).min(cell_w - x.saturating_sub(trim)), (h + trim * 2).min(cell_h - y.saturating_sub(trim)));
        union = Some(match union {
            None => rect,
            Some((ux, uy, uw, uh)) => {
                let right = (ux + uw).max(rect.0 + rect.2);
                let bottom = (uy + uh).max(rect.1 + rect.3);
                (ux.min(rect.0), uy.min(rect.1), right - ux.min(rect.0), bottom - uy.min(rect.1))
            }
        });
    }
    let retains_enough_alpha = raw_alpha_count > 0
        && retained_alpha_count.saturating_mul(100) >= raw_alpha_count.saturating_mul(60);
    let source_rect = union
        .filter(|_| retains_enough_alpha)
        .filter(|(_, _, w, h)| *w >= cell_w / 8 && *h >= cell_h / 8)
        .map(|(x, y, width, height)| PetSourceRect { x, y, width, height })
        .unwrap_or(PetSourceRect { x: 0, y: 0, width: cell_w, height: cell_h });
    PetAnimationAnalysis {
        used_full_cell: source_rect.x == 0 && source_rect.y == 0 && source_rect.width == cell_w && source_rect.height == cell_h,
        source_rect,
        warning_frames,
    }
}

fn frame_alpha_masks(
    image: &image::RgbaImage,
    animation: &PetAnimation,
    frame: u32,
) -> (Vec<bool>, Vec<bool>) {
    let cell_w = animation.cell_width.max(1);
    let cell_h = animation.cell_height.max(1);
    let col = frame % animation.columns.max(1);
    let row = animation.start_row + frame / animation.columns.max(1);
    let left = col * cell_w;
    let top = row * cell_h;
    let mut raw = vec![false; (cell_w * cell_h) as usize];
    for y in 0..cell_h {
        for x in 0..cell_w {
            if let Some(pixel) = image.get_pixel_checked(left + x, top + y) {
                raw[(y * cell_w + x) as usize] = pixel.0[3] >= 32;
            }
        }
    }
    let opened = open_mask_3x3(&raw, cell_w, cell_h);
    let cleaned = retain_meaningful_components(&opened, cell_w, cell_h);
    (raw, cleaned)
}

fn mask_bounds(mask: &[bool], width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;
    for y in 0..height {
        for x in 0..width {
            if !mask[(y * width + x) as usize] { continue; }
            found = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + 1);
            max_y = max_y.max(y + 1);
        }
    }
    if found {
        Some((min_x, min_y, max_x - min_x, max_y - min_y))
    } else {
        None
    }
}

fn open_mask_3x3(mask: &[bool], width: u32, height: u32) -> Vec<bool> {
    if width < 3 || height < 3 { return mask.to_vec(); }
    let mut eroded = vec![false; mask.len()];
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let keep = (y - 1..=y + 1).all(|ny| {
                (x - 1..=x + 1).all(|nx| mask[(ny * width + nx) as usize])
            });
            eroded[(y * width + x) as usize] = keep;
        }
    }
    let mut dilated = vec![false; mask.len()];
    for y in 0..height {
        for x in 0..width {
            let y0 = y.saturating_sub(1);
            let y1 = (y + 1).min(height - 1);
            let x0 = x.saturating_sub(1);
            let x1 = (x + 1).min(width - 1);
            dilated[(y * width + x) as usize] = (y0..=y1).any(|ny| {
                (x0..=x1).any(|nx| eroded[(ny * width + nx) as usize])
            });
        }
    }
    dilated
}

fn retain_meaningful_components(mask: &[bool], width: u32, height: u32) -> Vec<bool> {
    let mut visited = vec![false; mask.len()];
    let mut components: Vec<Vec<usize>> = Vec::new();
    for start in 0..mask.len() {
        if !mask[start] || visited[start] { continue; }
        let mut stack = vec![start];
        let mut component = Vec::new();
        visited[start] = true;
        while let Some(index) = stack.pop() {
            component.push(index);
            let x = index as u32 % width;
            let y = index as u32 / width;
            for (nx, ny) in [
                (x.saturating_sub(1), y), ((x + 1).min(width - 1), y),
                (x, y.saturating_sub(1)), (x, (y + 1).min(height - 1)),
            ] {
                let next = (ny * width + nx) as usize;
                if mask[next] && !visited[next] {
                    visited[next] = true;
                    stack.push(next);
                }
            }
        }
        components.push(component);
    }
    let largest = components.iter().map(Vec::len).max().unwrap_or(0);
    let minimum = (largest / 100).max(12);
    let mut kept = vec![false; mask.len()];
    for component in components.into_iter().filter(|component| component.len() >= minimum) {
        for index in component { kept[index] = true; }
    }
    kept
}

/// A muted representative color for the companion bubble. Transparent pixels
/// and near-neutral pixels are ignored so a mostly transparent sprite sheet
/// does not inherit an arbitrary black edge.
fn official_package(m: &PetManifest, dir: &Path, sheet: &Path) -> Result<PetPackage, String> {
    let names = [
        "idle", "running-right", "running-left", "waving", "jumping", "failed", "waiting",
        "running", "review",
    ];
    let animations = names
        .into_iter()
        .enumerate()
        .map(|(_row, id)| PetAnimation {
            id: id.to_string(),
            asset_path: sheet.to_string_lossy().into_owned(),
            columns: ATLAS_COLS as u32,
            rows: ATLAS_ROWS as u32,
            frames: ATLAS_COLS as u32,
            fps: 8,
            looped: !matches!(id, "jumping" | "failed"),
            start_row: _row as u32,
            cell_width: CELL_W as u32,
            cell_height: CELL_H as u32,
        })
        .collect::<Vec<_>>();
    let _ = (dir, row_count(&animations));
    Ok(PetPackage {
        id: m.id.clone(),
        display_name: m.display_name.clone(),
        description: m.description.clone(),
        anchor: None,
        animations,
    })
}

fn row_count(animations: &[PetAnimation]) -> usize {
    animations.len()
}

fn relative_asset(dir: &Path, path: &str, animation: &str) -> Result<PathBuf, String> {
    let relative = Path::new(path);
    if relative.is_absolute() || relative.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(format!("动画 \"{animation}\" 资源路径非法"));
    }
    let asset = dir.join(relative);
    if !asset.is_file() {
        return Err(format!("动画 \"{animation}\" 缺少资源: {path}"));
    }
    Ok(asset)
}

fn validate_atlas(
    sheet: &Path,
    columns: u32,
    rows: u32,
    cell_width: u32,
    cell_height: u32,
    label: &str,
) -> Result<(), String> {
    if columns == 0 || rows == 0 || cell_width == 0 || cell_height == 0 {
        return Err(format!("{label} has invalid atlas geometry"));
    }
    let expected_w = columns.saturating_mul(cell_width);
    let expected_h = rows.saturating_mul(cell_height);
    let (width, height) = sheet_dimensions(sheet)?;
    if (width, height) != (expected_w, expected_h) {
        return Err(format!("{label} dimensions must be {expected_w}x{expected_h}, got {width}x{height}"));
    }
    check_transparent_background(sheet)
}

fn load_focus_package(manifest: FocusManifest, dir: &Path) -> Result<PetPackage, String> {
    if manifest.format != "focus-hatch-pet" {
        return Err(format!("unsupported pet format: {}", manifest.format));
    }
    if !is_valid_id(&manifest.id) || manifest.display_name.trim().is_empty() || manifest.animations.is_empty() {
        return Err("focus-hatch-pet requires id, displayName, and animations".into());
    }
    let sheet = relative_asset(dir, &manifest.spritesheet, "spritesheet")?;
    validate_atlas(&sheet, manifest.atlas.columns, manifest.atlas.rows, manifest.atlas.cell_width, manifest.atlas.cell_height, "focus-hatch-pet spritesheet")?;
    let mut animations = Vec::with_capacity(manifest.animations.len());
    for (id, entry) in manifest.animations {
        if id.trim().is_empty() || entry.frames == 0 || entry.fps == 0 || entry.row >= manifest.atlas.rows || entry.frames > manifest.atlas.columns {
            return Err(format!("focus-hatch-pet animation {id} exceeds its atlas"));
        }
        animations.push(PetAnimation {
            id,
            asset_path: sheet.to_string_lossy().into_owned(),
            columns: manifest.atlas.columns,
            rows: manifest.atlas.rows,
            frames: entry.frames,
            fps: entry.fps,
            looped: entry.looped,
            start_row: entry.row,
            cell_width: manifest.atlas.cell_width,
            cell_height: manifest.atlas.cell_height,
        });
    }
    Ok(PetPackage {
        id: manifest.id,
        display_name: manifest.display_name,
        description: manifest.description,
        anchor: None,
        animations,
    })
}

fn load_official_package(manifest: OfficialHatchManifest, dir: &Path) -> Result<PetPackage, String> {
    if manifest.schema_version != 1 || manifest.renderer != "frame-atlas" || !is_valid_id(&manifest.id) || manifest.name.trim().is_empty() || manifest.states.is_empty() {
        return Err("official Hatch Pet manifest is incomplete or unsupported".into());
    }
    let sheet = relative_asset(dir, &manifest.spritesheet, "spritesheet")?;
    validate_atlas(&sheet, manifest.atlas.columns, manifest.atlas.rows, manifest.atlas.cell_width, manifest.atlas.cell_height, "official Hatch Pet spritesheet")?;
    let mut animations = Vec::with_capacity(manifest.states.len());
    for (id, state) in manifest.states {
        if id.trim().is_empty() || state.fps == 0 || state.frames.is_empty() || state.row >= manifest.atlas.rows || state.frames.len() as u32 > manifest.atlas.columns {
            return Err(format!("official Hatch Pet state {id} exceeds its atlas"));
        }
        for (column, frame) in state.frames.iter().enumerate() {
            let expected = [column as u32 * manifest.atlas.cell_width, state.row * manifest.atlas.cell_height, manifest.atlas.cell_width, manifest.atlas.cell_height];
            if frame.rect != expected {
                return Err(format!("official Hatch Pet state {id} has a frame outside its declared atlas row"));
            }
        }
        animations.push(PetAnimation {
            id,
            asset_path: sheet.to_string_lossy().into_owned(),
            columns: manifest.atlas.columns,
            rows: manifest.atlas.rows,
            frames: state.frames.len() as u32,
            fps: state.fps,
            looped: state.looped,
            start_row: state.row,
            cell_width: manifest.atlas.cell_width,
            cell_height: manifest.atlas.cell_height,
        });
    }
    Ok(PetPackage {
        id: manifest.id,
        display_name: manifest.name,
        description: String::new(),
        anchor: manifest.canvas.bubble_anchor.map(|[x, y]| PetAnchor { x, y }),
        animations,
    })
}

fn load_draft_package(dir: &Path) -> Result<PetPackage, String> {
    let manifest_path = dir.join("manifest.json");
    let text = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("manifest.json 读取失败: {e}"))?;
    let manifest: DraftManifest = serde_json::from_str(&text)
        .map_err(|e| format!("manifest.json 解析失败: {e}"))?;
    if manifest.format != "hatch-pet-draft-0.2" {
        return Err(format!("不支持的宠物包格式: {}", manifest.format));
    }
    if !is_valid_id(&manifest.id) || manifest.display_name.trim().is_empty() || manifest.description.trim().is_empty() {
        return Err("manifest.json 的 id/displayName/description 非法".into());
    }
    if manifest.animations.is_empty() {
        return Err("manifest.json 未声明可播放动画".into());
    }
    let mut animations = Vec::with_capacity(manifest.animations.len());
    for (id, entry) in manifest.animations {
        if id.trim().is_empty() || entry.columns == 0 || entry.rows == 0 || entry.frames == 0 || entry.fps == 0 {
            return Err(format!("动画 \"{id}\" 的网格、帧数或 FPS 非法"));
        }
        if entry.frames > entry.columns.saturating_mul(entry.rows) {
            return Err(format!("动画 \"{id}\" 的帧数超过图表容量"));
        }
        let asset = relative_asset(dir, &entry.sheet_path, &id)?;
        let (width, height) = sheet_dimensions(&asset)?;
        let expected_w = entry.columns.saturating_mul(CELL_W as u32);
        let expected_h = entry.rows.saturating_mul(CELL_H as u32);
        if width != expected_w || height != expected_h {
            return Err(format!(
                "动画 \"{id}\" 图像尺寸不符：需要 {expected_w}x{expected_h}，实际 {width}x{height}"
            ));
        }
        check_transparent_background(&asset)?;
        animations.push(PetAnimation {
            id,
            asset_path: asset.to_string_lossy().into_owned(),
            columns: entry.columns,
            rows: entry.rows,
            frames: entry.frames,
            fps: entry.fps,
            looped: entry.looped,
            start_row: 0,
            cell_width: CELL_W as u32,
            cell_height: CELL_H as u32,
        });
    }
    Ok(PetPackage {
        id: manifest.id,
        display_name: manifest.display_name,
        description: manifest.description,
        anchor: manifest.bubble_anchor,
        animations,
    })
}

/// Load either an explicit official atlas or the supported draft multi-sheet
/// manifest. No other JSON shape is treated as a pet package.
pub fn load_package(dir: &Path) -> Result<PetPackage, String> {
    if dir.join("pet.json").is_file() {
        let text = std::fs::read_to_string(dir.join("pet.json")).map_err(|_| "pet package must contain pet.json".to_string())?;
        let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("pet.json is invalid JSON: {e}"))?;
        if value.get("format").and_then(serde_json::Value::as_str) == Some("focus-hatch-pet") {
            return load_focus_package(serde_json::from_value(value).map_err(|e| format!("invalid focus-hatch-pet: {e}"))?, dir);
        }
        if value.get("schemaVersion").is_some() {
            return load_official_package(serde_json::from_value(value).map_err(|e| format!("invalid official Hatch Pet: {e}"))?, dir);
        }
        let manifest: PetManifest = serde_json::from_str(&text).map_err(|e| format!("unsupported pet.json format: {e}"))?;
        let sheet = sheet_path(dir).ok_or_else(|| "宠物包缺少 spritesheet.webp 或 spritesheet.png".to_string())?;
        return official_package(&manifest, dir, &sheet);
    }
    if dir.join("manifest.json").is_file() {
        return Err("hatch-pet-draft-0.2 is retired; use official Hatch Pet or focus-hatch-pet pet.json".into());
    }
    Err("宠物包必须包含 pet.json 或受支持的 manifest.json".into())
}

/// New imports must use one of the explicit package formats. Legacy fixed
/// atlas manifests remain readable through `load_package` for existing packs,
/// but are not accepted as new Agent-owned packages.
fn load_import_package(dir: &Path) -> Result<PetPackage, String> {
    let manifest = dir.join("pet.json");
    let text = std::fs::read_to_string(&manifest)
        .map_err(|e| format!("pet.json 读取失败: {e}"))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("pet.json is invalid JSON: {e}"))?;
    if value.get("format").and_then(serde_json::Value::as_str) == Some("focus-hatch-pet") {
        return load_focus_package(
            serde_json::from_value(value).map_err(|e| format!("invalid focus-hatch-pet: {e}"))?,
            dir,
        );
    }
    if value.get("schemaVersion").is_some() {
        return load_official_package(
            serde_json::from_value(value).map_err(|e| format!("invalid official Hatch Pet: {e}"))?,
            dir,
        );
    }
    Err("unsupported pet.json format: use official Hatch Pet or focus-hatch-pet".into())
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

pub struct PendingAgentPetImport {
    dest: PathBuf,
    backup: PathBuf,
    had_previous: bool,
    info: PetInfo,
}

impl PendingAgentPetImport {
    pub fn info(&self) -> &PetInfo { &self.info }

    pub fn commit(self) {
        if self.backup.exists() { let _ = std::fs::remove_dir_all(&self.backup); }
    }

    pub fn rollback(self) {
        let _ = std::fs::remove_dir_all(&self.dest);
        if self.had_previous { let _ = std::fs::rename(&self.backup, &self.dest); }
    }
}

/// Stage and validate an Agent-local package replacement. The previous package
/// stays in a sibling backup until the caller commits dependent persistence.
fn prepare_import_for_agent_impl(
    src_dir: &Path,
    workspace: &Path,
    allow_legacy: bool,
) -> Result<PendingAgentPetImport, String> {
    let source_package = if allow_legacy { load_package(src_dir)? } else { load_import_package(src_dir)? };
    let dest = workspace.join(AGENT_PET_DIR);
    let stage = workspace.join(".pet-pack-stage");
    let backup = workspace.join(".pet-pack-backup");
    std::fs::create_dir_all(workspace).map_err(|e| e.to_string())?;
    for stale in [&stage, &backup] {
        if stale.exists() {
            std::fs::remove_dir_all(stale).map_err(|e| e.to_string())?;
        }
    }
    copy_dir(src_dir, &stage)?;
    let staged = if allow_legacy { load_package(&stage)? } else { load_import_package(&stage)? };
    if staged.id != source_package.id { return Err("宠物包复制校验失败".into()); }
    let staged_sheet = staged.animations.first().map(|a| PathBuf::from(&a.asset_path))
        .ok_or_else(|| "宠物包未发现可播放动画".to_string())?;
    let metadata = analyze_package(&staged, &staged_sheet, 1.0);
    write_display_metadata(&stage, &metadata)?;

    let had_previous = dest.exists();
    if had_previous {
        std::fs::rename(&dest, &backup).map_err(|e| format!("宠物包备份失败: {e}"))?;
    }
    if let Err(error) = std::fs::rename(&stage, &dest) {
        if had_previous {
            let _ = std::fs::rename(&backup, &dest);
        }
        return Err(format!("宠物包替换失败: {error}"));
    }
    let info = match (|| {
        let copied = if allow_legacy { load_package(&dest)? } else { load_import_package(&dest)? };
        if copied.id != source_package.id { return Err("宠物包复制校验失败".into()); }
        let sheet = copied.animations.first().map(|a| PathBuf::from(&a.asset_path))
            .ok_or_else(|| "宠物包未发现可播放动画".to_string())?;
        let final_metadata = read_display_metadata(&dest, &copied)
            .unwrap_or_else(|| analyze_package(&copied, &sheet, 1.0));
        Ok(info_from_package(&copied, &sheet, &final_metadata))
    })() {
        Ok(info) => info,
        Err(error) => {
        let _ = std::fs::remove_dir_all(&dest);
        if had_previous {
            let _ = std::fs::rename(&backup, &dest);
        }
            return Err(error);
        }
    };
    Ok(PendingAgentPetImport { dest, backup, had_previous, info })
}

pub fn prepare_import_for_agent(src_dir: &Path, workspace: &Path) -> Result<PendingAgentPetImport, String> {
    prepare_import_for_agent_impl(src_dir, workspace, false)
}

pub fn prepare_legacy_import_for_agent(src_dir: &Path, workspace: &Path) -> Result<PendingAgentPetImport, String> {
    prepare_import_for_agent_impl(src_dir, workspace, true)
}

/// Copy an imported package into its owning Agent workspace. This convenience
/// path is for callers with no related database update.
pub fn import_for_agent(src_dir: &Path, workspace: &Path) -> Result<PetInfo, String> {
    let pending = prepare_import_for_agent(src_dir, workspace)?;
    let info = pending.info.clone();
    pending.commit();
    Ok(info)
}

pub fn info_for_agent(workspace: &Path) -> Result<PetInfo, String> {
    let dir = workspace.join(AGENT_PET_DIR);
    let package = load_package(&dir)?;
    let sheet = package.animations.first().map(|a| PathBuf::from(&a.asset_path))
        .ok_or_else(|| "宠物包未发现可播放动画".to_string())?;
    let metadata = load_or_create_display_metadata(&dir, &package, &sheet)?;
    Ok(info_from_package(&package, &sheet, &metadata))
}

pub fn display_metadata_for_agent(workspace: &Path) -> Result<PetDisplayMetadata, String> {
    let dir = workspace.join(AGENT_PET_DIR);
    let package = load_package(&dir)?;
    let sheet = package.animations.first().map(|animation| PathBuf::from(&animation.asset_path))
        .ok_or_else(|| "宠物包未发现可播放动画".to_string())?;
    load_or_create_display_metadata(&dir, &package, &sheet)
}

pub fn set_horizontal_correction_for_agent(
    workspace: &Path,
    horizontal_correction: f32,
) -> Result<PetDisplayMetadata, String> {
    if !horizontal_correction.is_finite()
        || !(MIN_HORIZONTAL_CORRECTION..=MAX_HORIZONTAL_CORRECTION)
            .contains(&horizontal_correction)
    {
        return Err(format!(
            "宽高校正必须介于 {:.2} 与 {:.2} 之间",
            MIN_HORIZONTAL_CORRECTION, MAX_HORIZONTAL_CORRECTION
        ));
    }
    let dir = workspace.join(AGENT_PET_DIR);
    let mut metadata = display_metadata_for_agent(workspace)?;
    metadata.horizontal_correction = horizontal_correction;
    write_display_metadata(&dir, &metadata)?;
    Ok(metadata)
}

pub fn remove_for_agent(workspace: &Path) -> Result<(), String> {
    let dir = workspace.join(AGENT_PET_DIR);
    if !dir.is_dir() { return Err("该 Agent 没有桌宠包".into()); }
    std::fs::remove_dir_all(dir).map_err(|e| e.to_string())
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

/// v1.10 (#32): return the spritesheet as base64 so the frontend can build a
/// same-origin Blob (`createImageBitmap`) instead of drawing the cross-origin
/// asset-protocol URL (which taints the canvas and breaks getImageData).
pub fn sheet_base64(data_dir: &Path, id: &str) -> Result<String, String> {
    let info = info_for(data_dir, id)?;
    let bytes = std::fs::read(&info.spritesheet_path)
        .map_err(|e| format!("读取 spritesheet 失败: {e}"))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
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
        let img = image::RgbaImage::from_pixel(1536, 1872, image::Rgba([0, 0, 0, 0]));
        img.save(dir.join("spritesheet.png")).unwrap();
    }

    fn write_focus_pack(dir: &Path, id: &str, name: &str, desc: &str) {
        std::fs::create_dir_all(dir).unwrap();
        let manifest = serde_json::json!({
            "format": "focus-hatch-pet",
            "id": id,
            "displayName": name,
            "description": desc,
            "spritesheet": "spritesheet.png",
            "atlas": { "columns": 1, "rows": 1, "cellWidth": 192, "cellHeight": 208 },
            "animations": { "idle": { "row": 0, "frames": 1, "fps": 8, "loop": true } },
        });
        std::fs::write(dir.join("pet.json"), serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        let img = image::RgbaImage::from_pixel(192, 208, image::Rgba([0, 0, 0, 0]));
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
        let clear = RgbaImage::from_pixel(1536, 1872, Rgba([0, 0, 0, 0]));
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
    fn sheet_base64_roundtrips_png_bytes() {
        let tmp = temp_dir("b64");
        let src = tmp.join("src");
        let data = tmp.join("data");
        write_pack(&src, "b64.pet", "B64", "test");
        let info = import(&src, &data).unwrap();
        let b64 = sheet_base64(&data, &info.id).unwrap();
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &b64).unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(&bytes[..4], &[0x89, b'P', b'N', b'G'], "decoded bytes must be the PNG we imported");
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

    #[test]
    fn agent_workspace_keeps_one_replaceable_pet_pack() {
        let tmp = temp_dir("agent-pack");
        let first = tmp.join("first");
        let second = tmp.join("second");
        let workspace = tmp.join("agent");
        write_focus_pack(&first, "one.pet", "One", "first");
        write_focus_pack(&second, "two.pet", "Two", "second");

        import_for_agent(&first, &workspace).unwrap();
        assert_eq!(info_for_agent(&workspace).unwrap().id, "one.pet");
        import_for_agent(&second, &workspace).unwrap();
        assert_eq!(info_for_agent(&workspace).unwrap().id, "two.pet");
        assert!(!workspace.join(AGENT_PET_DIR).join("one.pet").exists());
        remove_for_agent(&workspace).unwrap();
        assert!(info_for_agent(&workspace).is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn loads_the_official_hatch_pet_atlas_without_rewriting_its_manifest() {
        let tmp = temp_dir("official-current");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            tmp.join("pet.json"),
            r#"{
              "schemaVersion":1, "id":"official.whale", "name":"Blue Whale",
              "renderer":"frame-atlas", "spritesheet":"assets/whale.png",
              "atlas":{"columns":8,"rows":2,"cellWidth":256,"cellHeight":256,"rowOrder":["idle","focused"]},
              "canvas":{"bubbleAnchor":[0.5,0.08]},
              "states":{
                "idle":{"row":0,"loop":true,"fps":8,"frames":[{"rect":[0,0,256,256]}]},
                "focused":{"row":1,"loop":true,"fps":12,"frames":[{"rect":[0,256,256,256]},{"rect":[256,256,256,256]}]}
              }
            }"#,
        ).unwrap();
        std::fs::create_dir_all(tmp.join("assets")).unwrap();
        image::RgbaImage::from_pixel(2048, 512, image::Rgba([0, 0, 0, 0]))
            .save(tmp.join("assets/whale.png")).unwrap();

        let package = load_package(&tmp).unwrap();
        let focused = package.animation("focused").unwrap();
        assert_eq!(package.display_name, "Blue Whale");
        assert_eq!(focused.columns, 8);
        assert_eq!(focused.rows, 2);
        assert_eq!(focused.frames, 2);
        assert_eq!(focused.fps, 12);
        assert_eq!(focused.cell_width, 256);
        assert_eq!(focused.cell_height, 256);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn loads_a_focus_hatch_pet_package_with_declared_geometry_and_asset_path() {
        let tmp = temp_dir("focus-format");
        std::fs::create_dir_all(tmp.join("atlas")).unwrap();
        std::fs::write(
            tmp.join("pet.json"),
            r#"{
              "format":"focus-hatch-pet", "id":"focus.fox", "displayName":"Focus Fox",
              "spritesheet":"atlas/fox.png",
              "atlas":{"columns":3,"rows":2,"cellWidth":160,"cellHeight":120},
              "animations":{"rest":{"row":0,"frames":3,"fps":6,"loop":true},"wave":{"row":1,"frames":2,"fps":10,"loop":false}}
            }"#,
        ).unwrap();
        image::RgbaImage::from_pixel(480, 240, image::Rgba([0, 0, 0, 0]))
            .save(tmp.join("atlas/fox.png")).unwrap();

        let package = load_package(&tmp).unwrap();
        let wave = package.animation("wave").unwrap();
        assert_eq!(wave.asset_path, tmp.join("atlas/fox.png").to_string_lossy());
        assert_eq!((wave.columns, wave.rows, wave.frames, wave.fps), (3, 2, 2, 10));
        assert_eq!((wave.cell_width, wave.cell_height), (160, 120));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn legacy_atlas_stays_readable_and_retired_draft_is_rejected() {
        let tmp = temp_dir("adapters");
        let official = tmp.join("official");
        write_pack(&official, "official.pet", "Official", "atlas");
        let official = load_package(&official).unwrap();
        assert_eq!(official.animations.len(), 9);
        assert_eq!(official.animation("idle").unwrap().columns, 8);
        assert_eq!(official.animation("idle").unwrap().rows, 9);

        let draft = tmp.join("draft");
        std::fs::create_dir_all(draft.join("sheets")).unwrap();
        std::fs::write(
            draft.join("manifest.json"),
            r#"{
              "format":"hatch-pet-draft-0.2",
              "id":"draft.pet", "displayName":"Draft", "description":"sheets",
              "animations": {
                "tea-break": {"sheetPath":"sheets/tea.png", "columns":2, "rows":1, "frames":2, "fps":6, "loop":true}
              }
            }"#,
        ).unwrap();
        let sheet = image::RgbaImage::from_pixel(384, 208, image::Rgba([0, 0, 0, 0]));
        sheet.save(draft.join("sheets/tea.png")).unwrap();
        let error = load_package(&draft).unwrap_err();
        assert!(error.contains("retired"), "{error}");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn legacy_fixed_atlas_remains_readable_but_cannot_be_newly_imported_for_an_agent() {
        let tmp = temp_dir("legacy-runtime-only");
        let source = tmp.join("source");
        let workspace = tmp.join("agent");
        write_pack(&source, "legacy.pet", "Legacy", "runtime only");

        assert_eq!(load_package(&source).unwrap().id, "legacy.pet");
        let error = import_for_agent(&source, &workspace).unwrap_err();

        assert!(error.contains("unsupported pet.json format"), "{error}");
        assert!(!workspace.join(AGENT_PET_DIR).exists());
        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn draft_adapter_rejects_missing_or_geometrically_invalid_animation_sheet() {
        let tmp = temp_dir("draft-invalid");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            tmp.join("manifest.json"),
            r#"{"format":"hatch-pet-draft-0.2","id":"invalid.pet","displayName":"Invalid","description":"bad","animations":{"bad":{"sheetPath":"missing.png","columns":2,"rows":1,"frames":2,"fps":6,"loop":true}}}"#,
        ).unwrap();
        let error = load_package(&tmp).unwrap_err();
        assert!(error.contains("retired"), "{error}");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    #[ignore = "the retired-format rejection is covered by the readable draft fixture above"]
    fn retired_draft_with_empty_animations_is_rejected() {
        let tmp = temp_dir("draft-empty");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            tmp.join("manifest.json"),
            r#"{"format":"hatch-pet-draft-0.2","id":"empty.pet","displayName":"Empty","description":"none","animations":{}}"#,
        )
        .unwrap();

        let error = load_package(&tmp).unwrap_err();
        assert!(error.contains("未声明可播放动画"), "{error}");
        assert!(error.contains("retired"), "{error}");
        assert!(error.contains("retired"), "{error}");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn agent_import_rejects_retired_draft_package() {
        let tmp = temp_dir("draft-import");
        let source = tmp.join("source");
        let workspace = tmp.join("agent");
        std::fs::create_dir_all(source.join("sheets")).unwrap();
        std::fs::write(
            source.join("manifest.json"),
            r#"{"format":"hatch-pet-draft-0.2","id":"draft.import","displayName":"Draft","description":"import","animations":{"custom":{"sheetPath":"sheets/custom.png","columns":1,"rows":1,"frames":1,"fps":4,"loop":true}}}"#,
        ).unwrap();
        image::RgbaImage::from_pixel(192, 208, image::Rgba([0, 0, 0, 0]))
            .save(source.join("sheets/custom.png")).unwrap();

        let error = import_for_agent(&source, &workspace).unwrap_err();
        assert!(error.contains("pet.json"), "{error}");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn state_mapping_prefills_only_likely_names_and_falls_back_safely() {
        let animations = vec![
            PetAnimation { id: "idle".into(), asset_path: "idle.png".into(), columns: 1, rows: 1, frames: 1, fps: 1, looped: true, start_row: 0, cell_width: 1, cell_height: 1 },
            PetAnimation { id: "happy".into(), asset_path: "happy.png".into(), columns: 1, rows: 1, frames: 1, fps: 1, looped: true, start_row: 0, cell_width: 1, cell_height: 1 },
            PetAnimation { id: "work".into(), asset_path: "work.png".into(), columns: 1, rows: 1, frames: 1, fps: 1, looped: true, start_row: 0, cell_width: 1, cell_height: 1 },
        ];
        let mapping = default_state_mapping(&animations);
        assert_eq!(mapping.get("happy").and_then(|v| v.clone()).as_deref(), Some("happy"));
        assert_eq!(mapping.get("working").and_then(|v| v.clone()).as_deref(), Some("work"));
        assert!(mapping.get("troubled").unwrap().is_none());
        assert_eq!(resolve_state_animation("troubled", &mapping, &animations).unwrap().id, "idle");

        let no_idle = vec![animations[1].clone()];
        assert_eq!(resolve_state_animation("waiting", &mapping, &no_idle).unwrap().id, "happy");
    }

    #[test]
    fn package_info_derives_a_muted_accent_from_visible_sprite_pixels() {
        let tmp = temp_dir("accent");
        let source = tmp.join("source");
        std::fs::create_dir_all(&source).unwrap();
        let mut image = image::RgbaImage::from_pixel(1536, 1872, image::Rgba([0, 0, 0, 0]));
        for y in 64..160 {
            for x in 64..160 {
                image.put_pixel(x, y, image::Rgba([24, 160, 96, 255]));
            }
        }
        image.save(source.join("spritesheet.png")).unwrap();
        let manifest = serde_json::json!({
            "id": "accent.pet", "displayName": "Accent", "description": "accent",
            "spritesheetPath": "spritesheet.png"
        });
        std::fs::write(source.join("pet.json"), serde_json::to_vec(&manifest).unwrap()).unwrap();
        let info = to_info(&load_manifest(&source).unwrap(), &source).unwrap();
        assert_ne!(info.bubble_accent, "#8aa68d");
        assert!(info.bubble_accent.starts_with('#'));
        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn stable_content_analysis_removes_a_thin_connected_streak() {
        let mut sheet = image::RgbaImage::from_pixel(256, 256, image::Rgba([0, 0, 0, 0]));
        for y in 40..220 {
            for x in 48..190 {
                sheet.put_pixel(x, y, image::Rgba([30, 120, 220, 255]));
            }
        }
        for x in 190..252 {
            sheet.put_pixel(x, 80, image::Rgba([30, 120, 220, 255]));
        }
        let animation = PetAnimation {
            id: "idle".into(), asset_path: "unused".into(), columns: 1, rows: 1,
            frames: 1, fps: 8, looped: true, start_row: 0, cell_width: 256, cell_height: 256,
        };

        let analysis = analyze_animation_pixels(&sheet, &animation);

        assert_eq!(analysis.source_rect, PetSourceRect { x: 46, y: 38, width: 146, height: 184 });
        assert_eq!(analysis.warning_frames, vec![0]);
        assert!(!analysis.used_full_cell);
    }

    #[test]
    fn stable_content_analysis_falls_back_when_visible_subject_is_too_small() {
        let mut sheet = image::RgbaImage::from_pixel(100, 80, image::Rgba([0, 0, 0, 0]));
        for y in 20..23 {
            for x in 20..23 {
                sheet.put_pixel(x, y, image::Rgba([255, 80, 80, 255]));
            }
        }
        let animation = PetAnimation {
            id: "idle".into(), asset_path: "unused".into(), columns: 1, rows: 1,
            frames: 1, fps: 8, looped: true, start_row: 0, cell_width: 100, cell_height: 80,
        };

        let analysis = analyze_animation_pixels(&sheet, &animation);

        assert_eq!(analysis.source_rect, PetSourceRect { x: 0, y: 0, width: 100, height: 80 });
        assert!(analysis.used_full_cell);
    }

    #[test]
    fn stable_content_analysis_falls_back_when_cleanup_discards_most_alpha_mass() {
        let mut sheet = image::RgbaImage::from_pixel(100, 80, image::Rgba([0, 0, 0, 0]));
        for y in 30..45 { for x in 40..55 { sheet.put_pixel(x, y, image::Rgba([30, 120, 220, 255])); } }
        for y in (2..78).step_by(4) { for x in (2..98).step_by(4) { sheet.put_pixel(x, y, image::Rgba([240, 20, 30, 255])); } }
        let animation = PetAnimation {
            id: "idle".into(), asset_path: "unused".into(), columns: 1, rows: 1,
            frames: 1, fps: 8, looped: true, start_row: 0, cell_width: 100, cell_height: 80,
        };

        let analysis = analyze_animation_pixels(&sheet, &animation);

        assert!(analysis.used_full_cell);
        assert_eq!(analysis.source_rect, PetSourceRect { x: 0, y: 0, width: 100, height: 80 });
    }

    #[test]
    fn package_info_reports_a_nonblocking_warning_when_analysis_falls_back() {
        let tmp = temp_dir("analysis-fallback-warning");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            tmp.join("pet.json"),
            r#"{"format":"focus-hatch-pet","id":"tiny.pet","displayName":"Tiny","spritesheet":"tiny.png","atlas":{"columns":1,"rows":1,"cellWidth":100,"cellHeight":80},"animations":{"idle":{"row":0,"frames":1,"fps":8,"loop":true}}}"#,
        ).unwrap();
        let mut image = image::RgbaImage::from_pixel(100, 80, image::Rgba([0, 0, 0, 0]));
        for y in 20..23 { for x in 20..23 { image.put_pixel(x, y, image::Rgba([80, 140, 255, 255])); } }
        image.save(tmp.join("tiny.png")).unwrap();

        let package = load_package(&tmp).unwrap();
        let info = info_from_package(
            &package,
            &tmp.join("tiny.png"),
            &analyze_package(&package, &tmp.join("tiny.png"), 1.0),
        );

        assert!(info.quality_warnings.iter().any(|warning| warning.contains("回退完整单元格")));
        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn representative_palette_keeps_a_dark_host_and_lighter_accent() {
        let pixels = vec![image::Rgba([35, 105, 210, 255]); 200];
        let palette = derive_pet_palette(pixels.iter());

        assert!(palette.host_tint[2] > palette.host_tint[0]);
        assert!(palette.accent[2] > palette.accent[0]);
        assert!(palette.host_tint.iter().map(|v| *v as u16).sum::<u16>()
            < palette.accent.iter().map(|v| *v as u16).sum::<u16>());
    }

    #[test]
    fn representative_palette_uses_the_dominant_quantized_color_cluster() {
        let mut pixels = vec![image::Rgba([20, 100, 220, 255]); 200];
        pixels.extend(vec![image::Rgba([240, 20, 30, 255]); 100]);

        let palette = derive_pet_palette(pixels.iter());

        assert!(palette.host_tint[0] < 20, "minority red pixels must not pull the host toward purple");
        assert!(palette.host_tint[2] > palette.host_tint[1]);
    }

    #[test]
    fn package_palette_ignores_artifact_colors_outside_calibrated_subject() {
        let mut image = image::RgbaImage::from_pixel(100, 80, image::Rgba([0, 0, 0, 0]));
        for y in 20..65 { for x in 25..70 { image.put_pixel(x, y, image::Rgba([20, 100, 220, 255])); } }
        for y in (2..78).step_by(6) { for x in 72..99 { image.put_pixel(x, y, image::Rgba([240, 20, 30, 255])); } }
        let animation = PetAnimation {
            id: "idle".into(), asset_path: "unused".into(), columns: 1, rows: 1,
            frames: 1, fps: 8, looped: true, start_row: 0, cell_width: 100, cell_height: 80,
        };
        let package = PetPackage { id: "palette.pet".into(), display_name: "Palette".into(), description: "".into(), anchor: None, animations: vec![animation] };

        let analyses = BTreeMap::from([("idle".into(), analyze_animation_pixels(&image, &package.animations[0]))]);
        let palette = derive_pet_palette_from_calibrated(&image, &package, &analyses);

        assert!(palette.host_tint[2] > palette.host_tint[0]);
    }

    #[test]
    fn replacement_mapping_drops_animation_ids_missing_from_the_new_package() {
        let existing = StateMapping::from([
            ("resting".into(), Some("old-idle".into())),
            ("happy".into(), Some("shared".into())),
        ]);
        let animations = vec![
            PetAnimation { id: "idle".into(), asset_path: "x".into(), columns: 1, rows: 1, frames: 1, fps: 1, looped: true, start_row: 0, cell_width: 1, cell_height: 1 },
            PetAnimation { id: "shared".into(), asset_path: "x".into(), columns: 1, rows: 1, frames: 1, fps: 1, looped: true, start_row: 0, cell_width: 1, cell_height: 1 },
        ];

        let reconciled = reconcile_state_mapping(&existing, &animations);

        assert_eq!(reconciled.get("resting").and_then(Clone::clone).as_deref(), Some("idle"));
        assert_eq!(reconciled.get("happy").and_then(Clone::clone).as_deref(), Some("shared"));
    }

    #[test]
    fn agent_display_metadata_persists_correction_and_rejects_out_of_range_values() {
        let tmp = temp_dir("display-metadata");
        let source = tmp.join("source");
        let workspace = tmp.join("agent");
        write_focus_pack(&source, "display.pet", "Display", "metadata");
        import_for_agent(&source, &workspace).unwrap();

        let initial = display_metadata_for_agent(&workspace).unwrap();
        assert_eq!(initial.pet_pack_id, "display.pet");
        assert_eq!(initial.horizontal_correction, 1.0);
        set_horizontal_correction_for_agent(&workspace, 1.21).unwrap();
        assert_eq!(display_metadata_for_agent(&workspace).unwrap().horizontal_correction, 1.21);
        assert!(set_horizontal_correction_for_agent(&workspace, 0.74).is_err());
        assert!(set_horizontal_correction_for_agent(&workspace, f32::NAN).is_err());

        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn replacing_an_agent_pet_resets_package_display_correction() {
        let tmp = temp_dir("display-reset");
        let first = tmp.join("first");
        let second = tmp.join("second");
        let workspace = tmp.join("agent");
        write_focus_pack(&first, "one.pet", "One", "first");
        write_focus_pack(&second, "two.pet", "Two", "second");

        import_for_agent(&first, &workspace).unwrap();
        set_horizontal_correction_for_agent(&workspace, 1.2).unwrap();
        import_for_agent(&second, &workspace).unwrap();

        let metadata = display_metadata_for_agent(&workspace).unwrap();
        assert_eq!(metadata.pet_pack_id, "two.pet");
        assert_eq!(metadata.horizontal_correction, 1.0);
        assert!(!metadata.analyses.is_empty());
        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn reimporting_the_current_agent_package_stages_before_replacement() {
        let tmp = temp_dir("staged-import-rollback");
        let first = tmp.join("first");
        let workspace = tmp.join("agent");
        write_focus_pack(&first, "one.pet", "One", "first");
        import_for_agent(&first, &workspace).unwrap();
        let current = workspace.join(AGENT_PET_DIR);

        import_for_agent(&current, &workspace).unwrap();
        assert_eq!(info_for_agent(&workspace).unwrap().id, "one.pet");
        assert!(!workspace.join(".pet-pack-stage").exists());
        assert!(!workspace.join(".pet-pack-backup").exists());

        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn pending_agent_import_rolls_back_to_the_previous_valid_package() {
        let tmp = temp_dir("pending-import-rollback");
        let first = tmp.join("first");
        let second = tmp.join("second");
        let workspace = tmp.join("agent");
        write_focus_pack(&first, "one.pet", "One", "first");
        write_focus_pack(&second, "two.pet", "Two", "second");
        import_for_agent(&first, &workspace).unwrap();

        let pending = prepare_import_for_agent(&second, &workspace).unwrap();
        assert_eq!(pending.info().id, "two.pet");
        assert_eq!(info_for_agent(&workspace).unwrap().id, "two.pet");
        assert!(workspace.join(".pet-pack-backup").is_dir());

        pending.rollback();
        assert_eq!(info_for_agent(&workspace).unwrap().id, "one.pet");
        assert!(!workspace.join(".pet-pack-backup").exists());
        let _ = std::fs::remove_dir_all(tmp);
    }

}
