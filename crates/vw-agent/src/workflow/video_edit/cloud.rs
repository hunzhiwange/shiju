//! Native-client cloud transport. Remote business operations must be explicitly enabled.
//!
//! Verified official contracts: `get_my_permissions` takes an empty object;
//! `generation_status` takes only string `taskId` (3–200 characters), without
//! `saveToLibrary` (which mutates the library). Supported remote statuses are
//! queued/running/succeeded/failed; unknown states require reconciliation.
//! occ `--out` contains already-unwrapped JSON; stdout may be truncated at 4,000
//! characters. Exit 1 = tool/RPC error, 2 = usage/auth, 3 = transport, 4 = wait timeout.
//! Only the supported input schemas and sanitized read results leave this module.
use crate::skills::Skill;
use crate::tools::{ToolRuntimeContext, execute_tool_call};
use crate::workflow::{WorkflowToolProvider, WorkflowToolRequest, WorkflowToolResult};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const MAX_RECEIPT_BYTES: u64 = 16 * 1024 * 1024;
const READ_OPERATIONS: &[&str] = &["get_my_permissions", "generation_status"];

// Capture everything inside the authenticated process boundary. Neither credentials nor
// caller output is returned to bash hooks, observers, or the model. No shell evaluates argv.
const CAPTURE: &str = r#"import json, os, pathlib, subprocess, sys
p = pathlib.Path(sys.argv[1])
r = json.loads(p.read_text(encoding='utf-8'))
e = {k:v for k,v in os.environ.items() if not k.startswith(('SHIJU_', 'OCC_'))}
e['OCC_SHIJU_ENV'] = r['auth']
e['OCC_STATE_DIR'] = str(p.parent / 'occ-state')
key_present = False
try:
    for line in pathlib.Path(r['auth']).read_text(encoding='utf-8').splitlines():
        line = line.strip()
        if line.startswith('export '): line = line[7:].strip()
        if line.startswith('SHIJU_KEY=') and line.split('=',1)[1].strip().strip('\"\''):
            key_present = True
except OSError:
    pass
code = 2
os_error = None
if key_present:
    with open(p.parent / 'stdout', 'wb') as out, open(p.parent / 'stderr', 'wb') as err:
        try:
            code = subprocess.run(r['argv'], cwd=r['cwd'], env=e, stdin=subprocess.DEVNULL, stdout=out, stderr=err).returncode
        except OSError as error:
            code = 2
            os_error = error.errno
(p.parent / 'exit.json').write_text(json.dumps({'exit_code':code, 'missing_credentials':not key_present, 'os_error':os_error}))
"#;

pub(super) fn visible_shiju_root(skills: &[Skill]) -> Result<PathBuf, String> {
    let mut matching = skills.iter().filter(|skill| skill.name == "shiju");
    let skill = matching.next().ok_or("当前会话没有授权的 shiju 技能")?;
    if matching.next().is_some() {
        return Err("当前会话有多个 shiju 实例，请先明确选择".into());
    }
    let location = skill.location.as_ref().ok_or("shiju 技能缺少客户端来源路径")?;
    let file = location.canonicalize().map_err(|_| "shiju 技能路径不可用")?;
    if !file.is_file() || file.file_name().and_then(|s| s.to_str()) != Some("SKILL.md") {
        return Err("shiju 来源必须是当前加载的 SKILL.md".into());
    }
    Ok(file.parent().ok_or("shiju 技能缺少包目录")?.to_path_buf())
}

/// Bound to the visible native skill and its original tool context; never loads another agent.
/// This transport currently exposes only permission and generation-status reads. Later
/// preparation/apply stages add their own typed operations rather than arbitrary occ commands.
pub struct ShijuCloudVideoToolProvider {
    context: ToolRuntimeContext,
    allowed_tools: Option<Value>,
    hub_root: PathBuf,
    occ: PathBuf,
    schemas: std::collections::BTreeMap<String, Value>,
    installed_version: String,
    active_version: String,
}

