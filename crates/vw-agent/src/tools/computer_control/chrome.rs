//! Chrome connects through a private one-controller socket. File transfers use
//! bounded inline bytes, never paths; approval arguments contain metadata/digests.
use super::*;
use crate::computer_control::{
    chrome::{bridge::ChromeBridge, origin_policy::canonical_origin},
    policy::GrantScope,
};
use std::{
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
};
use tokio::net::UnixListener;
use vw_config_types::computer_control::GrantDecision;

pub(super) struct ChromeListener {
    listener: UnixListener,
    path: PathBuf,
}
impl Drop for ChromeListener {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
impl ChromeListener {
    pub(super) fn bind() -> anyhow::Result<Self> {
        if !cfg!(target_os = "macos") {
            bail!("Chrome desktop control is unsupported on this platform");
        }
        let home = PathBuf::from(std::env::var_os("HOME").context("missing user home")?);
        let directory = home.join(".shiju/computer-control");
        let info = std::fs::symlink_metadata(&directory)
            .context("install ShiJu Chrome native host first")?;
        if !info.is_dir()
            || info.uid() != unsafe { libc::geteuid() }
            || info.permissions().mode() & 0o777 != 0o700
            || directory.canonicalize()? != home.canonicalize()?.join(".shiju/computer-control")
        {
            bail!("unsafe Chrome host directory");
        }
        let path = directory.join("chrome.sock");
        // Never unlink a socket owned by another live controller.
        let listener = UnixListener::bind(&path)
            .context("Chrome controller already active or socket unavailable")?;
        let bound = Self { listener, path };
        std::fs::set_permissions(&bound.path, std::fs::Permissions::from_mode(0o600))?;
        Ok(bound)
    }
    pub(super) async fn serve(
        self,
        id: ControlSessionId,
        host: &ControlHost,
        controller: UnixStream,
    ) -> std::io::Result<()> {
        let (stream, _) = tokio::time::timeout(Duration::from_secs(30), self.listener.accept())
            .await
            .map_err(|_| std::io::ErrorKind::TimedOut)??;
        let bridge = ChromeBridge::pair(stream, id)
            .await
            .map_err(|_| std::io::ErrorKind::PermissionDenied)?;
        bridge.serve(host, controller).await
    }
}

pub(super) fn validate_command(command: &ChromeCommand) -> anyhow::Result<()> {
    match command {
        ChromeCommand::Snapshot => {}
        ChromeCommand::Click { selector } if !selector.is_empty() && selector.len() <= 2048 => {}
        ChromeCommand::Fill { selector, text }
            if !selector.is_empty() && selector.len() <= 2048 && text.len() <= 16000 => {}
        ChromeCommand::Download { selector, url }
            if !selector.is_empty() && selector.len() <= 2048 =>
        {
            canonical_origin(url).map_err(control_error)?;
        }
        ChromeCommand::Navigate { url } => {
            canonical_origin(url).map_err(control_error)?;
        }
        ChromeCommand::Upload { selector, file_name, content_base64 } => {
            crate::computer_control::chrome::bridge::validate_upload(
                selector,
                file_name,
                content_base64,
            )
            .map_err(control_error)?;
        }
        ChromeCommand::Screenshot | ChromeCommand::Press { .. } | ChromeCommand::Scroll { .. } => {
            bail!("unsupported Chrome capability")
        }
        _ => bail!("invalid Chrome command"),
    }
    Ok(())
}

pub(super) async fn authorize_site(
    policy: &Arc<RwLock<ComputerControlPolicy>>,
    context: &super::super::ToolUseContext,
    origin: &str,
) -> anyhow::Result<()> {
    let user = local_user();
    let version = {
        let p = policy.read();
        let config = p.config().users.get(&user).context("Chrome disabled")?;
        if !config.chrome {
            bail!("Chrome disabled");
        }
        match config.chrome_origins.get(origin) {
            Some(GrantDecision::Allow) => return Ok(()),
            Some(GrantDecision::Deny) => bail!("Chrome site explicitly denied"),
            None => {}
        }
        p.version()
    };
    if !super::super::decision::request_control_scope_approval(
        "computer_control.chrome_site",
        json!({"origin":origin,"scope":"website access until revoked or gateway restart"}),
        context,
    )
    .await
    {
        bail!("Chrome site approval required");
    }
    let mut p = policy.write();
    if p.version() != version {
        bail!("Chrome permission changed during approval; retry");
    }
    p.set_grant(&user, GrantScope::ChromeOrigin(origin.into()), Some(GrantDecision::Allow))
        .map_err(anyhow::Error::msg)?;
    Ok(())
}

pub(super) async fn approve_transfer(
    host: &ControlHost,
    request: &ComputerControlRequest,
) -> anyhow::Result<()> {
    use base64::Engine;
    use sha2::{Digest, Sha256};
    let (scope, metadata) = match &request.action {
        ComputerControlAction::Chrome {
            command: ChromeCommand::Upload { selector, file_name, content_base64 },
        } => {
            let bytes = base64::engine::general_purpose::STANDARD.decode(content_base64)?;
            (
                "computer_control.chrome_upload",
                json!({"selector":selector,"file_name":file_name,"bytes":bytes.len(),"sha256":hex::encode(Sha256::digest(&bytes))}),
            )
        }
        ComputerControlAction::Chrome { command: ChromeCommand::Download { selector, url } } => (
            "computer_control.chrome_download",
            json!({"selector":selector,"url_origin":canonical_origin(url).map_err(control_error)?,"url_path":url::Url::parse(url)?.path(),"url_sha256":hex::encode(Sha256::digest(url.as_bytes())),"limit_bytes":262144,"redirects":"denied","destination":"inline tool output; no local file"}),
        ),
        _ => return Ok(()),
    };
    host.recheck(request).map_err(control_error)?;
    let context = current_tool_use_context().context("file transfer requires executor context")?;
    let input = json!({"target":request.target,"request_id":request.request_id,"permission_version":request.permission_version,"transfer":metadata});
    if !super::super::decision::request_control_scope_approval(scope, input, &context).await {
        bail!("Chrome file transfer approval required");
    }
    host.approve_transfer(request).map_err(control_error)
}

pub(super) fn schema() -> Value {
    let selector = json!({"type":"string","minLength":1,"maxLength":2048});
    json!({"type":"object","additionalProperties":false,"required":["target","action"],"properties":{
        "target":{"type":"object","additionalProperties":false,"required":["kind","tab_id","origin"],"properties":{"kind":{"const":"chrome_tab"},"tab_id":{"type":"integer","minimum":1},"origin":{"type":"string","description":"Canonical HTTP(S) origin, without path, e.g. https://example.com"}}},
        "action":{"oneOf":[
            {"type":"object","additionalProperties":false,"required":["action"],"properties":{"action":{"const":"stop"}}},
            {"type":"object","additionalProperties":false,"required":["action","command"],"properties":{"action":{"const":"chrome"},"command":{"oneOf":[
                {"type":"object","additionalProperties":false,"required":["kind"],"properties":{"kind":{"const":"snapshot"}}},
                {"type":"object","additionalProperties":false,"required":["kind","url"],"properties":{"kind":{"const":"navigate"},"url":{"type":"string","maxLength":8192}}},
                {"type":"object","additionalProperties":false,"required":["kind","selector"],"properties":{"kind":{"const":"click"},"selector":selector}},
                {"type":"object","additionalProperties":false,"required":["kind","selector","url"],"properties":{"kind":{"const":"download"},"selector":selector,"url":{"type":"string","maxLength":8192}}},
                {"type":"object","additionalProperties":false,"required":["kind","selector","text"],"properties":{"kind":{"const":"fill"},"selector":selector,"text":{"type":"string","maxLength":16000}}},
                {"type":"object","additionalProperties":false,"required":["kind","selector","file_name","content_base64"],"properties":{"kind":{"const":"upload"},"selector":selector,"file_name":{"type":"string","pattern":"^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$"},"content_base64":{"type":"string","maxLength":349528}}}
            ]}}}
        ]}
    }})
}

#[cfg(test)]
#[path = "chrome_tests.rs"]
mod tests;
