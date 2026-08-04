//! `.vwtheme`（ZIP）主题包导出、校验与本地安装。

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::assets::{supported_image_extension, validate_relative_asset_name};
use super::{ThemeDefinition, ThemeStore};

const THEME_JSON: &str = "theme.json";
const MAX_THEME_PACK_BYTES: u64 = 32 * 1024 * 1024;

/// 将主题目录导出为扁平 `.vwtheme` ZIP，仅包含定义与定义引用的现存图片。
pub fn export_theme_dir_to_pack(
    theme_dir: impl AsRef<Path>,
    pack_path: impl AsRef<Path>,
) -> Result<()> {
    let theme_dir = theme_dir.as_ref();
    let pack_path = pack_path.as_ref();
    if !theme_dir.is_dir() {
        bail!("theme directory does not exist: {}", theme_dir.display());
    }

    let theme_json_path = theme_dir.join(THEME_JSON);
    let theme_json = fs::read(&theme_json_path)
        .with_context(|| format!("failed to read theme file {}", theme_json_path.display()))?;
    let definition = parse_and_validate_definition(&theme_json)?;
    let directory_id = theme_dir.file_name().and_then(|name| name.to_str());
    if directory_id != Some(definition.id.as_str()) {
        bail!(
            "theme id mismatch: directory `{}` but theme.json has `{}`",
            directory_id.unwrap_or("(invalid)"),
            definition.id
        );
    }

    let canonical_json =
        serde_json::to_vec_pretty(&definition).context("failed to serialize theme definition")?;
    write_theme_pack(&definition, &canonical_json, Some(theme_dir), pack_path)
}

/// 将不含图片资源的主题定义直接导出为 `.vwtheme`。
pub fn export_theme_definition_to_pack(
    definition: &ThemeDefinition,
    pack_path: impl AsRef<Path>,
) -> Result<()> {
    let mut definition = definition.clone();
    definition.normalize();
    definition.validate()?;
    if !definition.assets.is_empty() {
        bail!("theme definition with assets must be exported from its theme directory");
    }
    let theme_json =
        serde_json::to_vec_pretty(&definition).context("failed to serialize theme definition")?;
    write_theme_pack(&definition, &theme_json, None, pack_path)
}

fn write_theme_pack(
    definition: &ThemeDefinition,
    theme_json: &[u8],
    theme_dir: Option<&Path>,
    pack_path: impl AsRef<Path>,
) -> Result<()> {
    let pack_path = pack_path.as_ref();
    let asset_names = validated_asset_names(definition)?;
    let mut total_size =
        u64::try_from(theme_json.len()).context("theme definition is too large to package")?;
    for name in &asset_names {
        let theme_dir = theme_dir.context("theme assets require a source directory")?;
        let path = theme_dir.join(name);
        let metadata = fs::metadata(&path)
            .with_context(|| format!("failed to read theme asset {}", path.display()))?;
        if !metadata.is_file() {
            bail!("theme asset is not a file: {}", path.display());
        }
        total_size = total_size.checked_add(metadata.len()).context("theme pack size overflow")?;
    }
    ensure_size_within_limit(total_size)?;

    let cursor = Cursor::new(Vec::new());
    let mut archive = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    archive.start_file(THEME_JSON, options).context("failed to add theme.json to theme pack")?;
    archive.write_all(&theme_json).context("failed to write theme.json to theme pack")?;
    for name in asset_names {
        let theme_dir = theme_dir.context("theme assets require a source directory")?;
        let path = theme_dir.join(&name);
        archive
            .start_file(&name, options)
            .with_context(|| format!("failed to add theme asset `{name}` to theme pack"))?;
        let mut source = fs::File::open(&path)
            .with_context(|| format!("failed to open theme asset {}", path.display()))?;
        std::io::copy(&mut source, &mut archive)
            .with_context(|| format!("failed to write theme asset `{name}` to theme pack"))?;
    }
    let pack_bytes = archive.finish().context("failed to finish theme pack")?.into_inner();
    fs::write(pack_path, pack_bytes)
        .with_context(|| format!("failed to write theme pack {}", pack_path.display()))?;
    Ok(())
}

/// 校验 `.vwtheme` ZIP，并以不覆盖已有主题的方式安装到 `ThemeStore`。
pub fn import_theme_pack_into_store(
    pack_path: impl AsRef<Path>,
    store: &ThemeStore,
) -> Result<ThemeDefinition> {
    let pack_path = pack_path.as_ref();
    let file = fs::File::open(pack_path)
        .with_context(|| format!("failed to open theme pack {}", pack_path.display()))?;
    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("failed to read theme pack {}", pack_path.display()))?;
    let entries = inspect_archive(&mut archive)?;
    let theme_index = *entries.get(THEME_JSON).context("theme pack is missing theme.json")?;
    let theme_json = read_archive_entry(&mut archive, theme_index, THEME_JSON)?;
    let mut definition = parse_and_validate_definition(&theme_json)?;
    let asset_names = validated_asset_names(&definition)?;

    let mut assets = Vec::with_capacity(asset_names.len());
    for name in asset_names {
        let index = *entries
            .get(&name)
            .with_context(|| format!("theme pack is missing declared asset `{name}`"))?;
        let bytes = read_archive_entry(&mut archive, index, &name)?;
        assets.push((name, bytes));
    }

    let (installed_id, theme_dir) = create_unique_theme_dir(store, &definition.id)?;
    definition.id = installed_id;
    let install_result = install_validated_theme(&theme_dir, &definition, &assets)
        .and_then(|()| store.load(&definition.id));
    match install_result {
        Ok(installed) => Ok(installed),
        Err(err) => {
            let _ = fs::remove_dir_all(&theme_dir);
            Err(err)
        }
    }
}

