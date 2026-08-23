//! Pet manifest loading and normalization.
//!
//! This module converts several user-facing selectors into one normalized
//! in-memory `Pet`: built-in catalog ids, custom pet ids under CODEX_HOME,
//! legacy avatar directories, and explicit filesystem paths used by tests or
//! local iteration.
//!
//! The key invariant is that every returned `Pet` points at a local
//! spritesheet path that already exists and has app-compatible dimensions.
//! Asset acquisition is intentionally out of scope here; callers must ensure a
//! built-in pet has been downloaded before asking the model layer to load it.
//!
//! 本模块把内置 ID、CODEX_HOME 下的自定义 ID、旧版头像目录及显式路径统一规范化为 `Pet`。
//! 返回模型的核心不变量是本地精灵表已经存在且尺寸兼容；资源获取刻意不在此层，
//! 调用方必须先下载内置资源再加载模型。

use std::collections::HashMap;
use std::fs;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use serde::Deserialize;
use sha2::Digest as _;
use sha2::Sha256;

use super::catalog;

const MAX_PET_FRAMES: usize = 256;
const MAX_ANIMATION_FPS: f64 = 60.0;

/// 单个动画帧引用精灵网格索引，并携带该帧持续时间。
#[derive(Debug, Clone)]
pub struct AnimationFrame {
    pub sprite_index: usize,
    pub duration: Duration,
}

/// 规范化后的动画轨道，包括循环起点与一次性动画的回退名称。
#[derive(Debug, Clone)]
pub struct Animation {
    pub frames: Vec<AnimationFrame>,
    pub loop_start: Option<usize>,
    pub fallback: String,
}

impl Animation {
    pub(super) fn total_duration(&self) -> Duration {
        self.frames
            .iter()
            .map(|frame| frame.duration)
            .sum::<Duration>()
    }
}

/// One named animation track for a pet spritesheet.
///
/// Tracks use sprite indices into the already-decoded frame grid plus a
/// fallback animation name for one-shot sequences. Callers should not assume
/// an animation loops just because it has multiple frames; `loop_start == None`
/// means the final frame eventually hands off to `fallback`.
/// 宠物精灵表的一条命名动画轨道。轨道引用已解码网格中的索引并可声明一次性序列的回退动画；
/// 多帧不等于循环，`loop_start == None` 表示最终帧结束后转交给 `fallback`。
/// 已完成路径、几何、动画和缓存键规范化的本地宠物定义。
#[derive(Debug, Clone)]
pub struct Pet {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub spritesheet_path: PathBuf,
    pub frame_width: u32,
    pub frame_height: u32,
    pub columns: u32,
    pub rows: u32,
    pub frame_count: usize,
    pub animations: HashMap<String, Animation>,
}

impl Pet {
    /// Load a pet selector into a concrete local pet definition.
    ///
    /// Selectors may name a built-in catalog pet, a custom pet id, a legacy
    /// avatar id, or an explicit path. This method assumes any built-in asset
    /// has already been materialized into CODEX_HOME; if callers skip the
    /// asset-fetch step, they will get a missing-spritesheet error here on
    /// first use.
    /// 把用户选择器加载为具体本地宠物。选择器可为内置 ID、自定义 ID、旧版头像 ID 或显式路径；
    /// 内置资源必须已写入 CODEX_HOME，否则首次使用会返回精灵表缺失错误。
    pub(super) fn load_with_codex_home(value: &str, codex_home: Option<&Path>) -> Result<Self> {
        if path_like(value) {
            return load_pet_path(value);
        }

        if let Some(custom_id) = value.strip_prefix(CUSTOM_PET_PREFIX) {
            return load_custom_pet(custom_id, codex_home);
        }

        if let Some(builtin) = catalog::builtin_pet(value) {
            return load_builtin_pet(builtin, codex_home);
        }

        load_custom_pet(value, codex_home)
    }

    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    pub(super) fn frame_cache_key(&self) -> Result<String> {
        let bytes = fs::read(&self.spritesheet_path)
            .with_context(|| format!("read {}", self.spritesheet_path.display()))?;
        let digest = Sha256::digest(&bytes);
        Ok(format!(
            "sha256-{digest:x}-{}x{}-{}x{}",
            self.frame_width, self.frame_height, self.columns, self.rows
        ))
    }
}

pub(super) const CUSTOM_PET_PREFIX: &str = "custom:";

