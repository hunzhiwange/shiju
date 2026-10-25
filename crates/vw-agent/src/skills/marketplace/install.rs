//! 市场制品的统一校验与原子安装流程。
//!
//! 在落盘前验证签名、摘要、压缩包边界和技能清单，并以替换目录方式提交安装。

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ring::signature::{ED25519, UnparsedPublicKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tempfile::Builder;
use vw_config_types::skills::SkillSecurityConfig;
use vw_gateway_client::SkillMarketCoordinateDto;
use zip::ZipArchive;

use super::provider::{
    SkillMarketArtifact, SkillMarketArtifactProof, SkillMarketSkillSetArtifact,
    SkillMarketSkillSetVerificationPolicy,
};
use crate::app::agent::skills::audit;

const MAX_ARCHIVE_BYTES: usize = 32 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 512;
const MAX_EXTRACTED_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RELATIVE_PATH_BYTES: usize = 240;
const SKILL_MANIFEST: &str = "SKILL.md";
const META_FILE: &str = "_meta.json";

/// 验证、审计并原子安装一个不可变市场制品。
pub(crate) fn install_verified_market_artifact(
    skills_root: &Path,
    coordinate: &SkillMarketCoordinateDto,
    artifact: SkillMarketArtifact,
    security: &SkillSecurityConfig,
) -> Result<PathBuf> {
    install_verified_market_artifact_ref(skills_root, coordinate, &artifact, security)
}

/// 将已验证制品安装到每个不同根目录；后续目标失败时回滚本次已完成目标。
pub(crate) fn install_verified_market_artifact_for_roots(
    skills_roots: &[PathBuf],
    coordinate: &SkillMarketCoordinateDto,
    artifact: SkillMarketArtifact,
    security: &SkillSecurityConfig,
) -> Result<Vec<PathBuf>> {
    let mut seen = BTreeSet::new();
    let roots = skills_roots.iter().filter(|root| seen.insert((*root).clone())).collect::<Vec<_>>();
    if roots.is_empty() {
        bail!("skill market install requires at least one target");
    }

    let mut installed = Vec::with_capacity(roots.len());
    for root in roots {
        match install_verified_market_artifact_ref(root, coordinate, &artifact, security) {
            Ok(path) => installed.push(path),
            Err(error) => {
                let mut rollback_failures = Vec::new();
                for path in installed.iter().rev() {
                    if let Err(rollback_error) = fs::remove_dir_all(path) {
                        rollback_failures.push(format!("{}: {rollback_error}", path.display()));
                    }
                }
                if rollback_failures.is_empty() {
                    return Err(error);
                }
                bail!(
                    "{error:#}; failed to roll back market installs: {}",
                    rollback_failures.join(", ")
                );
            }
        }
    }
    Ok(installed)
}

fn install_verified_market_artifact_ref(
    skills_root: &Path,
    coordinate: &SkillMarketCoordinateDto,
    artifact: &SkillMarketArtifact,
    security: &SkillSecurityConfig,
) -> Result<PathBuf> {
    let target_name = target_directory_name(coordinate)?;
    if artifact.archive.len() > MAX_ARCHIVE_BYTES {
        bail!("skill market artifact exceeds the 32 MiB download limit");
    }
    verify_proof(coordinate, &artifact.proof)?;

    let target = skills_root.join(target_name);
    ensure_target_absent(&target)?;
    ensure_skills_root(skills_root)?;

    let staging =
        Builder::new().prefix(".skill-install-").tempdir_in(skills_root).with_context(|| {
            format!("failed to create skill market staging directory in {}", skills_root.display())
        })?;
    let package = staging.path().join("package");
    fs::create_dir(&package).context("failed to create skill market package staging directory")?;

    let content_hash = extract_verified_archive(&artifact.archive, &package)?;
    if content_hash != artifact.proof.content_hash {
        bail!("skill market artifact content hash does not match its signed proof");
    }

    let report = audit::audit_skill_directory_with_security(&package, security)?;
    if !report.is_clean() {
        bail!("Skill security audit failed: {}", report.summary());
    }

    write_safe_metadata(&package, coordinate, &content_hash, "skill", None)?;
    ensure_target_absent(&target)?;
    fs::rename(&package, &target).with_context(|| {
        format!("failed to atomically install skill market artifact at {}", target.display())
    })?;
    Ok(target)
}

/// 验证并暂存完整技能集，所有成员通过后才一次性提交。
pub(crate) fn install_verified_market_skill_set(
    skills_root: &Path,
    artifact: SkillMarketSkillSetArtifact,
    security: &SkillSecurityConfig,
) -> Result<Vec<PathBuf>> {
    validate_source(&artifact.source)?;
    validate_coordinate_component("skill-set slug", &artifact.slug)?;
    match &artifact.verification {
        SkillMarketSkillSetVerificationPolicy::TransportAndAudit => {}
        SkillMarketSkillSetVerificationPolicy::Unsupported(policy) => {
            bail!("unsupported skill-set verification policy: {policy}");
        }
    }
    if artifact.archive.len() > MAX_ARCHIVE_BYTES {
        bail!("skill market skill-set artifact exceeds the 32 MiB download limit");
    }
    ensure_skills_root(skills_root)?;

    let mut member_slugs = BTreeSet::new();
    let mut target_names = BTreeSet::new();
    let mut targets = Vec::with_capacity(artifact.members.len() + 1);
    let meta_coordinate = SkillMarketCoordinateDto {
        locator: vw_gateway_client::SkillMarketSkillLocatorDto {
            source: artifact.source.clone(),
            namespace: "skillset".to_string(),
            slug: artifact.slug.clone(),
        },
        version: String::new(),
    };
    let meta_target_name = format!(
        "{}--{}--{}",
        meta_coordinate.locator.source,
        meta_coordinate.locator.namespace,
        meta_coordinate.locator.slug
    );
    target_names.insert(meta_target_name.clone());
    targets.push(skills_root.join(&meta_target_name));
    for member in &artifact.members {
        if member.coordinate.locator.source != artifact.source {
            bail!("skill-set member does not belong to the requested source");
        }
        let target_name = target_directory_name(&member.coordinate)?;
        if !member_slugs.insert(member.coordinate.locator.slug.clone()) {
            bail!("skill-set contains a duplicate member slug");
        }
        if !target_names.insert(target_name.clone()) {
            bail!("skill-set contains a duplicate member coordinate");
        }
        targets.push(skills_root.join(target_name));
    }
    for target in &targets {
        ensure_target_absent(target)?;
    }

    let staging =
        Builder::new().prefix(".skill-install-").tempdir_in(skills_root).with_context(|| {
            format!("failed to create skill-set staging directory in {}", skills_root.display())
        })?;
    let suite_archive = staging.path().join("suite");
    fs::create_dir(&suite_archive).context("failed to create skill-set archive staging")?;
    let suite_files = extract_safe_archive(&artifact.archive, &suite_archive)?;
    let manifest =
        validate_skill_set_manifest(&suite_archive, &suite_files, &artifact.slug, &member_slugs)?;

    let identify_path = suite_archive.join("identify.md");
    let identify =
        fs::read_to_string(&identify_path).context("skill-set identify.md is not valid UTF-8")?;
    let version = validate_meta_skill(&identify, &artifact.slug, &member_slugs)?;
    let meta_coordinate = SkillMarketCoordinateDto { version, ..meta_coordinate };
    validate_market_coordinate(&meta_coordinate)?;

    let packages = staging.path().join("packages");
    fs::create_dir(&packages).context("failed to create skill-set package staging")?;
    let meta_package = packages.join("meta");
    fs::create_dir(&meta_package).context("failed to create meta-skill staging")?;
    fs::copy(&identify_path, meta_package.join(SKILL_MANIFEST))
        .context("failed to stage skill-set meta-skill")?;
    let identify_hash = manifest
        .files
        .iter()
        .find(|file| file.path == "identify.md")
        .map(|file| file.sha256.as_str())
        .context("skill-set manifest does not declare identify.md")?;
    audit_package(&meta_package, security)?;
    write_safe_metadata(
        &meta_package,
        &meta_coordinate,
        identify_hash,
        "meta-skill",
        Some(&artifact.slug),
    )?;

    let mut staged_packages = vec![meta_package];
    for (index, member) in artifact.members.iter().enumerate() {
        let package = packages.join(format!("member-{index}"));
        fs::create_dir(&package).context("failed to create member staging directory")?;
        prepare_signed_package(
            &package,
            &member.coordinate,
            &member.artifact,
            Some(&artifact.slug),
            security,
        )?;
        staged_packages.push(package);
    }

    for target in &targets {
        ensure_target_absent(target)?;
    }
    commit_packages_atomically(&staged_packages, &targets)?;
    Ok(targets)
}

/// 校验市场坐标的来源、标识、版本与内容摘要格式。
pub(crate) fn validate_market_coordinate(coordinate: &SkillMarketCoordinateDto) -> Result<()> {
    target_directory_name(coordinate).map(|_| ())
}

fn target_directory_name(coordinate: &SkillMarketCoordinateDto) -> Result<String> {
    validate_source(&coordinate.locator.source)?;
    validate_coordinate_component("namespace", &coordinate.locator.namespace)?;
    validate_coordinate_component("slug", &coordinate.locator.slug)?;
    if coordinate.version.trim().is_empty() {
        bail!("skill market coordinate version is required");
    }

    let name = format!(
        "{}--{}--{}",
        coordinate.locator.source, coordinate.locator.namespace, coordinate.locator.slug
    );
    if name.len() > MAX_RELATIVE_PATH_BYTES {
        bail!("skill market install directory exceeds the path limit");
    }
    Ok(name)
}

fn validate_source(source: &str) -> Result<()> {
    if source.is_empty()
        || !source
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || !source.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        || !source.as_bytes().last().is_some_and(u8::is_ascii_alphanumeric)
    {
        bail!("skill market coordinate source is invalid");
    }
    Ok(())
}

fn validate_coordinate_component(name: &str, value: &str) -> Result<()> {
    if value.is_empty() || value.trim() != value {
        bail!("skill market coordinate {name} is invalid");
    }
    if matches!(value, "." | "..")
        || value
            .chars()
            .any(|character| character == '/' || character == '\\' || character.is_control())
    {
        bail!("skill market coordinate {name} is unsafe");
    }
    Ok(())
}

fn verify_proof(
    coordinate: &SkillMarketCoordinateDto,
    proof: &SkillMarketArtifactProof,
) -> Result<()> {
    if !proof.signed {
        bail!("skill market artifact is unsigned");
    }
    if proof.key_id.trim().is_empty() {
        bail!("skill market artifact signing key is missing");
    }
    validate_content_hash(&proof.content_hash)?;

    let public_key = STANDARD
        .decode(&proof.public_key_b64)
        .context("skill market artifact public key is not valid base64")?;
    let signature = STANDARD
        .decode(&proof.signature_b64)
        .context("skill market artifact signature is not valid base64")?;
    UnparsedPublicKey::new(&ED25519, public_key)
        .verify(proof.payload.as_bytes(), &signature)
        .map_err(|_| anyhow::anyhow!("skill market artifact signature verification failed"))?;

    let payload: SignedPayload = serde_json::from_str(&proof.payload)
        .context("skill market artifact signed payload is invalid")?;
    if payload.content_hash != proof.content_hash {
        bail!("skill market artifact proof contains inconsistent content hashes");
    }
    if payload.skill_slug != coordinate.locator.slug || payload.skill_version != coordinate.version
    {
        bail!("skill market artifact proof does not match the requested coordinate");
    }
    Ok(())
}

fn validate_content_hash(content_hash: &str) -> Result<()> {
    if content_hash.len() != 64
        || !content_hash.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("skill market artifact content hash is invalid");
    }
    Ok(())
}

