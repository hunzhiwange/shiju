# Auggie 完全访问接入

日期：2026-11-06；批次 16 依赖提交 `1f977d37c` 已存在。
用户明确要求跳过 TDD、自动化测试与账号实机验收，不申请凭据、BYOK 或受控上游。

## 官方能力与边界

- [官方 ACP 文档](https://docs.augmentcode.com/cli/acp/agent)：以 `auggie --acp` 启动，通过 stdio JSON-RPC 通信；不保证交互模式全部能力均支持 ACP。
- [官方权限文档](https://docs.augmentcode.com/cli/permissions)：CLI `--permission` 为独立策略；跨策略以最严格规则优先，allow 不能覆盖 deny、外部策略或 ask-user。
- [官方 CLI 参考](https://docs.augmentcode.com/cli/reference)：公布可重复的 `--permission bash:ask-user`；当前文档说明恢复会话会恢复 permission 和 add-workspace，除非重新指定。这不是 ACP load/resume 已验收的声明。
- 当前文档包含 1.x 内容，与仓库 0.36.0 基线及此前查询的 0.37.0 不等价；没有执行安装、升级或 CLI，也没有取得当前运行制品哈希、原生权限 E2 或协议 E3。版本清单见 [静态基线](README.md#auggie)。

没有取得可安全回读及撤销的 ACP 原生权限契约，因此本次不杜撰 yolo、permission mode、全工具通配符或配置 ID，不自动写入 permission 规则或全局配置。

## 生产接入（E1）

- `AcpClient::build_event_client` 为 Auggie 启用已有 transient_permissions，复用 `access_policy::opencode::resolve_permission_request`。先排除 AllowAlways，再依据当前 permissionMode 决策；ApproveAll 仅选择 AllowOnce。缺少一次性允许项时拒绝或取消，不伪造供应商选项。
- `AcpClient::access_policy` 显式归类为客户端批准，任意 modes/configOptions 不升级为原生支持。关闭 full_access 为 NotRequested；ApproveReads 仍为 Partial，DenyAll 为 DeniedByHost。
- `run_actor_prompt_with_controls` 在 DenyAll 下阻止 prompt，避免供应商自身工具绕过客户端回调。取消中的会话继续取消审批；文件与终端沿用既有当前权限处理器。
- 开启/关闭沿用应用权限缓存身份变化与旧进程停止路径；同会话恢复使用当前客户端策略，不自动选择永久允许，不改逻辑 sessionId。已开始的工具与供应商自行保存的权限不声称瞬时撤销。
- 不新建没有调用者的后端模块。普通 npx/本地 `auggie --acp` 保持可访问；现有 new/load/resume 能力协商、session_model 的 Provider 注册限制、认证、模型、安装和平台门控保持原义。
- 本次只处理标准工具审批，不新增问答或计划回答路径。没有真实报文可证明 Auggie 的问答映射，不能外推为已验收。

## 验证

按用户要求未新增或运行测试，有效测试数 0；未启动第三方 CLI。
没有 initialize/new/load/resume/config/mode 报文、工具执行或审批计数；不声称 false→true→false、同会话恢复、企业 deny 或跨平台已实机通过。
静态检查结果将在完成后补充；静态编译不能证明供应商实际执行效果。

完成检查：

- 动态领取槽位 2；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。
- `cargo fmt --all`：退出码 0，无额外 features，仅保留本任务必要差异。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出码 0，19.35s。
- `git diff --check -- crates/vw-acp docs/shiju/acp-full-access`：退出码 0。
- 自动化测试/供应商实机验收按用户要求跳过，有效测试数 0；静态编译不替代工具行为证据。
- 子进程已结束，自己的 owner 记录及槽位锁已清理，构建缓存保留。
- 审查差异仅为复用逐次批准、客户端能力分类、host deny 屏障与本后端文档；其他工作区改动不纳入提交。