impl ShijuCloudVideoToolProvider {
    /// `skills` must be the native processor's already capability-filtered snapshot.
    /// Discovery fails closed; it does not log in, upgrade, choose tenants, or export media.
    pub async fn discover(
        context: ToolRuntimeContext,
        skills: &[Skill],
        allowed_tools: Option<Value>,
    ) -> Result<Self, String> {
        let hub_root = visible_shiju_root(skills)?;
        if cfg!(windows) {
            return Err("当前传输只支持 POSIX 官方调用器，Windows 调用合同尚未验证".into());
        }
        for tool in ["bash", "bash_wait", "bash_cancel"] {
            if let Some(allowed) = &allowed_tools {
                let allowed = allowed.as_array().ok_or("allowed_tools 必须是数组")?;
                if !allowed.iter().any(|v| v.as_str() == Some(tool)) {
                    return Err("云端传输需要当前会话授权完整进程生命周期工具".into());
                }
            }
        }
        let mut provider = Self {
            context,
            allowed_tools,
            hub_root,
            occ: PathBuf::new(),
            schemas: Default::default(),
            installed_version: String::new(),
            active_version: String::new(),
        };
        provider.authorize_path(&provider.hub_root)?;
        let cli = provider.hub_root.join("scripts/shiju");
        provider.authorize_path(&cli)?;
        let version = provider.cli(&cli, &["hub:version".into()]).await?;
        let package = Path::new(required_string(&version, "package_root")?)
            .canonicalize()
            .map_err(|_| "客户端包目录不可用")?;
        let current_version = required_string(&version, "current_version")?;
        if package != provider.hub_root
            || version["package_skill_version"].as_str() != Some(current_version)
        {
            return Err("shiju 客户端版本或来源不一致".into());
        }
        let gate = provider.cli(&cli, &["hub:check".into()]).await?;
        if gate["force_upgrade_required"].as_bool() != Some(false) {
            return Err("shiju 版本门禁要求升级或返回无效；请按客户端升级协作处理".into());
        }
        provider.cli(&cli, &["index".into(), "rebuild".into()]).await?;
        let installed = provider.cli(&cli, &["skill:me".into()]).await?;
        let items = installed["items"].as_array().ok_or("已安装技能列表无效")?;
        let mut matches = items.iter().filter(|v| v["slug"] == "cutvideo");
        let item = matches.next().ok_or("当前客户端未安装或未授权 cutvideo")?;
        if matches.next().is_some() {
            return Err("多个已安装 cutvideo 来源，请先在客户端明确选择".into());
        }
        if !matches!(item["distribution_source"].as_str(), Some("tenant" | "marketplace"))
            || required_string(item, "consumer_company_id").is_err()
        {
            return Err("cutvideo 安装来源或消费范围缺失".into());
        }
        let id = match &item["skill_id"] {
            Value::Number(n) if n.as_u64().is_some_and(|id| id > 0) => n.to_string(),
            Value::String(s) if s.parse::<u64>().is_ok_and(|id| id > 0) => s.clone(),
            _ => return Err("已安装技能身份无效".into()),
        };
        let fetched = provider
            .cli(&cli, &["skill:fetch".into(), "--skill-id".into(), id.clone(), "--in-use".into()])
            .await?;
        let fetched_id = fetched["skill_id"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| fetched["skill_id"].to_string());
        if fetched_id != id
            || fetched["slug"] != "cutvideo"
            || fetched["distribution_source"] != item["distribution_source"]
            || fetched["consumer_company_id"] != item["consumer_company_id"]
            || !matches!(fetched["source"].as_str(), Some("hub" | "dev"))
        {
            return Err("cutvideo 身份、来源或消费范围不一致".into());
        }
        provider.installed_version = required_string(&fetched, "installed_version")?.into();
        provider.active_version = required_string(&fetched, "active_version")?.into();
        if fetched["source"] == "hub" && provider.active_version != provider.installed_version {
            return Err("cutvideo 正式缓存活动版本与安装版本不一致".into());
        }
        if item["installed_version"].as_str() != Some(provider.installed_version.as_str()) {
            return Err("cutvideo 安装版本在发现期间变化，请重新开始".into());
        }
        let root = Path::new(required_string(&fetched, "path")?)
            .canonicalize()
            .map_err(|_| "cutvideo 有效版本路径不可用")?;
        provider.authorize_path(&root)?;
        provider.occ =
            root.join("scripts/occ.py").canonicalize().map_err(|_| "cutvideo 缺少官方调用器")?;
        if !provider.occ.starts_with(&root) {
            return Err("cutvideo 调用器逃逸有效版本目录".into());
        }
        provider.authorize_path(&provider.occ)?;
        // Read official usage before making schema or business requests.
        let usage = provider.occ_capture(&["--help".into()]).await?;
        if !usage.contains("--args-file")
            || !usage.contains("--out")
            || !usage.contains("--no-wait")
        {
            return Err("官方调用器不支持受控完整回执合同".into());
        }
        let tools = provider.occ_capture(&["tools".into()]).await?;
        for operation in READ_OPERATIONS {
            if !tools.lines().any(|line| line.split('\t').next() == Some(operation)) {
                return Err(format!("云端缺少已支持操作: {operation}"));
            }
            let schema = provider.occ_capture(&["schema".into(), (*operation).into()]).await?;
            let schema: Value = serde_json::from_str(&schema).map_err(|_| "云端 schema 无效")?;
            if schema["name"] != *operation || schema["inputSchema"]["type"] != "object" {
                return Err("云端 schema 身份或类型不一致".into());
            }
            if *operation == "generation_status"
                && schema["inputSchema"]["properties"]["taskId"]["type"] != "string"
            {
                return Err("云端 taskId schema 已变化".into());
            }
            let supported = if *operation == "generation_status" {
                json!({"type":"object","properties":{"taskId":{"type":"string","minLength":3,"maxLength":200}},"required":["taskId"],"additionalProperties":false})
            } else {
                json!({"type":"object","properties":{},"additionalProperties":false})
            };
            provider.schemas.insert((*operation).into(), supported);
        }
        Ok(provider)
    }