/// 清单文件的反序列化形状；缺失字段在后续规范化阶段应用产品默认值。
#[derive(Debug, Deserialize)]
struct PetFile {
    #[serde(default)]
    id: Option<String>,
    #[serde(default, rename = "displayName")]
    display_name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, rename = "spritesheetPath")]
    spritesheet_path: Option<String>,
    frame: Option<FrameSpec>,
    #[serde(default)]
    animations: HashMap<String, AnimationSpec>,
}

/// 清单声明的单帧尺寸及网格列数。
#[derive(Debug, Clone, Copy, Deserialize)]
struct FrameSpec {
    width: u32,
    height: u32,
    columns: u32,
    rows: u32,
}

impl Default for FrameSpec {
    fn default() -> Self {
        Self {
            width: catalog::DEFAULT_FRAME_WIDTH,
            height: catalog::DEFAULT_FRAME_HEIGHT,
            columns: catalog::DEFAULT_FRAME_COLUMNS,
            rows: catalog::DEFAULT_FRAME_ROWS,
        }
    }
}

pub(super) fn custom_pet_selector(id: &str) -> String {
    format!("{CUSTOM_PET_PREFIX}{id}")
}

/// 清单动画的帧索引、帧率、循环起点及回退轨道声明。
#[derive(Debug, Deserialize)]
struct AnimationSpec {
    #[serde(default)]
    frames: Vec<usize>,
    fps: Option<f64>,
    #[serde(rename = "loop")]
    loop_animation: Option<bool>,
    #[serde(default)]
    fallback: String,
}

/// 从版本化内置资源目录构造宠物，并应用 App 默认动画。
fn load_builtin_pet(pet: catalog::BuiltinPet, codex_home: Option<&Path>) -> Result<Pet> {
    let codex_home = codex_home.context("CODEX_HOME is not available")?;
    let spritesheet_path = super::builtin_spritesheet_path(codex_home, pet.spritesheet_file);
    if !spritesheet_path.exists() {
        bail!("missing spritesheet {}", spritesheet_path.display());
    }

    Ok(Pet {
        id: pet.id.to_string(),
        display_name: pet.display_name.to_string(),
        description: pet.description.to_string(),
        spritesheet_path,
        frame_width: catalog::DEFAULT_FRAME_WIDTH,
        frame_height: catalog::DEFAULT_FRAME_HEIGHT,
        columns: catalog::DEFAULT_FRAME_COLUMNS,
        rows: catalog::DEFAULT_FRAME_ROWS,
        frame_count: default_frame_count(),
        animations: default_animations(),
    })
}

/// 按新版 pets 目录优先、旧版 avatars 目录回退的顺序加载自定义宠物。
fn load_custom_pet(value: &str, codex_home: Option<&Path>) -> Result<Pet> {
    let codex_home = codex_home.context("CODEX_HOME is not available")?;
    let pet_dir = codex_home.join("pets").join(value);
    if pet_dir.join("pet.json").is_file() {
        return load_pet_manifest(&pet_dir, "pet.json", value, &custom_pet_cache_id(value));
    }

    let avatar_dir = codex_home.join("avatars").join(value);
    if avatar_dir.join("avatar.json").is_file() {
        return load_pet_manifest(
            &avatar_dir,
            "avatar.json",
            value,
            &custom_pet_cache_id(value),
        );
    }

    bail!("unknown pet {value}")
}

/// 将显式目录或 `pet.json` 路径归一到宠物目录后加载清单。
fn load_pet_path(value: &str) -> Result<Pet> {
    let path = expand_path(value)?;
    let metadata = fs::metadata(&path).with_context(|| format!("pet path {}", path.display()))?;
    let dir = if metadata.is_dir() {
        path
    } else {
        path.parent()
            .context("pet json path has no containing directory")?
            .to_path_buf()
    };
    let pet_dir = dir
        .canonicalize()
        .with_context(|| format!("resolve {}", dir.display()))?;
    let manifest_file = if pet_dir.join("pet.json").is_file() {
        "pet.json"
    } else if pet_dir.join("avatar.json").is_file() {
        "avatar.json"
    } else {
        bail!("missing pet.json or avatar.json in {}", pet_dir.display());
    };
    let fallback_id = pet_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("pet");
    load_pet_manifest(&pet_dir, manifest_file, fallback_id, fallback_id)
}

