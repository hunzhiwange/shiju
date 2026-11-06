# OpenCode 完全访问接入

日期：2026-11-06。依赖批次 11 提交 `8facc9d46` 已存在；Kiro 接入已落地。

## 行为与范围

分类为客户端自动批准，不宣称原生免审批或工具实机验证通过。
用户明确要求跳过测试、账号验收和 E3 完成门槛；未新增或执行行为测试，
未请求账号、密码、BYOK、受控上游，未安装或运行第三方 CLI。
本机 PATH 未找到 opencode；版本 1.19.5 为既有基线及本次阅读的上游 tag，
不是本机执行版本，未取得发行二进制哈希或真实协议报文。

`access_policy::opencode::resolve_permission_request` 复用现有审批决策器，
先移除 `allow_always`。OpenCode 1.19.5 提供 `once`/`always`/`reject`，
当前 ApproveAll 自动选 `allow_once`，ApproveReads/非交互策略/拒绝保留既有语义。
没有单次允许项时取消或拒绝，不回退到永久允许。
取消中的请求仍先取消，统计仍按原始请求和真实选择记录。
现有两个事件客户端测试 fixture 仅补齐新增字段，不新增或运行测试。

不开启 `permission="allow"`，不注入 `--auto`，不改用户全局配置或模型配置。
全局配置替换会扩大原生权限且可能覆盖项目 deny；ACP 的 mode 是 agent 选择，
不是权限开关，不能通过切换 build/plan 改变权限意图。
`AcpClient::access_policy` 对 OpenCode 排除无关 config/modes，
完整授权报告 ClientOnly/NoAdvertisedControls，读取授权报告 Partial，
关闭和 DenyAll 保留共享策略结果。DenyAll 在 prompt 入口拒绝原生执行，
防止未触发审批的原生工具绕过宿主拒绝。

普通/托管、cli_local/session_model 均复用同一客户端回调与新建/加载/恢复路径。
开启、关闭及恢复仍由当前 fullAccess/permissionMode 决定，不保存额外后端状态。
`acp_client_cache_key` 包含两者，`stop_incompatible_cached_clients` 在同一
runtime_identity 下关闭旧进程，再按当前授权恢复原会话。
逐次批准避免本次接入写入 OpenCode 会话规则；不承诺撤销用户自己配置的 allow、
旧版本先前写入的 always 或第三方插件权限。关闭完全访问不等于强制所有工具 ask，
正常模式继续沿用现有 permissionMode 和原生规则。原生 deny/企业策略不被绕过。

## 官方来源（源码确认 E2；没有协议实测 E3）

- [官方权限文档](https://opencode.ai/docs/permissions/)：permission allow/ask/deny
  属配置能力；auto 不是已确认的 ACP 会话权限控制。
- [v1.19.5 ACP service](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/acp/service.ts)：
  setSessionConfigOption 只处理 model/effort/mode；availableModes 由 agent.name 生成；
  prompt 将 modeId 作为 agent 传入。
  SHA-256：`238750d52e220080db7b7f4992432fbfc7fb7ec5a15a6175460f8f0832a2c8de`。
- [v1.19.5 ACP permission](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/acp/permission.ts)：
  permission.asked 转为 requestPermission，提供 once/always/reject；选择映射为
  permission.reply；取消和非允许结果映射 reject。
  SHA-256：`b83f1b30015f92f432e56a444a7304e131b1824b06870e7842c70db6bd969743`。

源码来自官方固定 tag 的 raw 文件，本次仅在临时目录阅读，不纳入仓库。
未采集 initialize/new/load/resume/config/mode 报文、审批计数、普通文件/终端结果；
未验证 false→true→false、恢复和不可覆盖 deny 的实际执行效果，未外推平台支持。

## 静态检查

动态领取槽位 2；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。
- `cargo fmt --all`：退出码 0，仅保留本任务必要格式化差异。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出码 0，22.91s。
- `git diff --check -- crates/vw-acp docs/shiju/acp-full-access`：退出码 0。
- 未执行测试，有效测试数 0（用户明确跳过），未声称红绿测试或实机通过。
- Cargo 子进程结束后已释放槽位 2 的 owner 和锁，保留缓存。

审查范围为 OpenCode 审批选项、能力分类、宿主拒绝及本证据/矩阵；
不纳入其他工作区变更，plan.md 保持只读。