#[derive(Deserialize)]
struct SignedPayload {
    content_hash: String,
    skill_slug: String,
    skill_version: String,
}

fn ensure_target_absent(target: &Path) -> Result<()> {
    match fs::symlink_metadata(target) {
        Ok(_) => bail!("skill market install target already exists: {}", target.display()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .with_context(|| format!("failed to inspect skill market target {}", target.display())),
    }
}

fn ensure_skills_root(skills_root: &Path) -> Result<()> {
    if let Some(parent) = skills_root.parent()
        && let Ok(metadata) = fs::symlink_metadata(parent)
        && metadata.file_type().is_symlink()
    {
        bail!("skill market install parent cannot be a symbolic link");
    }
    fs::create_dir_all(skills_root)
        .with_context(|| format!("failed to create skills directory {}", skills_root.display()))?;
    let metadata = fs::symlink_metadata(skills_root)
        .with_context(|| format!("failed to inspect skills directory {}", skills_root.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        bail!("skill market install root must be a real directory");
    }
    Ok(())
}

fn extract_verified_archive(archive_bytes: &[u8], destination: &Path) -> Result<String> {
    let file_hashes = extract_safe_archive(archive_bytes, destination)?;
    if !file_hashes.contains_key(SKILL_MANIFEST) {
        bail!("skill market artifact must contain SKILL.md at its root");
    }
    Ok(aggregate_content_hash(file_hashes))
}

fn extract_safe_archive(
    archive_bytes: &[u8],
    destination: &Path,
) -> Result<BTreeMap<String, String>> {
    let mut archive = ZipArchive::new(Cursor::new(archive_bytes))
        .context("skill market artifact is not a ZIP")?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        bail!("skill market artifact exceeds the 512 entry limit");
    }

    let mut paths = BTreeSet::new();
    let mut file_hashes = BTreeMap::new();
    let mut declared_total = 0_u64;
    let mut extracted_total = 0_u64;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .with_context(|| format!("failed to read skill market ZIP entry {index}"))?;
        let raw_name = std::str::from_utf8(entry.name_raw())
            .context("skill market ZIP path is not valid UTF-8")?
            .to_string();
        let relative_path = validate_archive_path(&raw_name)?;
        if !paths.insert(relative_path.clone()) {
            bail!("skill market ZIP contains a duplicate path: {raw_name}");
        }

        declared_total = declared_total
            .checked_add(entry.size())
            .context("skill market extracted size overflowed")?;
        if declared_total > MAX_EXTRACTED_BYTES {
            bail!("skill market artifact exceeds the 64 MiB extracted limit");
        }

        let mode = entry.unix_mode().unwrap_or(0);
        let file_type = mode & 0o170000;
        if entry.is_dir() {
            if !matches!(file_type, 0 | 0o040000) {
                bail!("skill market ZIP directory has an unsafe file type: {raw_name}");
            }
            if !ignored_archive_path(&raw_name) {
                fs::create_dir_all(destination.join(&relative_path)).with_context(|| {
                    format!("failed to create skill market directory {raw_name}")
                })?;
            }
            continue;
        }
        if !matches!(file_type, 0 | 0o100000) {
            bail!("skill market ZIP contains a symbolic link or device: {raw_name}");
        }

        let ignored = ignored_archive_path(&raw_name);
        let output_path = (!ignored).then(|| destination.join(&relative_path));
        let file_hash = read_entry(&mut entry, output_path.as_deref(), &mut extracted_total)?;
        if extracted_total > MAX_EXTRACTED_BYTES {
            bail!("skill market artifact exceeds the 64 MiB extracted limit");
        }
        if !ignored {
            file_hashes.insert(raw_name, file_hash);
        }
    }

    Ok(file_hashes)
}

