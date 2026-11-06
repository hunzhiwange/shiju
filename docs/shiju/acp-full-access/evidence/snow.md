# Snow ACP 权限接入

日期：2026-11-06。批次 19 依赖提交 `2aa6649a5` 已存在。
用户明确跳过测试驱动、自动化测试及账号实机验收；本次没有启动 Snow、申请凭据或连接模型。

## 来源与身份（E2）

- 注册表普通入口 `snow --acp`；托管契约 `snow-ai@0.9.14`，入口 `bundle/cli.mjs`。
- [npm 固定版本元数据](https://registry.npmjs.org/snow-ai/0.9.14) 指向
  [MayDay-wpf/snow-cli](https://github.com/MayDay-wpf/snow-cli)，不是按名称推断其他 Snow 产品。
- 下载官方 [0.9.14 发行包](https://registry.npmjs.org/snow-ai/-/snow-ai-0.9.14.tgz)，仅解包读取，未安装、执行或运行 postinstall。
  原始 `bundle/cli.mjs` SHA256 为 `bcc0a6cf468a245729605bcf75fd762fd13cb48553d342b31121cafc3153d905`，
  与仓库 `patches/acp-model-source/snow/patch.json::original` 一致。
  托管契约的 `dea7a6b9b28490a78c7c0f95d294b5c5203c7aa8f4a9f7cedc97720bbdd9d25e`
  是既有技能隔离补丁后的哈希，不能误判成官方原始制品。本任务不更改制品、版本或补丁。
- 官方 tag `v0.9.14` 指向 `a9eba5f9062afbf639829b4992e90a986924f519`；
  [acpManager.ts](https://github.com/MayDay-wpf/snow-cli/blob/a9eba5f9062afbf639829b4992e90a986924f519/source/utils/acp/acpManager.ts)
  SHA256 `8ff077c98a92e6bf00da245b054ad2a51e27338bf2d342fcd87174164e710a8e`。
  与发行包内 `dist/utils/acp/acpManager.js` 的权限逻辑对应。

源码实现 initialize、new/load、prompt、cancel，没有 session/set_mode、权限 config 或 resume handler。
new 仅返回 sessionId，load 返回空对象；initialize 声明 loadSession。
普通工具回调提供 `approve_once`/`allow_once`、`approve_always`/`allow_always`、
`reject_once`/`reject_once`；敏感终端命令不提供永久允许项。
选择 approve_always 会调用 `addToolToPermissions` 写入工作区持久权限。
已有 alwaysApprovedTools、todo/subagent/askuser/tool_search 的内部自动批准以及 toolConfirmation hooks
可在客户端回调之前生效。CLI 交互界面的 YOLO 参数不等同于 ACP 原生权限接口，不注入该参数。

## 生产行为（E1）

分类：**客户端自动批准**，不宣称原生免审批或工具实测通过。

- `build_event_client` 将 Snow 纳入现有 transient_permissions：过滤 AllowAlways，
  当前 host permissionMode 通过公共 resolver 处理逐次批准/拒绝，不为本次授权写入持久规则。
- `AcpClient::access_policy` 对 Snow 保持 ClientOnly；DenyAll 优先为 Unavailable，
  ApproveReads 为 Partial；未知协议选项不会升级为 Native。
- `run_actor_prompt_with_controls` 在 Snow 的 host DenyAll 下拒绝原生 prompt，
  覆盖 Snow 内部自动批准和 hook 可能不发客户端回调的路径。
- 普通/托管入口与 cli_local/session_model 复用相同客户端；私有模型配置、版本门控与认证不变。
  客户端文件和终端沿用既有处理器；供应商 MCP 工具主要在 Snow 进程执行，不宣称由客户端文件边界限制。
- false→true→false 按当前 permissionMode 处理每个回调。权限变化沿用
  `acp_client_cache_key` 的 fullAccess/permissionMode 维度和
  `stop_incompatible_cached_clients` 的 runtime_identity 关闭机制；恢复沿用同 sessionId load 回退。
  必须先停止旧 turn/客户端再使用新授权；不承诺同进程热撤销。
  不撤销用户自己保存的批准和 hook 配置，不把关闭完全访问描述成撤销全部第三方权限。
- 本任务只复用已有生产路径，无需新建后端模块或未调用的抽象；没有新增测试文件。

## 验证与限制

按用户要求未运行测试；有效测试数 0，没有红绿测试证据。
以上字段与行为来自源码阅读，不是真实 initialize/new/load/resume/config/mode 协议抓取；
审批计数、文件/终端执行、敏感命令、恢复和跨平台效果均未实机验收。
版本 0.9.15 沿用矩阵先前查询，不代表本次执行版本。

静态检查使用动态领取的槽位 **2**，
`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。

- `cargo fmt --all`：退出码 0；仅保留本任务必要差异。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出码 0，21.46s；未执行测试。
- `git diff --check -- crates/vw-acp docs/shiju/acp-full-access`：退出码 0。
- 验证进程已全部结束，owner 和槽位锁已释放，编译缓存保留。

审查仅包含 Snow 客户端批准、host deny、能力分类和本证据/支持矩阵。
未修改只读 plan.md，其他任务及工作区改动不纳入提交。