    /// Verified input contracts only, without remote descriptions or business templates.
    pub fn schemas(&self) -> &std::collections::BTreeMap<String, Value> {
        &self.schemas
    }
    pub fn versions(&self) -> (&str, &str) {
        (&self.installed_version, &self.active_version)
    }

    fn authorize_path(&self, path: &Path) -> Result<(), String> {
        if !path.is_absolute()
            || !self.context.path_security_policy().is_resolved_path_allowed(path)
        {
            return Err("原生安全策略拒绝云端调用路径".into());
        }
        Ok(())
    }

    fn cancelled(&self) -> bool {
        self.context.tool_use_context().abort_token().is_some_and(|token| token.is_cancelled())
    }

    fn native(&self, name: &str, input: Value) -> Result<Value, String> {
        if let Some(allowed) = &self.allowed_tools {
            let allowed = allowed.as_array().ok_or("allowed_tools 必须是数组")?;
            if !allowed.iter().any(|v| v.as_str() == Some(name)) {
                return Err("云端传输工具不在当前会话能力范围内".into());
            }
        }
        execute_tool_call(name, &input.to_string(), &self.context)
            .map(|r| r.data)
            // Never propagate hook/tool errors that may echo private request content.
            .map_err(|e| match e {
                crate::tools::ToolCallError::Denied { .. } => "原生权限或审批拒绝云端传输".into(),
                crate::tools::ToolCallError::Failed(_) => "原生云端传输工具失败".into(),
            })
    }

    async fn cli(&self, cli: &Path, args: &[String]) -> Result<Value, String> {
        let argv = std::iter::once(cli.to_string_lossy().into_owned())
            .chain(args.iter().cloned())
            .collect();
        let text = self.capture(argv, None).await.map_err(|e| format!("{}: {e}", args[0]))?;
        let receipt: Value = serde_json::from_str(&text).map_err(|_| "客户端完整回执无效")?;
        if receipt["code"] != 200 {
            return Err("客户端技能服务拒绝请求；请在原客户端处理授权、版本或来源问题".into());
        }
        receipt.get("data").filter(|v| v.is_object()).cloned().ok_or("客户端回执缺少 data".into())
    }