fn validate_archive_path(raw_name: &str) -> Result<PathBuf> {
    if raw_name.is_empty()
        || raw_name.as_bytes().len() > MAX_RELATIVE_PATH_BYTES
        || raw_name.contains('\\')
        || raw_name.contains('\0')
    {
        bail!("skill market ZIP contains an unsafe path");
    }
    let path = Path::new(raw_name);
    if path.is_absolute()
        || path.components().any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("skill market ZIP contains an unsafe path: {raw_name}");
    }
    Ok(path.to_path_buf())
}

fn ignored_archive_path(path: &str) -> bool {
    path.split('/').any(|component| {
        component == META_FILE
            || component == "__MACOSX"
            || component == ".DS_Store"
            || component == "Thumbs.db"
            || component.starts_with("._")
    })
}

fn read_entry<R: Read>(
    entry: &mut R,
    output_path: Option<&Path>,
    extracted_total: &mut u64,
) -> Result<String> {
    let mut output = match output_path {
        Some(path) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create skill market directory {}", parent.display())
                })?;
            }
            Some(OpenOptions::new().write(true).create_new(true).open(path).with_context(|| {
                format!("failed to create skill market file {}", path.display())
            })?)
        }
        None => None,
    };
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = entry.read(&mut buffer).context("failed to extract skill market ZIP entry")?;
        if read == 0 {
            break;
        }
        let read_u64 = u64::try_from(read).context("skill market extracted size overflowed")?;
        *extracted_total = extracted_total
            .checked_add(read_u64)
            .context("skill market extracted size overflowed")?;
        if *extracted_total > MAX_EXTRACTED_BYTES {
            bail!("skill market artifact exceeds the 64 MiB extracted limit");
        }
        hasher.update(&buffer[..read]);
        if let Some(file) = output.as_mut() {
            file.write_all(&buffer[..read]).context("failed to write skill market ZIP entry")?;
        }
    }
    if let Some(file) = output.as_mut() {
        file.flush().context("failed to flush skill market ZIP entry")?;
    }
    Ok(hex::encode(hasher.finalize()))
}