/// 读取清单、校验精灵表与帧网格，并生成稳定帧缓存键。
fn load_pet_manifest(
    pet_dir: &Path,
    manifest_file: &str,
    fallback_id: &str,
    cache_id: &str,
) -> Result<Pet> {
    let config_path = pet_dir.join(manifest_file);
    let raw = fs::read_to_string(&config_path)
        .with_context(|| format!("read {}", config_path.display()))?;
    let file: PetFile =
        serde_json::from_str(&raw).with_context(|| format!("parse {}", config_path.display()))?;

    let manifest_id = file
        .id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty());
    let display_name = file
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .or(manifest_id)
        .unwrap_or(fallback_id)
        .to_string();
    let pet_id = if cache_id == fallback_id {
        manifest_id.unwrap_or(fallback_id).to_string()
    } else {
        cache_id.to_string()
    };
    let description = file
        .description
        .map(|description| description.trim().to_string())
        .unwrap_or_default();
    let spritesheet_path = resolve_spritesheet_path(
        pet_dir,
        file.spritesheet_path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .unwrap_or("spritesheet.webp"),
    )?;
    if !spritesheet_path.exists() {
        bail!("missing spritesheet {}", spritesheet_path.display());
    }
    let (spritesheet_width, spritesheet_height) =
        validate_app_spritesheet_dimensions(&spritesheet_path)?;

    let frame = file.frame.unwrap_or_default();
    let frame_count = validate_frame_spec(&frame, spritesheet_width, spritesheet_height)?;
    Ok(Pet {
        id: pet_id,
        display_name,
        description,
        spritesheet_path,
        frame_width: frame.width,
        frame_height: frame.height,
        columns: frame.columns,
        rows: frame.rows,
        frame_count,
        animations: load_animations(file.animations, frame_count)?,
    })
}

/// Resolve a manifest-relative spritesheet path while keeping it inside the pet directory.
///
/// The manifest format intentionally supports only relative child paths.
/// Allowing absolute or parent-traversing paths here would let one custom pet
/// manifest reach outside its own directory and make cache/debug behavior
/// depend on unrelated local files.
/// 解析相对清单的精灵表路径并把结果限制在宠物目录内。
/// 清单只允许相对子路径；绝对路径或父目录穿越会越出用户宠物边界，因此明确拒绝。
fn resolve_spritesheet_path(pet_dir: &Path, spritesheet_path: &str) -> Result<PathBuf> {
    let path = Path::new(spritesheet_path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        bail!("spritesheet path must stay inside {}", pet_dir.display());
    }
    Ok(pet_dir.join(path))
}

/// 校验精灵表满足 Codex App 兼容的整体尺寸。
fn validate_app_spritesheet_dimensions(path: &Path) -> Result<(u32, u32)> {
    let (width, height) =
        image::image_dimensions(path).with_context(|| format!("read {}", path.display()))?;
    if width != catalog::SPRITESHEET_WIDTH || height != catalog::SPRITESHEET_HEIGHT {
        bail!(
            "spritesheet must be {}x{} pixels",
            catalog::SPRITESHEET_WIDTH,
            catalog::SPRITESHEET_HEIGHT
        );
    }
    Ok((width, height))
}

/// 校验单帧尺寸、列数及计算出的帧总数均有效且受上限约束。
fn validate_frame_spec(
    frame: &FrameSpec,
    spritesheet_width: u32,
    spritesheet_height: u32,
) -> Result<usize> {
    if frame.width == 0 || frame.height == 0 || frame.columns == 0 || frame.rows == 0 {
        bail!("pet frame dimensions and grid counts must be non-zero");
    }

    let total_width = frame
        .width
        .checked_mul(frame.columns)
        .context("pet frame grid width overflow")?;
    let total_height = frame
        .height
        .checked_mul(frame.rows)
        .context("pet frame grid height overflow")?;
    if total_width != spritesheet_width || total_height != spritesheet_height {
        bail!(
            "pet frame grid must cover spritesheet exactly: expected {spritesheet_width}x{spritesheet_height}, got {total_width}x{total_height}"
        );
    }

    let frame_count = frame
        .columns
        .checked_mul(frame.rows)
        .context("pet frame count overflow")?;
    let frame_count = usize::try_from(frame_count).context("pet frame count does not fit usize")?;
    if frame_count > MAX_PET_FRAMES {
        bail!("pet frame count {frame_count} exceeds maximum {MAX_PET_FRAMES}");
    }
    Ok(frame_count)
}

