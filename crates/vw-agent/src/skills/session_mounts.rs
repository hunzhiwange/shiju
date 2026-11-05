//! Private session directory leases. Sources stay live: validate before handing a view
//! to an executor and retain the lease until that executor has stopped using it.
use anyhow::{Result, bail, ensure};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};
use vw_acp::SessionSkillMount;
use vw_config_types::skills::SkillSecurityConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSkillBinding {
    pub tenant: String,
    pub project: PathBuf,
    pub session: String,
    pub revision: u64,
}

struct Source {
    mount: SessionSkillMount,
    canonical: PathBuf,
    identity: (u64, u64),
    digest: String,
}

/// Dropping a lease removes only its private directory and links, never the sources.
/// Replacements are prepared separately; callers swap leases only after success.
pub struct SessionSkillView {
    _directory: tempfile::TempDir,
    root: PathBuf,
    binding: SessionSkillBinding,
    sources: Vec<Source>,
    fingerprint: String,
}

impl SessionSkillView {
    pub fn prepare(
        mut binding: SessionSkillBinding,
        mounts: &[SessionSkillMount],
        security: &SkillSecurityConfig,
    ) -> Result<Self> {
        ensure!(
            !binding.tenant.trim().is_empty() && !binding.session.trim().is_empty(),
            "missing session skill binding"
        );
        binding.project = binding.project.canonicalize()?;
        ensure!(binding.project.is_dir(), "project must be a directory");
        let directory = tempfile::Builder::new().prefix("shiju-session-skills-").tempdir()?;
        let stage = directory.path().join("staging");
        fs::create_dir(&stage)?;
        let mut names = HashSet::new();
        let mut sources = Vec::new();
        for mount in mounts {
            mount.validate()?;
            ensure!(names.insert(mount.name.to_lowercase()), "duplicate skill mount name");
            let original = Path::new(&mount.path);
            let canonical = original.canonicalize()?;
            let identity = directory_identity(original)?;
            let before = tree_digest(&canonical)?;
            ensure!(canonical.join("SKILL.md").is_file(), "skill mount requires a native SKILL.md");
            ensure!(
                super::loader::load_skill_directory(
                    &canonical,
                    super::SkillLoadMode::Full,
                    security,
                    true
                )?
                .is_some(),
                "disabled or invalid skill mount"
            );
            let source = Source { mount: mount.clone(), canonical, identity, digest: before };
            source.validate()?;
            link_directory(&source.canonical, &stage.join(&mount.name))?;
            source.validate()?;
            sources.push(source);
        }
        sources.sort_by(|a, b| a.mount.name.cmp(&b.mount.name));
        // Length-prefixed structured encoding keeps collection and binding boundaries unambiguous.
        let entries: Vec<_> = sources
            .iter()
            .map(|source| (&source.mount.name, &source.canonical, source.identity, &source.digest))
            .collect();
        let fingerprint = hex::encode(Sha256::digest(serde_json::to_vec(&(
            &binding.tenant,
            &binding.project,
            &binding.session,
            binding.revision,
            entries,
        ))?));
        for source in &sources {
            source.validate()?;
        }
        fs::rename(&stage, directory.path().join("skills"))?;
        let root = directory.path().join("skills");
        Ok(Self { _directory: directory, root, binding, sources, fingerprint })
    }

    /// Validate live sources immediately before passing this path to an executor.
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn binding(&self) -> &SessionSkillBinding {
        &self.binding
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn validate(&self) -> Result<()> {
        directory_identity(self.root())?;
        for source in &self.sources {
            source.validate()?;
            let link = self.root().join(&source.mount.name);
            ensure!(
                fs::symlink_metadata(&link)?.file_type().is_symlink(),
                "skill view link replaced"
            );
            ensure!(fs::read_link(link)? == source.canonical, "skill view link changed");
        }
        ensure!(
            fs::read_dir(self.root())?.count() == self.sources.len(),
            "skill view collection changed"
        );
        Ok(())
    }
}

impl Source {
    fn validate(&self) -> Result<()> {
        let original = Path::new(&self.mount.path);
        ensure!(
            directory_identity(original)? == self.identity
                && original.canonicalize()? == self.canonical,
            "skill source identity changed"
        );
        ensure!(!super::is_local_skill_disabled(&self.canonical), "skill source disabled");
        ensure!(tree_digest(&self.canonical)? == self.digest, "skill source content changed");
        Ok(())
    }
}

#[cfg(unix)]
fn directory_identity(path: &Path) -> Result<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "skill source must be a real directory"
    );
    Ok((metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn directory_identity(_path: &Path) -> Result<(u64, u64)> {
    bail!("safe session skill directory links are unsupported on this platform")
}

#[cfg(unix)]
fn link_directory(source: &Path, destination: &Path) -> Result<()> {
    std::os::unix::fs::symlink(source, destination)?;
    Ok(())
}
#[cfg(not(unix))]
fn link_directory(_source: &Path, _destination: &Path) -> Result<()> {
    bail!("safe session skill directory links are unsupported on this platform")
}

fn tree_digest(root: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    hash_tree(root, root, &mut hasher)?;
    Ok(hex::encode(hasher.finalize()))
}

fn hash_tree(root: &Path, path: &Path, hasher: &mut Sha256) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    // Conservatively reject even in-tree links: linked sources must not acquire
    // unaudited files through a later symlink replacement.
    ensure!(
        !metadata.file_type().is_symlink(),
        "symbolic links inside skill sources are unsupported"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        hasher.update(metadata.mode().to_le_bytes());
    }
    let relative = path.strip_prefix(root)?;
    let encoded = serde_json::to_vec(relative)?;
    hasher.update((encoded.len() as u64).to_le_bytes());
    hasher.update(encoded);
    if metadata.is_dir() {
        hasher.update(b"directory");
        let mut children = fs::read_dir(path)?.collect::<std::io::Result<Vec<_>>>()?;
        children.sort_by_key(|entry| entry.file_name());
        hasher.update((children.len() as u64).to_le_bytes());
        for child in children {
            hash_tree(root, &child.path(), hasher)?;
        }
    } else if metadata.is_file() {
        hasher.update(b"file");
        hasher.update(metadata.len().to_le_bytes());
        let mut options = fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW);
        }
        let mut file = options.open(path)?;
        let mut buffer = [0; 8192];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
    } else {
        bail!("unsupported special file in skill source");
    }
    Ok(())
}