fn aggregate_content_hash(file_hashes: BTreeMap<String, String>) -> String {
    let mut aggregate = Sha256::new();
    for (path, hash) in file_hashes {
        aggregate.update(path.as_bytes());
        aggregate.update(b":");
        aggregate.update(hash.as_bytes());
        aggregate.update(b"\n");
    }
    hex::encode(aggregate.finalize())
}

fn write_safe_metadata(
    package: &Path,
    coordinate: &SkillMarketCoordinateDto,
    content_hash: &str,
    package_type: &str,
    source_pack: Option<&str>,
) -> Result<()> {
    let mut metadata = serde_json::json!({
        "source": coordinate.locator.source,
        "namespace": coordinate.locator.namespace,
        "slug": coordinate.locator.slug,
        "version": coordinate.version,
        "content_hash": content_hash,
        "package_type": package_type,
    });
    if let Some(source_pack) = source_pack {
        metadata["source_pack"] = serde_json::Value::String(source_pack.to_string());
    }
    let bytes = serde_json::to_vec_pretty(&metadata)
        .context("failed to serialize skill market metadata")?;
    let path = package.join(META_FILE);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .with_context(|| format!("failed to create safe metadata {}", path.display()))?;
    file.write_all(&bytes)
        .with_context(|| format!("failed to write safe metadata {}", path.display()))?;
    file.write_all(b"\n")
        .with_context(|| format!("failed to finish safe metadata {}", path.display()))?;
    file.flush().with_context(|| format!("failed to flush safe metadata {}", path.display()))?;
    Ok(())
}

