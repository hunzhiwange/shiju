# Droid 完全访问接入

日期：2026-11-06；依赖批次 21 提交 `0b9eb1754` 已存在。
用户最新要求覆盖原计划的 TDD、自动化测试、账号实机验收与 E3 完成门槛；本次不运行测试或请求凭据。

## 官方能力与版本边界

- [官方 IDE 集成](https://docs.factory.com/ide-integrations) 公布 `droid exec --output-format acp`，与仓库普通、本地及托管入口一致。
- [官方 Exec 文档](https://docs.factory.com/droid-exec/overview) 公布 `--auto low|medium|high` 与 `--skip-permissions-unsafe`。普通 Exec 默认只读不能据此推出 ACP 的默认模式或精确 mode/config ID。
- [官方自治文档](https://docs.factory.com/autonomy-and-safety/auto-run) 将自治等级与交互模式分开；High 仍受 ask、block、安全检查与沙箱审批约束。始终允许可能提高自治等级或保存沙箱路径/域名授权，因此客户端不自动选择 AllowAlways。skip-permissions-unsafe 跳过提示，但有效 command block 仍生效。
- 文档是当前公开契约，不是仓库基线 1.3.0 或此前发布查询 1.5.0 的固定制品源码证明。版本及已有制品哈希入口见 [静态基线](README.md#droid)。未安装、升级或执行 Droid，未取得本次运行制品哈希、原生权限 E2 或协议 E3。

## 生产接入（E1）

- `AcpClient::build_event_client` 对 Droid 启用 `transient_permissions`，复用 `access_policy::opencode::resolve_permission_request`：先排除 AllowAlways，再按当前 permissionMode 决策。ApproveAll 仅选择 AllowOnce；缺少一次性允许项时拒绝或取消，不伪造选项。
- `AcpClient::access_policy` 将 Droid 显式归为客户端批准；任意 modes/configOptions 不使其升级为原生支持。关闭 full_access 为 NotRequested，ApproveReads 为 Partial，DenyAll 为 DeniedByHost。
- `run_actor_prompt_with_controls` 在当前 DenyAll 时阻止 prompt，避免原生工具绕过客户端回调。文件、终端及取消中的审批仍沿用现有权限处理器。
- `access_policy::droid::validate_launch` 在启动材料化点检查显式 `--skip-permissions-unsafe`（含等号形式，停止于参数分隔符）。仅当前 full_access 且 ApproveAll 可启动；关闭授权后必须移除此显式参数，否则明确拒绝。正常 `exec --output-format acp` 不受此校验阻挡。
- 不自动注入 unsafe、auto、权限配置或自治 mode；既有用户自治参数保持原义，不把分级自治宣称为完全免审批。不写全局配置，不改模型、安装或认证。
- 开启/关闭使用现有权限缓存身份变化及旧进程停止机制；恢复使用当前客户端授权，不新增永久供应商授权。已开始的工具或用户自行保存的权限不声称被瞬时撤销。new/load/resume 继续现有协商路径，不新增协议字段。
- session_model 的双向 ACP/隔离根门控是既有模型来源限制，本任务未扩大或新增禁用。普通 CLI 本地访问保持现有行为。
- 标准工具审批与计划/问答独立；没有真实报文，不宣称 Droid 计划确认映射或工具执行已验收。

## 验证与限制

按用户要求未新增或运行测试，有效测试数 0；未启动第三方 CLI。
未采集 initialize/new/load/resume/config/mode 报文、工具执行或审批计数；false→true→false、恢复、供应商 deny、普通/托管及跨平台工具效果未实机验收。
静态检查不能替代行为证据，完成结果仅指本次代码接入。

完成检查：

- 动态领取槽位 2；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。
- `cargo fmt --all`：退出码 0，无额外 features，仅保留本任务必要格式差异。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出码 0，18.90s。
- `git diff --check -- crates/vw-acp docs/shiju/acp-full-access`：退出码 0。
- 自动化测试及供应商实机验收按用户要求跳过，有效测试数 0。
- Cargo 进程结束后已清理自己的 owner 及槽位锁，构建缓存保留。
- 审查差异仅为 Droid 逐次批准、客户端能力分类、host deny 屏障、显式启动 bypass 授权校验和本后端文档；其他工作区改动不纳入提交。
