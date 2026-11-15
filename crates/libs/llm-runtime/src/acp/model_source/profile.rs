//! Request-owned launch material. No Debug/Serialize: credentials never form a record.
use agent_config::schema::AcpAgentConfig;
use model_transport::{RelayBinding, RelayIdentity};
use model_relay_http::RelayServer;
use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

pub const REFERENCE_ENV: &str = "VW_ACP_LAUNCH_REFERENCE";

pub fn launch_reference(
    identity: &RelayIdentity,
) -> anyhow::Result<String> {
    use sha2::Digest;
    Ok(format!(
        "vw-session-model:{}",
        hex::encode(sha2::Sha256::digest(serde_json::to_vec(&serde_json::json!({
            "tenant": identity.tenant, "project": identity.project, "session": identity.session,
            "provider": identity.provider, "model": identity.model, "generation": identity.generation,
        }))?))
    ))
}

/// Only the request owner may expose this configuration to the child process.
/// Dropping the profile revokes HTTP access before removing its private files.
pub struct AcpLaunchProfile {
    config: AcpAgentConfig,
    directory: tempfile::TempDir,
    pub binding: Arc<RelayBinding>,
    server: Option<RelayServer>,
    pub gateway: Option<super::gateway::OwnedGateway>,
    #[allow(dead_code)] // Read by individual CLI drivers as they are verified.
    pub model_alias: String,
    pub model_output_limit: u64,
    pub model_context_limit: u64,
    pub model_supports_temperature: bool,
    pub model_supports_reasoning: bool,
    pub model_reasoning_options:
        Option<Vec<model_reasoning::ReasoningOption>>,
    pub model_supports_tool_call: bool,
    pub state_root: Option<PathBuf>,
    pub model_api: Option<model_types::ApiInfo>,
    /// A selected native provider/model that must be applied by ACP RPC before prompt.
    pub session_model_id: Option<String>,
}
impl AcpLaunchProfile {
    pub fn new(config: &AcpAgentConfig, identity: RelayIdentity) -> anyhow::Result<Self> {
        anyhow::ensure!(!config.env.contains_key(REFERENCE_ENV), "reserved ACP launch environment");
        // No Windows launch is allowed until a restrictive DACL can be verified.
        #[cfg(not(unix))]
        {
            let _ = identity;
            anyhow::bail!(
                "private ACP launch directory ACL verification unavailable on this platform"
            );
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let directory = tempfile::Builder::new()
                .prefix("shiju-acp-launch-")
                .permissions(std::fs::Permissions::from_mode(0o700))
                .tempdir()?;
            anyhow::ensure!(
                directory.path().symlink_metadata()?.permissions().mode() & 0o777 == 0o700,
                "insecure ACP launch directory"
            );
            let reference = launch_reference(&identity)?;
            let model_alias = identity.model.clone();
            let binding = RelayBinding::for_prompt(identity)?;
            let mut config = config.clone();
            config.env.insert(REFERENCE_ENV.into(), reference);
            Ok(Self {
                config,
                directory,
                binding,
                server: None,
                gateway: None,
                model_alias,
                model_output_limit: model_state::DEFAULT_MODEL_OUTPUT_LIMIT,
                model_context_limit: model_state::DEFAULT_MODEL_CONTEXT_LIMIT,
                model_supports_temperature: false,
                model_supports_reasoning: false,
                model_reasoning_options: None,
                model_supports_tool_call: false,
                state_root: None,
                model_api: None,
                session_model_id: None,
            })
        }
    }
    /// Apply the model snapshot freshly resolved by the host for this launch.
    pub fn apply_model(&mut self, resolved: &model_types::Model) {
        self.model_output_limit = resolved.limit.output;
        self.model_context_limit = resolved.limit.context;
        self.model_supports_temperature = resolved.capabilities.temperature;
        self.model_supports_reasoning = resolved.capabilities.reasoning;
        self.model_reasoning_options = resolved.capabilities.reasoning_options.clone();
        self.model_supports_tool_call = resolved.capabilities.toolcall;
        self.model_api = Some(resolved.api.clone());
    }