fn prepare_signed_package(
    package: &Path,
    coordinate: &SkillMarketCoordinateDto,
    artifact: &SkillMarketArtifact,
    source_pack: Option<&str>,
    security: &SkillSecurityConfig,
) -> Result<()> {
    if artifact.archive.len() > MAX_ARCHIVE_BYTES {
        bail!("skill market artifact exceeds the 32 MiB download limit");
    }
    verify_proof(coordinate, &artifact.proof)?;
    let content_hash = extract_verified_archive(&artifact.archive, package)?;
    if content_hash != artifact.proof.content_hash {
        bail!("skill market artifact content hash does not match its signed proof");
    }
    audit_package(package, security)?;
    write_safe_metadata(package, coordinate, &content_hash, "skill", source_pack)
}

fn audit_package(package: &Path, security: &SkillSecurityConfig) -> Result<()> {
    let report = audit::audit_skill_directory_with_security(package, security)?;
    if !report.is_clean() {
        bail!("Skill security audit failed: {}", report.summary());
    }
    Ok(())
}

fn commit_packages_atomically(packages: &[PathBuf], targets: &[PathBuf]) -> Result<()> {
    let mut committed = Vec::new();
    for (package, target) in packages.iter().zip(targets) {
        if let Err(error) = fs::rename(package, target) {
            for (committed_target, original_package) in committed.iter().rev() {
                let _ = fs::rename(committed_target, original_package);
            }
            return Err(error).with_context(|| {
                format!("failed to atomically install skill-set package at {}", target.display())
            });
        }
        committed.push((target, package));
    }
    Ok(())
}

#[derive(Deserialize)]
struct SkillSetManifest {
    #[serde(rename = "manifestVersion")]
    manifest_version: String,
    #[serde(rename = "type")]
    package_type: String,
    slug: String,
    #[serde(rename = "skillSlugs")]
    skill_slugs: Vec<String>,
    skills: Vec<SkillSetManifestMember>,
    files: Vec<SkillSetManifestFile>,
}