    async fn occ_capture(&self, args: &[String]) -> Result<String, String> {
        let argv = ["python3".to_owned(), self.occ.to_string_lossy().into_owned()]
            .into_iter()
            .chain(args.iter().cloned())
            .collect();
        self.capture(argv, None).await
    }

    async fn capture(
        &self,
        mut argv: Vec<String>,
        arguments: Option<Value>,
    ) -> Result<String, String> {
        if self.cancelled() {
            return Err("cancelled".into());
        }
        let workspace = self.context.path_security_policy().workspace_dir.clone();
        self.authorize_path(&workspace)?;
        let dir = tempfile::Builder::new()
            .prefix(".video-cloud-")
            .tempdir_in(&workspace)
            .map_err(|_| "无法创建私有云端回执目录")?;
        let dir = dir.path().canonicalize().map_err(|_| "私有回执目录不可用")?;
        let receipt = dir.join("receipt.json");
        let expects_receipt = arguments.is_some();
        if let Some(arguments) = arguments {
            private_write(
                &dir.join("args.json"),
                &serde_json::to_vec(&arguments).map_err(|_| "云端参数编码失败")?,
            )?;
            argv.extend([
                "--args-file".into(),
                dir.join("args.json").to_string_lossy().into_owned(),
                "--out".into(),
                receipt.to_string_lossy().into_owned(),
                "--no-wait".into(),
            ]);
        }
        private_write(
            &dir.join("request.json"),
            &serde_json::to_vec(
                &json!({"argv":argv,"cwd":self.hub_root,"auth":self.hub_root.join(".env")}),
            )
            .map_err(|_| "云端传输编码失败")?,
        )?;
        let manifest = dir.join("request.json");
        let command = format!(
            "python3 -c {} {}",
            shell_words::quote(CAPTURE),
            shell_words::quote(manifest.to_str().ok_or("调用路径不是 UTF-8")?)
        );
        let mut result = self.native("bash", json!({"command":command,"description":"受控云端剪辑传输（私有回执）","yield_time_ms":250,"timeout":300000}))?;
        let mut process = CloudProcess {
            provider: self,
            id: result["process_id"].as_str().ok_or("原生传输缺少进程标识")?.into(),
            terminal: result["state"] != "running",
        };
        while result["state"] == "running" {
            let process_id = result["process_id"].as_str().ok_or("原生传输缺少进程标识")?;
            if self.cancelled() {
                process.cancel()?;
                return Err("cancelled".into());
            }
            tokio::task::yield_now().await;
            result = self.native(
                "bash_wait",
                json!({"process_id":process_id,"cursor":result["next_cursor"],"yield_time_ms":250}),
            )?;
        }
        process.terminal = true;
        if result["state"] != "exited" || result["exit_code"] != 0 {
            return Err("原生传输未正常结束".into());
        }
        let status: Value = serde_json::from_str(&read_receipt(&dir.join("exit.json"))?)
            .map_err(|_| "调用器退出回执无效")?;
        if status["missing_credentials"] == true {
            return Err("当前客户端缺少凭据；请在原客户端登录".into());
        }
        let code = status["exit_code"].as_i64().ok_or("调用器退出码无效")?;
        if let Some(errno) = status["os_error"].as_i64() {
            return Err(format!("云端调用器进程创建失败: errno={errno}"));
        }
        if code != 0 {
            return Err(format!(
                "云端调用器失败: {}",
                match code {
                    1 => "工具或 RPC 错误",
                    2 => "用法或认证错误",
                    3 => "传输错误",
                    4 => "等待超时",
                    _ => "未知退出码",
                }
            ));
        }
        let output = if expects_receipt { receipt } else { dir.join("stdout") };
        read_receipt(&output)
    }
}

