# Cursor 完全访问接入

日期：2026-11-06；批次 20 依赖提交 `620de1f46` 已存在，Snow 接入及公共逐次批准机制可复用。
用户最新要求覆盖测试与账号验收计划：不新增或运行测试，不申请账号、密码、BYOK 或受控上游；实机未测不阻塞正常访问。

## 官方契约与证据范围

- [官方 ACP 文档](https://cursor.com/docs/cli/acp)：stdio JSON-RPC；new/load 会话；工具审批提供 allow-once、allow-always、reject-once。agent/plan/ask 为工作模式，不作为免审批开关。
- [官方权限文档](https://cursor.com/docs/cli/reference/permissions)、[参数文档](https://cursor.com/docs/cli/reference/parameters)：force/yolo 为 CLI 授权参数；deny 优先。没有确认可回读、可撤销的 ACP 会话权限配置，不杜撰 config ID 或自动注入启动参数。
- 问答和计划确认使用独立 Cursor 扩展方法。现有客户端返回 method-not-found，不会当作工具审批自动回答；相关交互能力不属于本任务实现范围。
- 版本、入口与历史发布元数据沿用 [静态清单](README.md#cursor)，没有执行安装、升级、登录或第三方 CLI，没有当前制品哈希、权限实现源码 E2 或协议 E3。官方文档不能推出基线版本实际执行效果。

## 生产接入（E1）

- `AcpClient::build_event_client` 为 Cursor 启用 transient_permissions，复用现有 `access_policy::opencode::resolve_permission_request`；移除 AllowAlways，只按当前 permissionMode 逐次决定，缺少 AllowOnce 时拒绝或取消。关闭或恢复不遗留由本次客户端自动批准创建的永久授权。
- `AcpClient::access_policy` 将 Cursor 归类为 ClientOnly；不会因为公布工作模式或任意 configOptions 声称原生免审批。full_access=false、ApproveReads、DenyAll 沿用共享明确分类。
- `access_policy::cursor::validate_launch` 在缺少当前 full_access + ApproveAll 时拒绝显式 -f/--force/--yolo。已授权自定义参数保持原样；默认入口不注入 force，逐次审批可随当前授权改变。
- `run_actor_prompt_with_controls` 在 DenyAll 下拒绝 prompt，避免代理自身工具绕开客户端 deny。取消请求沿用取消处理；文件和终端使用既有处理器。
- 开关复用应用已有客户端缓存身份及进程停止路径；同会话恢复使用当前策略、不改变逻辑 sessionId。已经执行的工具、用户已有 allowlist 与供应商权限不承诺撤回；显式 force 参数需在关闭时移除，否则明确报错。
- 普通/托管 cursor-agent acp、新建/加载/恢复保持既有协商路径；session_model 第三方 Provider 门控保持原义。不写用户全局权限，不改认证、模型、安装及模式。

## 静态检查

自动化测试按用户要求跳过，有效测试数 0；无真实 initialize/new/load/resume/config/mode、审批计数或工具执行报文。
开启→关闭→恢复、MCP、deny 与跨平台效果均未实机验收，不宣称原生支持。

- 动态槽位 2：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。
- `cargo fmt --all`：退出码 0；只保留本任务必要格式差异。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出码 0，19.78s。
- `git diff --check`（限定本任务代码和文档）：退出码 0。
- 槽位子进程已结束，自己的 owner 记录及锁已清理；缓存保留。
- 差异审查：仅本后端逐次审批、能力分类、host deny、启动授权检查及说明；不提交其他任务改动。