fn inspect_archive<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
) -> Result<BTreeMap<String, usize>> {
    let mut entries = BTreeMap::new();
    let mut total_size = 0u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .with_context(|| format!("failed to inspect theme pack entry {index}"))?;
        let name = entry.name().to_string();
        validate_archive_entry_name(&name)?;
        if entries.insert(name.clone(), index).is_some() {
            bail!("theme pack contains duplicate entry `{name}`");
        }
        total_size = total_size.checked_add(entry.size()).context("theme pack size overflow")?;
        ensure_size_within_limit(total_size)?;
    }
    Ok(entries)
}

fn validate_archive_entry_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.starts_with('/')
        || name.starts_with('\\')
        || name.contains("..")
        || name.contains('/')
        || name.contains('\\')
    {
        bail!("invalid theme pack entry path `{name}`: expected a single relative filename");
    }
    Ok(())
}

fn read_archive_entry<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    index: usize,
    name: &str,
) -> Result<Vec<u8>> {
    let entry = archive
        .by_index(index)
        .with_context(|| format!("failed to open theme pack entry `{name}`"))?;
    let mut bytes = Vec::new();
    entry
        .take(MAX_THEME_PACK_BYTES + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("failed to read theme pack entry `{name}`"))?;
    ensure_size_within_limit(u64::try_from(bytes.len()).context("theme pack entry is too large")?)?;
    Ok(bytes)
}

fn parse_and_validate_definition(bytes: &[u8]) -> Result<ThemeDefinition> {
    let mut definition: ThemeDefinition =
        serde_json::from_slice(bytes).context("failed to parse theme.json")?;
    definition.normalize();
    definition.validate()?;
    Ok(definition)
}

fn validated_asset_names(definition: &ThemeDefinition) -> Result<BTreeSet<String>> {
    let names = [
        definition.assets.cover.as_deref(),
        definition.assets.background.as_deref(),
        definition.assets.chat_header.as_deref(),
        definition.assets.chat_body.as_deref(),
        definition.assets.chat_input.as_deref(),
    ];
    let mut validated = BTreeSet::new();
    for name in names.into_iter().flatten() {
        validate_relative_asset_name(name)?;
        if supported_image_extension(Path::new(name)).is_none() {
            bail!(
                "unsupported image extension for theme asset `{name}` (allowed: png, jpg, jpeg, webp)"
            );
        }
        validated.insert(name.to_string());
    }
    Ok(validated)
}

fn create_unique_theme_dir(store: &ThemeStore, base_id: &str) -> Result<(String, PathBuf)> {
    let themes_root = store
        .theme_asset_path(base_id, THEME_JSON)
        .parent()
        .and_then(Path::parent)
        .context("failed to resolve themes root")?
        .to_path_buf();
    fs::create_dir_all(&themes_root)
        .with_context(|| format!("failed to create themes root {}", themes_root.display()))?;

    for suffix in 1..=10_000u32 {
        let candidate = theme_id_candidate(base_id, suffix);
        let theme_dir = themes_root.join(&candidate);
        match fs::create_dir(&theme_dir) {
            Ok(()) => return Ok((candidate, theme_dir)),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(err) => {
                return Err(err).with_context(|| {
                    format!("failed to create theme directory {}", theme_dir.display())
                });
            }
        }
    }
    bail!("failed to allocate a unique id for imported theme `{base_id}`")
}

fn theme_id_candidate(base_id: &str, suffix: u32) -> String {
    if suffix == 1 {
        return base_id.to_string();
    }
    let suffix = format!("-{suffix}");
    let base_len = 64usize.saturating_sub(suffix.len());
    format!("{}{}", &base_id[..base_id.len().min(base_len)], suffix)
}

fn install_validated_theme(
    theme_dir: &Path,
    definition: &ThemeDefinition,
    assets: &[(String, Vec<u8>)],
) -> Result<()> {
    for (name, bytes) in assets {
        let path = theme_dir.join(name);
        fs::write(&path, bytes)
            .with_context(|| format!("failed to write theme asset {}", path.display()))?;
    }
    let theme_json =
        serde_json::to_vec_pretty(definition).context("failed to serialize theme definition")?;
    let theme_json_path = theme_dir.join(THEME_JSON);
    fs::write(&theme_json_path, theme_json)
        .with_context(|| format!("failed to write theme file {}", theme_json_path.display()))?;
    Ok(())
}

fn ensure_size_within_limit(size: u64) -> Result<()> {
    if size > MAX_THEME_PACK_BYTES {
        bail!(
            "theme pack exceeds the {} MiB uncompressed size limit",
            MAX_THEME_PACK_BYTES / 1024 / 1024
        );
    }
    Ok(())
}