// Cancellation through the same native chain also runs if this future is dropped.
struct CloudProcess<'a> {
    provider: &'a ShijuCloudVideoToolProvider,
    id: String,
    terminal: bool,
}
impl CloudProcess<'_> {
    fn cancel(&mut self) -> Result<(), String> {
        let result = self.provider.native("bash_cancel", json!({"process_id":self.id}))?;
        if !matches!(result["state"].as_str(), Some("exited" | "cancelled" | "timed_out")) {
            return Err("云端传输取消尚未确认".into());
        }
        self.terminal = true;
        Ok(())
    }
}
impl Drop for CloudProcess<'_> {
    fn drop(&mut self) {
        if !self.terminal && self.cancel().is_err() {
            tracing::warn!("原生云端传输清理被拒绝，需确认进程终态");
        }
    }
}

fn private_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).and_then(|mut f| f.write_all(bytes)).map_err(|_| "私有请求写入失败".into())
}

fn read_receipt(path: &Path) -> Result<String, String> {
    use std::io::Read;
    let metadata = std::fs::symlink_metadata(path).map_err(|_| "缺少完整调用回执")?;
    if !metadata.is_file() || metadata.len() > MAX_RECEIPT_BYTES {
        return Err("完整调用回执类型或大小无效".into());
    }
    let mut text = String::new();
    std::fs::File::open(path)
        .and_then(|f| f.take(MAX_RECEIPT_BYTES + 1).read_to_string(&mut text))
        .map_err(|_| "完整回执读取失败")?;
    if text.len() as u64 > MAX_RECEIPT_BYTES {
        return Err("完整回执超限".into());
    }
    Ok(text)
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("云端回执缺少字段: {key}"))
}

fn normalize_receipt(receipt: Value, operation: &str) -> Result<Value, String> {
    // occ.call_once unwraps MCP structuredContent (or JSON text) before --out.
    if !receipt.is_object() || receipt.get("error").is_some() {
        return Err("云端返回工具错误或无效回执".into());
    }
    let payload = receipt;
    if operation == "generation_status" {
        let status = required_string(&payload, "status")?;
        if !matches!(status, "queued" | "running" | "succeeded" | "failed") {
            return Err("未知云端作业状态；必须对账，不能重发".into());
        }
        // URLs/messages stay private. Future asset stages consume private receipts directly.
        return Ok(json!({"taskId": required_string(&payload, "taskId")?, "status":status}));
    }
    // Permission reads are a capability check, not an opportunity to echo private payloads.
    if !payload["member"].is_object() {
        return Err("权限回执缺少 member".into());
    }
    Ok(json!({"verified":true}))
}

#[async_trait::async_trait]
impl WorkflowToolProvider for ShijuCloudVideoToolProvider {
    async fn call(&self, request: WorkflowToolRequest) -> Result<WorkflowToolResult, String> {
        if request.provider != "shiju"
            || request.action.is_some()
            || request.credential_id.is_some()
        {
            return Err("云端传输不允许覆盖 provider、action 或凭据".into());
        }
        if !READ_OPERATIONS.contains(&request.tool_name.as_str()) {
            return Err("不支持的受控云端操作".into());
        }
        match request.tool_name.as_str() {
            "get_my_permissions" if !request.inputs.is_empty() => {
                return Err("权限查询不接受参数".into());
            }
            "generation_status" => {
                let id =
                    request.inputs.get("taskId").and_then(Value::as_str).ok_or("缺少 taskId")?;
                if request.inputs.len() != 1 || !(3..=200).contains(&id.chars().count()) {
                    return Err("taskId 参数无效或含未授权字段".into());
                }
            }
            _ => {}
        }
        let argv = vec![
            "python3".into(),
            self.occ.to_string_lossy().into_owned(),
            "call".into(),
            request.tool_name.clone(),
        ];
        let text = self.capture(argv, Some(json!(request.inputs))).await?;
        let receipt = serde_json::from_str(&text).map_err(|_| "云端完整 JSON 回执无效")?;
        let result = normalize_receipt(receipt, &request.tool_name)?;
        if request.tool_name == "generation_status" && result["taskId"] != request.inputs["taskId"]
        {
            return Err("云端作业身份不一致，必须对账".into());
        }
        Ok(WorkflowToolResult {
            result: result.clone(),
            json: Some(result),
            text: None,
            files: vec![],
        })
    }
}
