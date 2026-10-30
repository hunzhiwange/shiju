//! Stage original packages and provenance before publishing a complete, non-overwriting install.

use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

use super::identity::{metadata_path, read_install_metadata, write_install_metadata};
use crate::plugins::install::{
    PluginInstallResult, copy_dir_recursive_secure, ensure_install_root,
    load_plugin_manifest_for_post_copy_audit, observe_post_copy,
};
use crate::plugins::{
    PluginHookSource, PluginIdentity, PluginInstallMetadata, PluginManifest, PluginMcpServers,
    PluginPath,
};

pub(crate) fn install_package(
    source: &PluginManifest,
    identity: PluginIdentity,
    plugins_dir: &Path,
) -> Result<PluginInstallResult> {
    reject_installation_inside_source(&source.root, plugins_dir)?;
    ensure_install_root(plugins_dir)?;
    let canonical_parent = plugins_dir.canonicalize().context("failed to resolve install root")?;
    if canonical_parent.starts_with(&source.root) {
        bail!("Plugin install directory must not be inside the source package");
    }
    let destination = plugins_dir.join(identity.as_key());
    ensure_absent(&destination, "Destination plugin")?;
    let destination_metadata = metadata_path(&destination)?;
    ensure_absent(&destination_metadata, "Plugin installation metadata")?;
    let source_manifest_bytes =
        fs::read(&source.manifest_path).context("failed to snapshot source plugin manifest")?;
    let transaction = tempfile::Builder::new()
        .prefix(".plugin-install-")
        .tempdir_in(&canonical_parent)
        .context("failed to create temporary plugin installation directory")?;
    // The outer staging directory must never resemble a manifest root, even when an existing
    // native plugin name is itself a reserved manifest-directory name such as `.codex-plugin`.
    let staged = transaction.path().join("payload").join(identity.as_key());
    let result = (|| {
        copy_dir_recursive_secure(&source.root, &staged)?;
        observe_post_copy(&staged);
        let mut manifest = load_plugin_manifest_for_post_copy_audit(&staged)
            .context("installed plugin failed post-copy audit")?;
        if manifest.name != source.name {
            bail!(
                "Installed plugin manifest name changed from '{}' to '{}'",
                source.name,
                manifest.name
            );
        }
        if manifest.version != source.version {
            bail!("Installed plugin manifest version changed during installation");
        }
        if manifest.manifest_path.strip_prefix(&manifest.root)?
            != source.manifest_path.strip_prefix(&source.root)?
        {
            bail!("Installed plugin selected manifest changed during installation staging");
        }
        if fs::read(&manifest.manifest_path).context("failed to read staged plugin manifest")?
            != source_manifest_bytes
        {
            bail!("Installed plugin manifest contents changed during installation staging");
        }
        let metadata = PluginInstallMetadata {
            identity: identity.clone(),
            source: source.root.to_string_lossy().into_owned(),
            version: source.version.clone(),
        };
        write_install_metadata(&staged, &metadata)?;
        if read_install_metadata(&staged, &manifest)? != Some(metadata) {
            bail!("Staged plugin installation metadata did not preserve its identity");
        }
        rebase_manifest(&mut manifest, &canonical_parent.join(identity.as_key()))?;

        // Metadata is complete before the package becomes discoverable. An exclusive hard link
        // also claims this identity between concurrent installers without replacing any state.
        fs::hard_link(metadata_path(&staged)?, &destination_metadata)
            .context("failed to publish plugin installation metadata without overwriting")?;
        if let Err(error) = publish_directory(&staged, &destination) {
            fs::remove_file(&destination_metadata).with_context(|| {
                format!("{error:#}; failed to roll back owned plugin installation metadata")
            })?;
            return Err(error);
        }
        Ok(PluginInstallResult { installed_dir: destination, manifest, identity })
    })();
    match result {
        Ok(installed) => Ok(installed),
        Err(error) => {
            transaction.close().with_context(|| {
                format!("{error:#}; failed to clean temporary plugin installation")
            })?;
            Err(error)
        }
    }
}

