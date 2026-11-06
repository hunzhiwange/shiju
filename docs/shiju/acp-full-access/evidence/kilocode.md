# Kilocode 完全访问接入

日期：2026-11-06；依赖批次 22 提交 `b6f923b9d` 已存在。
用户最新要求覆盖原计划的 TDD、自动化测试、账号实机验收与 E3 完成门槛；本次不运行测试或请求凭据。

## 固定源码与入口边界

仓库基线 `@kilocode/cli` 7.8.18，发布查询 7.8.27；版本、发行 SRI 和托管制品哈希入口见 [静态基线](README.md#kilocode)。本次未安装、升级或启动 CLI，未取得本次运行制品哈希。

核对官方 `v7.8.18` 源码（E2，不等于实机协议或工具效果）：

- [ACP 权限处理](https://github.com/Kilo-Org/kilocode/blob/v7.8.18/packages/opencode/src/acp/permission.ts) 公布 `once/allow_once`、`always/allow_always`、`reject/reject_once`；取消映射 reject，允许后向原生 permission.reply 发送 once 或 always。
- [ACP service](https://github.com/Kilo-Org/kilocode/blob/v7.8.18/packages/opencode/src/acp/service.ts) 实现 new/load/resume，config 仅处理 model、effort、mode；mode 切换 agent，不是单独的免审批开关。[directory](https://github.com/Kilo-Org/kilocode/blob/v7.8.18/packages/opencode/src/acp/directory.ts) 从 agent.list 构造 availableModes。不自动切换 agent，避免改变模型、工具和提示词。
- [CLI 入口](https://github.com/Kilo-Org/kilocode/blob/v7.8.18/packages/opencode/src/index.ts) 的 `--pure` 设置 `KILO_PURE=1`，描述为禁用外部插件。[配置加载](https://github.com/Kilo-Org/kilocode/blob/v7.8.18/packages/opencode/src/config/config.ts) 仍读取 KILO_CONFIG、项目配置及 KILO_CONFIG_CONTENT，管理员配置最后加载。不能把 pure 当作忽略配置或放开权限。
- [官方权限文档](https://kilo.ai/docs/customize/agent-permissions) 区分 allow、ask、deny，并按规则顺序处理。不注入通配 allow 或 `--auto`，保留原生规则与硬拒绝。CLI 普通运行参数不推断为 ACP 会话权限字段。

普通默认 `npx -y @kilocode/cli acp`、本地 `kilocode acp`、托管 `kilocode acp --pure` 保持原样；session_model 沿用现有私有模型配置与 pure 入口，不修改模型来源、认证或安装体系。

本次读取的固定源码 SHA256：

- `permission.ts`：`6e62347fc925b2f8bf1cdb6282c659bfedc409fe093d495383acbc59cb2e7519`。
- `service.ts`：`eb1bb08744490260245281ad7aef20a7a7e260844c1dc4251d9e06e61188150f`。
- `directory.ts`：`c4f0f38ef433713afdd50f89d7ecf2ce8130f5104dc7606a2ff3f090726109b4`。
- `index.ts`：`1a1243e2a71a63b09f82de67f957729118398060d41e91aa57ae052e876863a4`。
- `config.ts`：`3126e79f4bab9a46383e03ce57233558335c0cb160c9feadda5adf40d22599e6`。

## 生产接入（E1）

- `AcpClient::build_event_client` 对 kilocode 启用 transient_permissions，复用 `access_policy::opencode::resolve_permission_request`，排除 AllowAlways，再依当前 permissionMode 选择。ApproveAll 只批准 AllowOnce；无一次性允许项时拒绝或取消。
- `AcpClient::access_policy` 将 Kilocode 明确归为客户端自动批准，不因公布 modes/configOptions 升级为原生支持。full_access 关闭报告 NotRequested，ApproveReads 为 Partial，DenyAll 为 DeniedByHost。
- `run_actor_prompt_with_controls` 在当前 DenyAll 时拦截 prompt，避免原生工具不回调客户端时绕过 host deny。文件、终端及取消审批仍沿用现有处理器。
- 开启按当前授权批准每次回调；关闭与恢复依现有权限缓存身份变化、旧进程停止和当前客户端授权，不向供应商写入新的永久批准。已开始的工具、用户原有的 allow 规则不声称瞬时撤销。
- new/load/resume 沿现有路径，不杜撰配置值，不阻断正常访问；session_model 既有隔离与版本门控保持。客户端批准不等于解除原生沙箱，也不覆盖供应商 deny。问答、计划与 agent 切换不因完全访问自动选择。
- 共享解析器已有实际生产调用，本任务无需新增重复后端模块或未来配置接口。

## 验证与限制

按用户要求未新增或运行测试，有效测试数 0；未启动第三方 CLI。
未采集 initialize/new/load/resume/config/mode 报文、工具结果或审批计数；false→true→false、恢复、deny、普通/托管及跨平台权限效果未实机验收。既有模型来源实机证据不外推为本次权限验收。
静态检查不能替代行为证据，完成结果仅指本次代码接入。

完成检查：

- 动态领取槽位 2；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，`CARGO_BUILD_BUILD_DIR` 同一绝对路径。
- `cargo fmt --all`：退出码 0，未增加 features，仅保留本任务必要格式差异。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出码 0，2m 19s。
- `git diff --check -- crates/vw-acp docs/shiju/acp-full-access`：退出码 0。
- 按用户要求跳过自动化测试与供应商实机验收，有效测试数 0。
- Cargo 进程结束后已清理自有 owner 和槽位锁，保留编译缓存。
- 审查仅提交本后端三处生产连线与独立证据/矩阵；工作区其他任务变更不纳入提交。