fn custom_pet_cache_id(id: &str) -> String {
    format!("custom-{id}")
}

fn path_like(value: &str) -> bool {
    value == "."
        || value == ".."
        || value.starts_with("~/")
        || value.starts_with("../")
        || value.starts_with("./")
        || Path::new(value).is_absolute()
        || value.contains('/')
        || value.contains('\\')
}

/// 路径选择器只展开当前用户主目录前缀，不做环境变量或 shell 扩展。
fn expand_path(value: &str) -> Result<PathBuf> {
    if value == "~" || value.starts_with("~/") {
        let home = std::env::var_os("HOME").context("HOME is not set")?;
        if value == "~" {
            return Ok(PathBuf::from(home));
        }
        return Ok(PathBuf::from(home).join(&value[2..]));
    }

    Ok(PathBuf::from(value))
}

/// 把清单动画规范化为逐帧持续时间，并验证帧率与循环形状。
fn load_animations(
    specs: HashMap<String, AnimationSpec>,
    frame_count: usize,
) -> Result<HashMap<String, Animation>> {
    let mut animations = default_animations();
    if specs.is_empty() {
        validate_animation_indices(&animations, frame_count)?;
        return Ok(animations);
    }

    for (name, spec) in specs {
        if spec.frames.is_empty() {
            bail!("animation {name} must include at least one frame");
        }
        for sprite_index in &spec.frames {
            if *sprite_index >= frame_count {
                bail!(
                    "animation {name} references sprite index {sprite_index}, but pet has {frame_count} frames"
                );
            }
        }

        let fps = match spec.fps {
            Some(fps) if fps.is_finite() && fps > 0.0 && fps <= MAX_ANIMATION_FPS => fps,
            Some(fps) => {
                bail!(
                    "animation {name} fps must be finite and between 0 and {MAX_ANIMATION_FPS}, got {fps}"
                );
            }
            None => 8.0,
        };
        let duration = Duration::from_secs_f64(1.0 / fps);
        let fallback = if spec.fallback.is_empty() {
            "idle".to_string()
        } else {
            spec.fallback
        };
        let loop_start = spec
            .loop_animation
            .unwrap_or(/*default*/ true)
            .then_some(/*loop_start*/ 0);

        animations.insert(
            name,
            Animation {
                frames: spec
                    .frames
                    .into_iter()
                    .map(|sprite_index| AnimationFrame {
                        sprite_index,
                        duration,
                    })
                    .collect(),
                loop_start,
                fallback,
            },
        );
    }

    animations
        .entry("idle".to_string())
        .or_insert_with(idle_animation);
    validate_animation_indices(&animations, frame_count)?;
    Ok(animations)
}

/// 确保所有动画索引都落在精灵网格内，且回退名称指向现有动画。
fn validate_animation_indices(
    animations: &HashMap<String, Animation>,
    frame_count: usize,
) -> Result<()> {
    for (name, animation) in animations {
        if animation.frames.is_empty() {
            bail!("animation {name} must include at least one frame");
        }
        for frame in &animation.frames {
            if frame.sprite_index >= frame_count {
                bail!(
                    "animation {name} references sprite index {}, but pet has {frame_count} frames",
                    frame.sprite_index
                );
            }
        }
        if !animations.contains_key(&animation.fallback) {
            bail!(
                "animation {name} fallback {} does not exist",
                animation.fallback
            );
        }
    }
    Ok(())
}

fn default_frame_count() -> usize {
    (catalog::DEFAULT_FRAME_COLUMNS * catalog::DEFAULT_FRAME_ROWS) as usize
}

