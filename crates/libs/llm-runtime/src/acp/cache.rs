use std::{collections::HashMap, sync::{Arc, LazyLock}};
use parking_lot::Mutex;
use tokio::sync::mpsc;
use acp_client::AcpClient;
use acp_types::AcpJsonRpcMessage;
pub static ACP_CLIENT_CACHE: LazyLock<Mutex<HashMap<String, Arc<CachedAcpClient>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 绑定到一组 ACP 启动配置的客户端缓存项。
///
/// 同一个 ACP 代理进程会复用客户端连接；`prompt_lock` 用来串行化同一客户端上的 prompt，
/// `output_tx` 则在请求运行期间临时接收 ACP JSON-RPC 输出回调。
pub struct CachedAcpClient {
    pub permission_confirmation: Arc<parking_lot::Mutex<Option<acp_host_io::permissions::PermissionConfirmation>>>,
    pub tenant: vw_api_transport::TenantScope,
    pub runtime_identity: Option<String>,
    pub client: Arc<AcpClient>,
    pub prompt_lock: Arc<tokio::sync::Mutex<()>>,
    pub output_tx: Arc<Mutex<Option<mpsc::UnboundedSender<AcpJsonRpcMessage>>>>,
    // Native peers can retain terminals between prompts. Keep their actual cwd
    // protected until the cached client is retired; drop this after the client.
    pub worktree_use: Mutex<Option<Arc<std::fs::File>>>,
}