#[derive(Deserialize)]
struct SkillSetManifestMember {
    slug: String,
}

#[derive(Deserialize)]
struct SkillSetManifestFile {
    path: String,
    sha256: String,
    size: u64,
}

fn validate_skill_set_manifest(
    suite_archive: &Path,
    file_hashes: &BTreeMap<String, String>,
    expected_slug: &str,
    expected_members: &BTreeSet<String>,
) -> Result<SkillSetManifest> {
    let manifest_bytes = fs::read(suite_archive.join("manifest.json"))
        .context("skill-set artifact must contain manifest.json")?;
    let manifest: SkillSetManifest =
        serde_json::from_slice(&manifest_bytes).context("skill-set manifest.json is invalid")?;
    if manifest.manifest_version != "1"
        || manifest.package_type != "skillset"
        || manifest.slug != expected_slug
    {
        bail!("skill-set manifest does not match the requested suite");
    }

    let declared_slugs = unique_values(&manifest.skill_slugs, "skillSlugs")?;
    let listed_slugs = unique_values(
        &manifest.skills.iter().map(|member| member.slug.clone()).collect::<Vec<_>>(),
        "skills",
    )?;
    if &declared_slugs != expected_members || listed_slugs != declared_slugs {
        bail!("skill-set manifest members do not match the detail endpoint");
    }

    let mut declared_files = BTreeSet::new();
    for file in &manifest.files {
        validate_archive_path(&file.path)?;
        validate_content_hash(&file.sha256)?;
        if !declared_files.insert(file.path.clone()) {
            bail!("skill-set manifest contains a duplicate file");
        }
        let actual_hash =
            file_hashes.get(&file.path).context("skill-set manifest declares a missing file")?;
        if actual_hash != &file.sha256 {
            bail!("skill-set file hash does not match its manifest");
        }
        let actual_size = fs::metadata(suite_archive.join(&file.path))
            .context("failed to inspect skill-set file")?
            .len();
        if actual_size != file.size {
            bail!("skill-set file size does not match its manifest");
        }
    }
    let actual_files = file_hashes
        .keys()
        .filter(|path| path.as_str() != "manifest.json")
        .cloned()
        .collect::<BTreeSet<_>>();
    if actual_files != declared_files || !declared_files.contains("identify.md") {
        bail!("skill-set archive files do not match its manifest");
    }
    Ok(manifest)
}

fn unique_values(values: &[String], field: &str) -> Result<BTreeSet<String>> {
    let mut unique = BTreeSet::new();
    for value in values {
        validate_coordinate_component(field, value)?;
        if !unique.insert(value.clone()) {
            bail!("skill-set manifest contains duplicate {field}");
        }
    }
    Ok(unique)
}

#[derive(Deserialize)]
struct MetaSkillFrontmatter {
    name: String,
    version: String,
    metadata: MetaSkillMetadata,
    orchestration: MetaSkillOrchestration,
}

#[derive(Deserialize)]
struct MetaSkillMetadata {
    package_type: String,
}

#[derive(Deserialize)]
struct MetaSkillOrchestration {
    children: Vec<String>,
}

fn validate_meta_skill(
    content: &str,
    expected_name: &str,
    expected_children: &BTreeSet<String>,
) -> Result<String> {
    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some("---") {
        bail!("skill-set identify.md must contain YAML frontmatter");
    }
    let mut yaml = String::new();
    for line in lines {
        if line.trim() == "---" {
            let frontmatter: MetaSkillFrontmatter = serde_yaml::from_str(&yaml)
                .context("unsupported skill-set meta-skill frontmatter")?;
            if frontmatter.metadata.package_type != "meta-skill" {
                bail!("skill-set frontmatter package_type must be meta-skill");
            }
            if frontmatter.name != expected_name {
                bail!("skill-set frontmatter name does not match suite slug");
            }
            if frontmatter.version.trim().is_empty() {
                bail!("skill-set frontmatter version is required");
            }
            let children = unique_values(&frontmatter.orchestration.children, "children")?;
            if &children != expected_children {
                bail!("skill-set frontmatter children do not match suite members");
            }
            return Ok(frontmatter.version);
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    bail!("skill-set frontmatter is unterminated")
}