/// 构造与 Codex App 各语义状态行相匹配的默认动画集合。
fn default_animations() -> HashMap<String, Animation> {
    [
        ("idle", idle_animation()),
        (
            "running-right",
            app_state_animation(
                /*row_index*/ 1, /*frame_count*/ 8, /*frame_duration_ms*/ 120,
                /*final_frame_duration_ms*/ 220,
            ),
        ),
        (
            "running-left",
            app_state_animation(
                /*row_index*/ 2, /*frame_count*/ 8, /*frame_duration_ms*/ 120,
                /*final_frame_duration_ms*/ 220,
            ),
        ),
        (
            "waving",
            app_state_animation(
                /*row_index*/ 3, /*frame_count*/ 4, /*frame_duration_ms*/ 140,
                /*final_frame_duration_ms*/ 280,
            ),
        ),
        (
            "jumping",
            app_state_animation(
                /*row_index*/ 4, /*frame_count*/ 5, /*frame_duration_ms*/ 140,
                /*final_frame_duration_ms*/ 280,
            ),
        ),
        (
            "failed",
            app_state_animation(
                /*row_index*/ 5, /*frame_count*/ 8, /*frame_duration_ms*/ 140,
                /*final_frame_duration_ms*/ 240,
            ),
        ),
        (
            "waiting",
            app_state_animation(
                /*row_index*/ 6, /*frame_count*/ 6, /*frame_duration_ms*/ 150,
                /*final_frame_duration_ms*/ 260,
            ),
        ),
        (
            "running",
            app_state_animation(
                /*row_index*/ 7, /*frame_count*/ 6, /*frame_duration_ms*/ 120,
                /*final_frame_duration_ms*/ 220,
            ),
        ),
        (
            "review",
            app_state_animation(
                /*row_index*/ 8, /*frame_count*/ 6, /*frame_duration_ms*/ 150,
                /*final_frame_duration_ms*/ 280,
            ),
        ),
        (
            "move_right",
            app_state_animation(
                /*row_index*/ 1, /*frame_count*/ 8, /*frame_duration_ms*/ 120,
                /*final_frame_duration_ms*/ 220,
            ),
        ),
        (
            "move_left",
            app_state_animation(
                /*row_index*/ 2, /*frame_count*/ 8, /*frame_duration_ms*/ 120,
                /*final_frame_duration_ms*/ 220,
            ),
        ),
        (
            "wave",
            app_state_animation(
                /*row_index*/ 3, /*frame_count*/ 4, /*frame_duration_ms*/ 140,
                /*final_frame_duration_ms*/ 280,
            ),
        ),
        (
            "bounce",
            app_state_animation(
                /*row_index*/ 4, /*frame_count*/ 5, /*frame_duration_ms*/ 140,
                /*final_frame_duration_ms*/ 280,
            ),
        ),
        (
            "sad",
            app_state_animation(
                /*row_index*/ 5, /*frame_count*/ 8, /*frame_duration_ms*/ 140,
                /*final_frame_duration_ms*/ 240,
            ),
        ),
    ]
    .into_iter()
    .map(|(name, animation)| (name.to_string(), animation))
    .collect()
}

fn idle_animation() -> Animation {
    Animation {
        frames: [(0, 1680), (1, 660), (2, 660), (3, 840), (4, 840), (5, 1920)]
            .into_iter()
            .map(|(sprite_index, duration_ms)| AnimationFrame {
                sprite_index,
                duration: Duration::from_millis(duration_ms),
            })
            .collect(),
        loop_start: Some(/*loop_start*/ 0),
        fallback: "idle".to_string(),
    }
}

