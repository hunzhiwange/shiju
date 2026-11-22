//! Verified original Junie Linux distribution for native accounts and models.
use super::{Stage, root};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Read},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    time::Duration,
};

const MANIFEST: &str =
    include_str!("../../../../../patches/acp-model-source/junie/linux-artifacts.json");
const INSTALLER: &str =
    include_str!("../../../../../patches/acp-model-source/junie/install_linux_native.py");

#[derive(Deserialize)]
struct Manifest {
    native_revision: String,
    entry: String,
    args: Vec<String>,
    targets: BTreeMap<String, Target>,
}

#[derive(Deserialize)]
struct Target {
    tree_sha256: String,
}

impl Manifest {
    fn load() -> Result<Self, String> {
        serde_json::from_str(MANIFEST).map_err(|_| "Invalid Junie Linux artifact manifest".into())
    }
}

pub(super) fn discover(root: &Path) -> Result<Option<(String, Vec<String>)>, String> {
    let manifest = Manifest::load()?;
    let root = match root.canonicalize() {
        Ok(root) => root,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("Cannot resolve Junie Linux runtime root".into()),
    };
    let directory = root.join(&manifest.native_revision);
    match directory.symlink_metadata() {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("Cannot inspect Junie Linux runtime".into()),
        Ok(_) => {}
    }
    validate_runtime_parent(&root)?;
    verify(&directory, &manifest)?;
    let command = directory.join(&manifest.entry);
    let command = command.to_str().ok_or("Junie Linux entry path is not Unicode")?;
    Ok(Some((command.into(), manifest.args)))
}

pub(super) fn verify_launch(root: &Path, command: &Path, args: &[String]) -> Result<(), String> {
    let manifest = Manifest::load()?;
    let relative = Path::new(&manifest.native_revision).join(&manifest.entry);
    let canonical_root = root.canonicalize().ok();
    let canonical_command = command.canonicalize().ok();
    if command != root.join(&relative)
        && !canonical_root.as_ref().is_some_and(|root| {
            let fixed = root.join(&relative);
            command == fixed || canonical_command.as_ref() == Some(&fixed)
        })
    {
        return Ok(());
    }
    if args != manifest.args.as_slice() {
        return Err("Unexpected Junie Linux ACP arguments".into());
    }
    discover(root)?.ok_or("Fixed Junie Linux executable is missing")?;
    Ok(())
}

pub(super) async fn install() -> Result<(), String> {
    let root = root("junie")?;
    if discover(&root)?.is_some() {
        return Ok(());
    }
    fs::create_dir_all(&root).map_err(|_| "Cannot create Junie Linux runtime root")?;
    let root = root.canonicalize().map_err(|_| "Cannot resolve Junie Linux runtime root")?;
    validate_runtime_parent(&root)?;
    let manifest = Manifest::load()?;
    let stage = Stage::new("junie", &root)?;
    let runtime = stage.0.join(&manifest.native_revision);
    for (name, body) in [("linux-artifacts.json", MANIFEST), ("install_linux_native.py", INSTALLER)]
    {
        fs::write(stage.0.join(name), body).map_err(|_| "Cannot stage Junie Linux installer")?;
    }
    let script = stage.0.join("install_linux_native.py");
    crate::install::run_installer(
        "python3",
        &[
            "-I",
            script.to_str().ok_or("Invalid Junie Linux installer path")?,
            stage.0.to_str().ok_or("Invalid Junie Linux staging path")?,
            runtime.to_str().ok_or("Invalid Junie Linux runtime path")?,
            std::env::consts::ARCH,
        ],
        tokio::time::Instant::now() + Duration::from_secs(300),
    )
    .await?;
    verify(&runtime, &manifest)?;
    if fs::rename(&runtime, root.join(&manifest.native_revision)).is_err()
        && discover(&root)?.is_none()
    {
        return Err("Cannot publish Junie Linux runtime".into());
    }
    discover(&root)?.ok_or("Junie Linux runtime was not installed")?;
    Ok(())
}

fn validate_runtime_parent(root: &Path) -> Result<(), String> {
    let uid = unsafe { libc::geteuid() };
    for ancestor in root.ancestors() {
        let metadata =
            ancestor.symlink_metadata().map_err(|_| "Cannot inspect Junie runtime parent")?;
        let mode = metadata.permissions().mode();
        if !metadata.is_dir()
            || !matches!(metadata.uid(), 0) && metadata.uid() != uid
            || (mode & 0o022 != 0 && (ancestor == root || mode & 0o1000 == 0))
            || (ancestor == root && metadata.uid() != uid)
        {
            return Err("Insecure Junie Linux runtime parent".into());
        }
    }
    Ok(())
}

fn verify(root: &Path, manifest: &Manifest) -> Result<(), String> {
    let target =
        manifest.targets.get(std::env::consts::ARCH).ok_or("Missing Junie Linux target")?;
    let mut files = BTreeMap::new();
    let uid = unsafe { libc::geteuid() };
    collect(root, root, uid, &mut files).map_err(|_| "Cannot verify Junie Linux runtime tree")?;
    if files.get(&manifest.entry) != Some(&0o700) {
        return Err("Missing fixed Junie Linux entry".into());
    }

    // The fixed archive identity covers the launcher, JRE, application and all
    // other regular files. A filename, execution bit and payload form each entry.
    let mut tree = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    for (name, mode) in files {
        let mut file =
            File::open(root.join(&name)).map_err(|_| "Cannot read Junie Linux runtime file")?;
        let mut digest = Sha256::new();
        loop {
            let count =
                file.read(&mut buffer).map_err(|_| "Cannot hash Junie Linux runtime file")?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
        tree.update(name.as_bytes());
        tree.update(if mode == 0o700 { b"\x001" } else { b"\x000" });
        tree.update(digest.finalize());
    }
    if hex::encode(tree.finalize()) != target.tree_sha256 {
        return Err("Junie Linux runtime tree checksum mismatch".into());
    }
    Ok(())
}

fn collect(
    root: &Path,
    path: &Path,
    uid: u32,
    files: &mut BTreeMap<String, u32>,
) -> io::Result<()> {
    let metadata = path.symlink_metadata()?;
    let mode = metadata.permissions().mode() & 0o7777;
    if metadata.uid() == uid {
        if metadata.is_dir() && mode == 0o700 {
            for child in path.read_dir()? {
                collect(root, &child?.path(), uid, files)?;
            }
            return Ok(());
        }
        if metadata.is_file() && matches!(mode, 0o600 | 0o700) && metadata.nlink() == 1 {
            let name = path.strip_prefix(root).ok().and_then(Path::to_str).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Invalid runtime path")
            })?;
            files.insert(name.into(), mode);
            return Ok(());
        }
    }
    Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid runtime entry"))
}
