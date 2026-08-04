//! 内置预设主题：`include_dir` 嵌入与「目录缺失才种子」写入用户 themes_root。

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use include_dir::{Dir, include_dir};

static BUNDLED_PRESETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../assets/themes/presets");

/// ensure 结果：本次新写入与因已存在而跳过的预设 id。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnsurePresetsReport {
    /// 本次从嵌入资源写出的预设 id。
    pub seeded: Vec<String>,
    /// 因 `theme.json` 已存在而跳过的预设 id。
    pub skipped: Vec<String>,
}

/// 嵌入预设目录名列表（按 id 排序）。
pub fn list_bundled_preset_ids() -> Vec<String> {
    let mut ids: Vec<String> = BUNDLED_PRESETS.dirs().filter_map(|dir| dir_id(dir)).collect();
    ids.sort();
    ids
}

/// 是否为仓库内置嵌入预设 id。
pub fn is_bundled_preset_id(id: &str) -> bool {
    let id = id.trim();
    if id.is_empty() {
        return false;
    }
    BUNDLED_PRESETS.get_dir(id).is_some()
}

/// 将缺失的内置预设写入 `themes_root`；已有 `theme.json` 的目录跳过，不覆盖用户编辑。
pub fn ensure_bundled_presets(themes_root: impl AsRef<Path>) -> Result<EnsurePresetsReport> {
    let themes_root = themes_root.as_ref();
    fs::create_dir_all(themes_root)
        .with_context(|| format!("failed to create themes root {}", themes_root.display()))?;

    let mut report = EnsurePresetsReport::default();
    let mut dirs: Vec<&Dir<'_>> = BUNDLED_PRESETS.dirs().collect();
    dirs.sort_by_key(|dir| dir.path());

    for dir in dirs {
        let Some(id) = dir_id(dir) else {
            continue;
        };
        let theme_json = themes_root.join(&id).join("theme.json");
        if theme_json.is_file() {
            report.skipped.push(id);
            continue;
        }
        seed_embedded_dir(dir, themes_root).with_context(|| {
            format!("failed to seed bundled preset `{id}` into {}", themes_root.display())
        })?;
        if !theme_json.is_file() {
            bail!(
                "bundled preset `{id}` is missing theme.json after seed at {}",
                theme_json.display()
            );
        }
        report.seeded.push(id);
    }

    Ok(report)
}

/// 检查本地预设的嵌入文件是否有缺失或字节差异。
pub fn bundled_preset_is_modified(themes_root: impl AsRef<Path>, id: &str) -> Result<bool> {
    let id = id.trim();
    let Some(embedded) = BUNDLED_PRESETS.get_dir(id) else {
        bail!("unknown bundled preset id `{id}`");
    };

    embedded_dir_is_modified(embedded, themes_root.as_ref())
}

/// 删除本地预设目录并从嵌入资源重新写出默认内容。
pub fn restore_bundled_preset(themes_root: impl AsRef<Path>, id: &str) -> Result<()> {
    let themes_root = themes_root.as_ref();
    let id = id.trim();
    let Some(embedded) = BUNDLED_PRESETS.get_dir(id) else {
        bail!("unknown bundled preset id `{id}`");
    };

    let preset_dir = themes_root.join(id);
    match fs::remove_dir_all(&preset_dir) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            return Err(err).with_context(|| {
                format!("failed to remove local bundled preset directory {}", preset_dir.display())
            });
        }
    }

    seed_embedded_dir(embedded, themes_root).with_context(|| {
        format!("failed to restore bundled preset `{id}` into {}", themes_root.display())
    })?;

    let theme_json = preset_dir.join("theme.json");
    if !theme_json.is_file() {
        bail!(
            "bundled preset `{id}` is missing theme.json after restore at {}",
            theme_json.display()
        );
    }

    Ok(())
}

fn dir_id(dir: &Dir<'_>) -> Option<String> {
    dir.path()
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .filter(|id| !id.is_empty())
}

fn embedded_dir_is_modified(embedded: &Dir<'_>, themes_root: &Path) -> Result<bool> {
    for file in embedded.files() {
        let local_path = join_embedded_path(themes_root, file.path())?;
        let local_contents = match fs::read(&local_path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(true),
            Err(err) => {
                return Err(err).with_context(|| {
                    format!("failed to read local theme asset {}", local_path.display())
                });
            }
        };
        if local_contents != file.contents() {
            return Ok(true);
        }
    }
    for child in embedded.dirs() {
        if embedded_dir_is_modified(child, themes_root)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn seed_embedded_dir(embedded: &Dir<'_>, themes_root: &Path) -> Result<()> {
    for file in embedded.files() {
        let dest = join_embedded_path(themes_root, file.path())?;
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create theme asset directory {}", parent.display())
            })?;
        }
        fs::write(&dest, file.contents())
            .with_context(|| format!("failed to write theme asset {}", dest.display()))?;
    }
    for child in embedded.dirs() {
        seed_embedded_dir(child, themes_root)?;
    }
    Ok(())
}

fn join_embedded_path(themes_root: &Path, relative: &Path) -> Result<PathBuf> {
    if relative.is_absolute() {
        bail!("embedded theme path must be relative: {}", relative.display());
    }
    for component in relative.components() {
        use std::path::Component;
        match component {
            Component::Normal(_) => {}
            Component::CurDir => {}
            _ => bail!("embedded theme path has unsafe component: {}", relative.display()),
        }
    }
    Ok(themes_root.join(relative))
}