fn app_state_animation(
    row_index: usize,
    frame_count: usize,
    frame_duration_ms: u64,
    final_frame_duration_ms: u64,
) -> Animation {
    let primary_frames = (0..frame_count)
        .map(|column_index| AnimationFrame {
            sprite_index: row_index * catalog::DEFAULT_FRAME_COLUMNS as usize + column_index,
            duration: Duration::from_millis(if column_index == frame_count - 1 {
                final_frame_duration_ms
            } else {
                frame_duration_ms
            }),
        })
        .collect::<Vec<_>>();
    let primary_frame_count = primary_frames.len() * 3;
    let frames = primary_frames
        .iter()
        .chain(primary_frames.iter())
        .chain(primary_frames.iter())
        .cloned()
        .chain(idle_animation().frames)
        .collect();
    Animation {
        frames,
        loop_start: Some(primary_frame_count),
        fallback: "idle".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 写入最小有效清单与标准精灵表，供成功路径测试复用。
    fn write_minimal_pet() -> tempfile::TempDir {
        write_pet_manifest(
            r#"{
                "id": "chefito",
                "displayName": "Chefito",
                "description": "A tiny recipe-loving chef",
                "spritesheetPath": "spritesheet.webp"
            }"#,
        )
    }

    fn write_pet_manifest(manifest: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("pet.json"), manifest).unwrap();
        catalog::write_test_spritesheet(&dir.path().join("spritesheet.webp"));
        dir
    }

    fn load_pet_from_dir(dir: &tempfile::TempDir) -> Pet {
        Pet::load_with_codex_home(dir.path().to_str().unwrap(), /*codex_home*/ None).unwrap()
    }

    fn load_pet_error_from_dir(dir: &tempfile::TempDir) -> anyhow::Error {
        Pet::load_with_codex_home(dir.path().to_str().unwrap(), /*codex_home*/ None).unwrap_err()
    }

    #[test]
    // 验证内置宠物从应用目录约定位置读取资源与默认元数据。
    fn load_builtin_pet_uses_app_catalog_storage() {
        let codex_home = tempfile::tempdir().unwrap();
        super::super::asset_pack::write_test_pack(codex_home.path());

        let pet =
            Pet::load_with_codex_home("dewey", /*codex_home*/ Some(codex_home.path())).unwrap();

        assert_eq!(pet.id, "dewey");
        assert_eq!(pet.display_name, "Dewey");
        assert_eq!(pet.description, "A tidy duck for calm workspace days");
        assert_eq!(
            pet.spritesheet_path,
            super::super::builtin_spritesheet_path(codex_home.path(), "dewey-spritesheet-v4.webp")
        );
        assert_eq!(pet.frame_width, 192);
        assert_eq!(pet.frame_height, 208);
        assert_eq!(pet.columns, 8);
        assert_eq!(pet.rows, 9);
    }

    #[test]
    // 验证默认 idle 动画只循环平静区段。
    fn app_idle_animation_uses_calm_loop() {
        let animations = default_animations();
        let idle = &animations["idle"];

        assert_eq!(sprite_indices(idle), vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(durations_ms(idle), vec![1680, 660, 660, 840, 840, 1920]);
        assert_eq!(idle.loop_start, Some(/*loop_start*/ 0));
    }

    #[test]
    // 验证 running 动画重复工作帧后按声明回退到 idle。
    fn app_running_animation_repeats_then_settles_into_idle() {
        let animations = default_animations();
        let running = &animations["running"];
        let primary = vec![56, 57, 58, 59, 60, 61];

        assert_eq!(sprite_indices(running)[0..6], primary);
        assert_eq!(sprite_indices(running)[6..12], primary);
        assert_eq!(sprite_indices(running)[12..18], primary);
        assert_eq!(
            sprite_indices(running)[18..],
            sprite_indices(&animations["idle"])
        );
        assert_eq!(
            durations_ms(running)[0..6],
            vec![120, 120, 120, 120, 120, 220]
        );
        assert_eq!(running.loop_start, Some(/*loop_start*/ 18));
    }

    #[test]
    // 验证通知语义状态使用产品精灵表约定的对应行。
    fn app_notification_states_use_expected_rows() {
        let animations = default_animations();

        assert_eq!(
            sprite_indices(&animations["waiting"])[0..6],
            vec![48, 49, 50, 51, 52, 53]
        );
        assert_eq!(
            sprite_indices(&animations["review"])[0..6],
            vec![64, 65, 66, 67, 68, 69]
        );
        assert_eq!(
            sprite_indices(&animations["failed"])[0..8],
            vec![40, 41, 42, 43, 44, 45, 46, 47]
        );
    }

    #[test]
    // 验证自定义动画保留清单帧率、循环起点与回退形状。
    fn custom_animation_specs_keep_manifest_fps_and_loop_shape() {
        let animations = load_animations(
            HashMap::from([(
                "custom".to_string(),
                AnimationSpec {
                    frames: vec![1, 2],
                    fps: Some(/*fps*/ 2.0),
                    loop_animation: Some(/*loop_animation*/ false),
                    fallback: "idle".to_string(),
                },
            )]),
            default_frame_count(),
        )
        .unwrap();
        let custom = &animations["custom"];

        assert_eq!(sprite_indices(custom), vec![1, 2]);
        assert_eq!(durations_ms(custom), vec![500, 500]);
        assert_eq!(custom.loop_start, None);
        assert_eq!(custom.fallback, "idle");
    }

    #[test]
    // 验证目录选择器使用应用清单默认值补全缺失字段。
    fn load_pet_directory_uses_app_pet_manifest_defaults() {
        let dir = write_minimal_pet();

        let pet = load_pet_from_dir(&dir);

        assert_eq!(pet.id, "chefito");
        assert_eq!(pet.display_name, "Chefito");
        assert_eq!(pet.frame_width, 192);
        assert_eq!(pet.frame_height, 208);
        assert_eq!(pet.columns, 8);
        assert_eq!(pet.rows, 9);
        assert_eq!(pet.frame_count(), 72);
        assert!(!pet.animations["idle"].frames.is_empty());
    }

    #[test]
    // 验证精灵表内容变化会使帧缓存键变化。
    fn frame_cache_key_changes_with_spritesheet_contents() {
        let dir = write_minimal_pet();
        let spritesheet_path = dir.path().join("spritesheet.webp");
        let pet = load_pet_from_dir(&dir);
        let first_key = pet.frame_cache_key().unwrap();

        let image = image::RgbaImage::from_pixel(
            catalog::SPRITESHEET_WIDTH,
            catalog::SPRITESHEET_HEIGHT,
            image::Rgba([1, 2, 3, 255]),
        );
        image.save(&spritesheet_path).unwrap();
        let pet = load_pet_from_dir(&dir);

        assert_ne!(pet.frame_cache_key().unwrap(), first_key);
    }

    #[test]
    // 验证帧几何变化会使帧缓存键变化。
    fn frame_cache_key_changes_with_frame_spec() {
        let default_dir = write_minimal_pet();
        let default_pet = load_pet_from_dir(&default_dir);
        let custom_dir = write_pet_manifest(
            r#"{
                "displayName": "Tall",
                "spritesheetPath": "spritesheet.webp",
                "frame": { "width": 384, "height": 104, "columns": 4, "rows": 18 }
            }"#,
        );
        let custom_pet = load_pet_from_dir(&custom_dir);

        assert_ne!(
            custom_pet.frame_cache_key().unwrap(),
            default_pet.frame_cache_key().unwrap()
        );
    }

    #[test]
    // 验证显式 `pet.json` 路径使用其所在目录解析相对资源。
    fn load_pet_json_path_uses_containing_directory() {
        let dir = write_minimal_pet();

        let pet = Pet::load_with_codex_home(
            dir.path().join("pet.json").to_str().unwrap(),
            /*codex_home*/ None,
        )
        .unwrap();
        let expected = dir.path().join("spritesheet.webp").canonicalize().unwrap();

        assert_eq!(pet.spritesheet_path, expected);
    }

    #[test]
    // 验证自定义 selector 从 CODEX_HOME 的 pets 目录加载清单。
    fn custom_pet_selector_loads_codex_home_pet_manifest() {
        let dir = write_minimal_pet();
        let codex_home = tempfile::tempdir().unwrap();
        let pet_dir = codex_home.path().join("pets").join("chefito");
        fs::create_dir_all(&pet_dir).unwrap();
        fs::copy(dir.path().join("pet.json"), pet_dir.join("pet.json")).unwrap();
        fs::copy(
            dir.path().join("spritesheet.webp"),
            pet_dir.join("spritesheet.webp"),
        )
        .unwrap();

        let pet = Pet::load_with_codex_home(
            &custom_pet_selector("chefito"),
            /*codex_home*/ Some(codex_home.path()),
        )
        .unwrap();

        assert_eq!(pet.id, "custom-chefito");
        assert_eq!(pet.spritesheet_path, pet_dir.join("spritesheet.webp"),);
    }

    #[test]
    // 验证自定义 selector 可回退读取旧版 avatars 目录。
    fn custom_pet_selector_falls_back_to_legacy_avatar_manifest() {
        let dir = write_minimal_pet();
        let codex_home = tempfile::tempdir().unwrap();
        let avatar_dir = codex_home.path().join("avatars").join("legacy");
        fs::create_dir_all(&avatar_dir).unwrap();
        fs::copy(dir.path().join("pet.json"), avatar_dir.join("avatar.json")).unwrap();
        fs::copy(
            dir.path().join("spritesheet.webp"),
            avatar_dir.join("spritesheet.webp"),
        )
        .unwrap();

        let pet = Pet::load_with_codex_home(
            &custom_pet_selector("legacy"),
            /*codex_home*/ Some(codex_home.path()),
        )
        .unwrap();

        assert_eq!(pet.id, "custom-legacy");
        assert_eq!(pet.display_name, "Chefito");
    }

    #[test]
    // 验证精灵表相对路径不能逃逸自定义宠物目录。
    fn custom_pet_rejects_spritesheet_path_escape() {
        let codex_home = tempfile::tempdir().unwrap();
        let pet_dir = codex_home.path().join("pets").join("escape");
        fs::create_dir_all(&pet_dir).unwrap();
        fs::write(
            pet_dir.join("pet.json"),
            r#"{
                "displayName": "Escape",
                "spritesheetPath": "../spritesheet.webp"
            }"#,
        )
        .unwrap();

        let err = Pet::load_with_codex_home(
            &custom_pet_selector("escape"),
            /*codex_home*/ Some(codex_home.path()),
        )
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("spritesheet path must stay inside")
        );
    }

    #[test]
    // 验证零尺寸帧在切图前被拒绝。
    fn custom_pet_rejects_zero_frame_dimensions() {
        let dir = write_pet_manifest(
            r#"{
                "displayName": "Zero",
                "spritesheetPath": "spritesheet.webp",
                "frame": { "width": 0, "height": 208, "columns": 8, "rows": 9 }
            }"#,
        );

        let err = load_pet_error_from_dir(&dir);

        assert!(
            err.to_string()
                .contains("pet frame dimensions and grid counts must be non-zero")
        );
    }

    #[test]
    // 验证帧网格必须完整覆盖精灵表尺寸。
    fn custom_pet_rejects_frame_grid_that_does_not_cover_spritesheet() {
        let dir = write_pet_manifest(
            r#"{
                "displayName": "Short",
                "spritesheetPath": "spritesheet.webp",
                "frame": { "width": 192, "height": 208, "columns": 7, "rows": 9 }
            }"#,
        );

        let err = load_pet_error_from_dir(&dir);

        assert!(
            err.to_string()
                .contains("pet frame grid must cover spritesheet exactly")
        );
    }

    #[test]
    // 验证计算出的帧数超过安全上限时拒绝清单。
    fn custom_pet_rejects_excessive_frame_count() {
        let dir = write_pet_manifest(
            r#"{
                "displayName": "Dense",
                "spritesheetPath": "spritesheet.webp",
                "frame": { "width": 8, "height": 8, "columns": 192, "rows": 234 }
            }"#,
        );

        let err = load_pet_error_from_dir(&dir);

        assert!(err.to_string().contains("exceeds maximum"));
    }

    #[test]
    // 验证动画不允许声明空帧序列。
    fn custom_pet_rejects_empty_animation_frames() {
        let dir = write_pet_manifest(
            r#"{
                "displayName": "Empty",
                "spritesheetPath": "spritesheet.webp",
                "animations": {
                    "idle": { "frames": [] }
                }
            }"#,
        );

        let err = load_pet_error_from_dir(&dir);

        assert!(
            err.to_string()
                .contains("animation idle must include at least one frame")
        );
    }

    #[test]
    // 验证动画帧索引不能越出精灵网格。
    fn custom_pet_rejects_animation_frame_outside_grid() {
        let dir = write_pet_manifest(
            r#"{
                "displayName": "Outside",
                "spritesheetPath": "spritesheet.webp",
                "animations": {
                    "idle": { "frames": [72] }
                }
            }"#,
        );

        let err = load_pet_error_from_dir(&dir);

        assert!(
            err.to_string()
                .contains("animation idle references sprite index 72")
        );
    }

    #[test]
    // 验证非有限、非正数或过高的动画帧率均被拒绝。
    fn custom_pet_rejects_invalid_animation_fps() {
        let dir = write_pet_manifest(
            r#"{
                "displayName": "Fast",
                "spritesheetPath": "spritesheet.webp",
                "animations": {
                    "idle": { "frames": [0], "fps": 120.0 }
                }
            }"#,
        );

        let err = load_pet_error_from_dir(&dir);

        assert!(
            err.to_string()
                .contains("animation idle fps must be finite and between")
        );
    }

    #[test]
    // 验证一次性动画的回退目标必须存在。
    fn custom_pet_rejects_animation_fallback_to_missing_animation() {
        let dir = write_pet_manifest(
            r#"{
                "displayName": "Fallback",
                "spritesheetPath": "spritesheet.webp",
                "animations": {
                    "wave": { "frames": [1], "loop": false, "fallback": "missing" }
                }
            }"#,
        );

        let err = load_pet_error_from_dir(&dir);

        assert!(
            err.to_string()
                .contains("animation wave fallback missing does not exist")
        );
    }

    fn sprite_indices(animation: &Animation) -> Vec<usize> {
        animation
            .frames
            .iter()
            .map(|frame| frame.sprite_index)
            .collect()
    }

    fn durations_ms(animation: &Animation) -> Vec<u128> {
        animation
            .frames
            .iter()
            .map(|frame| frame.duration.as_millis())
            .collect()
    }
}