    pub fn config(&self) -> &AcpAgentConfig {
        &self.config
    }
    #[allow(dead_code)] // Shared boundary used by the subsequent CLI drivers.
    pub fn directory(&self) -> &Path {
        self.directory.path()
    }
    /// Persistent SDK history is separate from revocable request credentials.
    pub fn claude_state_directory(&self) -> anyhow::Result<PathBuf> {
        #[cfg(not(unix))]
        anyhow::bail!("private ACP state directory ACL verification unavailable");
        #[cfg(unix)]
        {
            use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
            let root = self.state_root.as_deref().unwrap_or(self.directory.path());
            anyhow::ensure!(root.is_absolute() && root.is_dir(), "invalid ACP state root");
            let reference = self.config.env[REFERENCE_ENV]
                .strip_prefix("vw-session-model:")
                .ok_or_else(|| anyhow::anyhow!("invalid ACP state identity"))?;
            let mut path = root.to_path_buf();
            for component in ["acp-claude-state", reference] {
                path.push(component);
                match std::fs::DirBuilder::new().mode(0o700).create(&path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(error) => return Err(error.into()),
                }
                let metadata = path.symlink_metadata()?;
                anyhow::ensure!(
                    metadata.is_dir()
                        && !metadata.file_type().is_symlink()
                        && metadata.permissions().mode() & 0o777 == 0o700
                        && metadata.uid() == unsafe { libc::geteuid() },
                    "insecure ACP state directory"
                );
            }
            Ok(path)
        }
    }
    pub fn attach_server(&mut self, server: impl Into<RelayServer>) {
        self.server = Some(server.into());
    }

    /// The driver must enumerate every key it controls. Conflicting user values fail closed.
    #[allow(dead_code)] // Shared boundary used by the subsequent CLI drivers.
    pub fn overlay_environment(
        &mut self,
        overlay: HashMap<String, String>,
    ) -> anyhow::Result<()> {
        for (key, value) in &overlay {
            anyhow::ensure!(
                !matches!(
                    key.to_ascii_uppercase().as_str(),
                    "HOME" | "USERPROFILE" | "VW_ACP_LAUNCH_REFERENCE"
                ) && !key.is_empty()
                    && !key.contains(['=', '\0'])
                    && !value.contains('\0'),
                "unsafe ACP environment override"
            );
            anyhow::ensure!(
                !self.config.env.keys().any(|existing| existing.eq_ignore_ascii_case(key)),
                "conflicting ACP environment override"
            );
        }
        self.config.env.extend(overlay);
        Ok(())
    }
    /// Credentials belong in the private environment or file, never in process argv.
    #[allow(dead_code)] // Used by drivers requiring explicit config/model flags.
    pub fn append_arguments(&mut self, args: &[String]) -> anyhow::Result<()> {
        anyhow::ensure!(
            args.iter()
                .all(|arg| !arg.contains('\0') && !arg.contains(self.binding.bearer_token())),
            "unsafe ACP launch argument"
        );
        self.config.args.extend_from_slice(args);
        Ok(())
    }

    /// Atomically publish a new private file; no path traversal, symlink reuse or overwrite.
    #[allow(dead_code)] // Shared boundary used by the subsequent CLI drivers.
    pub fn write_config(&self, name: &str, bytes: &[u8]) -> anyhow::Result<PathBuf> {
        anyhow::ensure!(
            !name.is_empty()
                && Path::new(name).components().count() == 1
                && matches!(
                    Path::new(name).components().next(),
                    Some(std::path::Component::Normal(_))
                ),
            "invalid ACP config filename"
        );
        let path = self.directory.path().join(name);
        let mut file = tempfile::NamedTempFile::new_in(self.directory.path())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            anyhow::ensure!(
                file.as_file().metadata()?.permissions().mode() & 0o777 == 0o600,
                "insecure ACP config file"
            );
        }
        file.write_all(bytes)?;
        file.as_file().sync_all()?;
        file.persist_noclobber(&path)
            .map_err(|_| anyhow::anyhow!("ACP private config already exists"))?;
        Ok(path)
    }
}
impl Drop for AcpLaunchProfile {
    fn drop(&mut self) {
        self.binding.revoke();
        // Reap the owned Gateway before its configuration and credentials disappear.
        drop(self.gateway.take());
    }
}

#[cfg(all(test, unix))]
#[path = "profile_tests.rs"]
mod tests;