// Check an existing ancestor before creating any directory: rejecting a nested destination must
// not add directories to the original package, and staging there could recursively copy itself.
fn reject_installation_inside_source(source: &Path, destination: &Path) -> Result<()> {
    let absolute = if destination.is_absolute() {
        destination.to_path_buf()
    } else {
        std::env::current_dir()
            .context("failed to resolve plugin install directory")?
            .join(destination)
    };
    let mut ancestor = absolute.as_path();
    loop {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => {
                if ancestor == absolute && metadata.file_type().is_symlink() {
                    bail!(
                        "Plugin install directory must not be a symlink: {}",
                        destination.display()
                    );
                }
                let canonical =
                    ancestor.canonicalize().context("failed to resolve plugin install ancestor")?;
                if canonical.starts_with(source) {
                    bail!("Plugin install directory must not be inside the source package");
                }
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                ancestor = ancestor
                    .parent()
                    .context("plugin install directory has no existing ancestor")?;
            }
            Err(error) => return Err(error).context("failed to inspect plugin install directory"),
        }
    }
}

fn ensure_absent(path: &Path, kind: &str) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => bail!("{kind} already exists: {}", path.display()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("failed to inspect {kind}")),
    }
}

/// Rebase every audited component before publication, so success needs no fallible path reload.
fn rebase_manifest(manifest: &mut PluginManifest, destination: &Path) -> Result<()> {
    let old_root = manifest.root.clone();
    let relocate = |path: &mut PathBuf| -> Result<()> {
        *path = destination.join(
            path.strip_prefix(&old_root)
                .context("staged plugin path escaped its audited package")?,
        );
        Ok(())
    };
    let component = |path: &mut PluginPath| relocate(&mut path.canonical_path);
    relocate(&mut manifest.manifest_path)?;
    for path in &mut manifest.skills {
        component(path)?;
    }
    if let Some(PluginMcpServers::Path(path)) = &mut manifest.mcp_servers {
        component(path)?;
    }
    for hook in &mut manifest.hooks {
        if let PluginHookSource::Path(path) = hook {
            component(path)?;
        }
    }
    for path in [&mut manifest.apps, &mut manifest.onboarding_skill].into_iter().flatten() {
        component(path)?;
    }
    if let Some(interface) = &mut manifest.interface {
        for path in [&mut interface.composer_icon, &mut interface.logo, &mut interface.logo_dark]
            .into_iter()
            .flatten()
        {
            component(path)?;
        }
        for path in &mut interface.screenshots {
            component(path)?;
        }
    }
    manifest.root = destination.to_path_buf();
    Ok(())
}

fn publish_directory(source: &Path, destination: &Path) -> Result<()> {
    // A plain Unix rename can replace a concurrently-created empty directory; the kernel must
    // enforce no-replace in the same operation which publishes the staged tree.
    #[cfg(any(target_vendor = "apple", target_os = "linux"))]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let source = CString::new(source.as_os_str().as_bytes())?;
        let destination = CString::new(destination.as_os_str().as_bytes())?;
        #[cfg(target_vendor = "apple")]
        // SAFETY: both C strings are NUL-terminated and live throughout the no-replace rename.
        let result =
            unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) };
        #[cfg(target_os = "linux")]
        // SAFETY: paths are valid C strings; AT_FDCWD and RENAME_NOREPLACE are kernel constants.
        let result = unsafe {
            libc::syscall(
                libc::SYS_renameat2,
                libc::AT_FDCWD,
                source.as_ptr(),
                libc::AT_FDCWD,
                destination.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        if result != 0 {
            return Err(std::io::Error::last_os_error())
                .context("failed to publish plugin directory without overwriting");
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        // Windows directory rename refuses an existing directory, including an empty one.
        fs::rename(source, destination)
            .context("failed to publish plugin directory without overwriting")
    }
    #[cfg(not(any(target_vendor = "apple", target_os = "linux", windows)))]
    {
        let _ = (source, destination);
        bail!("Atomic non-overwriting plugin directory publication is unsupported on this platform")
    }
}
