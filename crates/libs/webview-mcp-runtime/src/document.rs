//! Initial document assembly and trusted document revocation.
use std::io::{self, BufRead, Write};
use std::sync::Mutex;
use serde_json::Value;
use vw_api_types::mcp_app_network::McpAppNetworkPolicy;
use webview_mcp_assets::{DOCUMENT_SCRIPT, render_wrapper};
use webview_mcp_policy::{ENGINE_APP_URL, allowed_origins, app_csp, wrapper_csp};

pub(crate) struct Document {
    pub(crate) nonce: String,
    pub(crate) policy: McpAppNetworkPolicy,
    pub(crate) form_origins: Vec<String>,
    pub(crate) form_enabled: bool,
    pub(crate) app_csp: String,
    pub(crate) wrapper_csp: String,
    pub(crate) wrapper: Vec<u8>,
    pub(crate) app: Mutex<Option<Vec<u8>>>,
    pub(crate) bootstrap: String,
}

pub(crate) fn read_document() -> Result<Document, Box<dyn std::error::Error>> {
    let mut line = String::new();
    std::io::Read::take(io::stdin().lock(), 8 * 1024 * 1024 + 1).read_line(&mut line)?;
    if line.len() > 8 * 1024 * 1024 {
        return Err("MCP app resource too large".into());
    }
    let input: Value = serde_json::from_str(&line)?;
    let nonce = input["nonce"]
        .as_str()
        .filter(|s| s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("missing host nonce")?
        .to_owned();
    let html = input["html"].as_str().ok_or("missing app HTML")?;
    let policy = McpAppNetworkPolicy::parse(input.get("csp"))?;
    let form_origins = allowed_origins(&policy);
    let form_enabled = !form_origins.is_empty();
    let app_csp = app_csp(&policy);
    let wrapper_csp = wrapper_csp(&form_origins);
    let document_nonce = format!("{:016x}{:016x}", getrandom::u64()?, getrandom::u64()?);
    let wrapper = render_wrapper(&nonce, &document_nonce, form_enabled).into_bytes();
    let app = Mutex::new(Some(html.as_bytes().to_vec()));
    let bootstrap = format!(
        "(() => {{
            if (window === top || parent !== top || location.href !== '{ENGINE_APP_URL}') return;
            const documentNonce = '{document_nonce}';
            {}
        }})();",
        DOCUMENT_SCRIPT,
    );
    Ok(Document {
        nonce,
        policy,
        form_origins,
        form_enabled,
        app_csp,
        wrapper_csp,
        wrapper,
        app,
        bootstrap,
    })
}

// A line distinct from the page's RPC envelope, emitted only by trusted helper code.
pub(crate) fn revoke_document() {
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "mcp-app-closed").and_then(|_| stdout.flush());
}
