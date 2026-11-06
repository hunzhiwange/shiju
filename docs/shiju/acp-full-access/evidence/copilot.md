# Copilot 完全访问接入

日期：2026-11-06；批次 14 依赖提交 `9f16fa0db` 已存在。
按用户最新授权跳过 TDD、自动化测试和账号实机验收，有效测试数 0，不申请账号或 BYOK。

## 官方资料与制品

- [官方 ACP 文档](https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server)：`copilot --acp` 默认 stdio，new/load 继承服务器初始配置；未取得可回读的会话权限切换契约。
- [官方 CLI 文档](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference)：allow-all 系列控制工具、路径、URL，deny 和企业限制仍有效。这不能替代 ACP 继承及执行验收。
- [官方权限说明](https://docs.github.com/en/copilot/how-tos/copilot-cli/use-copilot-cli/allowing-tools)：会话启动参数和权限文件生命周期不同；本次不写权限文件。
- 核对 npm `@github/copilot@1.0.106` package.json、npm-loader.js，buildMetadata.gitCommit 为 `2bfb456b9e0`；loader 执行对应平台原生包。[上游仓库](https://github.com/github/copilot-cli)用于确认发行来源，不当作实现证据。
- 主包 SHA256：`0005981f62de0ebcc65bee7f5421cb6ffb1202dbe69254d328bbfe6d608d5f2c`。
- 下载并静态检查 `@github/copilot-darwin-arm64@1.0.106`；tar SHA256：`17cf3b6a0cfc81b1483813f83056f3d7443f205ac4fb920b1bc5c3ab1b061fa7`。制品为原生二进制，未执行；字符串检查未取得 ACP 权限实现，不计权限 E2。

## 生产接入（E1）

普通/托管 `copilot --acp`、cli_local/session_model 继续使用正常连接与既有门控。
不因未实机验收禁用后端，不改模型、认证、安装及平台配置。

`build_event_client` 为 Copilot 启用 transient_permissions，复用已有逐次审批解析函数，
排除 AllowAlways 后按当前 permissionMode 决策。ApproveAll 选择 AllowOnce；只有永久允许项
或没有允许项时拒绝/取消，不伪造一次性授权。文件、终端与取消检查沿用共享处理器。
DenyAll 在 prompt 前拒绝原生执行。关闭沿用应用 fullAccess/permissionMode 缓存身份和
旧进程停止机制，恢复原 sessionId 后使用当前客户端策略，不留下客户端永久允许选择。

`access_policy::copilot::validate_launch` 校验显式 --allow-all、--allow-all-tools、
--allow-all-paths、--allow-all-urls、--yolo（含等号形式）；只有当前 full_access && ApproveAll
才允许启动。授权参数保留原义，关闭后必须移除，否则明确报错。
进程环境移除 COPILOT_ALLOW_ALL，防止继承环境或私有配置的 blanket grant 越权；
不注入全放行参数、不写全局配置、不解除原生沙箱。
能力标记 ClientOnly / NoAdvertisedControls；不把任意 modes/configOptions 当作权限证据。

显式启动参数的 ACP 生效范围仍未测；不声称已证明 tools/paths/URLs、企业 deny 或历史权限撤销。
上游独立保存的原生授权不在客户端 allow_once 控制范围内，正在执行的工具也不承诺瞬时撤销。
未发送权限 slash 命令，避免未公布命令被当成模型 prompt。

## 验证边界

未运行第三方 CLI、自动化测试或实机验收。没有 initialize/new/load/resume/config/mode
真实报文、审批计数或工具执行证据；有效测试数 0，features 不适用（用户明确跳过）。
建议后续补 false→true→false、同会话恢复、tools/paths/URLs、deny 和其他平台验收。
静态编译不能证明实际权限效果。审查范围仅为当前授权校验、逐次批准、能力分类与本证据。
既有其他工作区变更不纳入提交。测试辅助结构仅同步字段改名，没有新增或运行测试。
初次 clippy 退出码 101：新增模块错误导入 `errors::AcpError`，修为公共导出 `crate::AcpError` 后重跑。


最终静态验证：

- 动态领取槽位 2，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`；两次检查阶段均使用该槽位。
- `cargo fmt --all`：退出码 0，仅保留本任务格式差异。
- `cargo clippy --all-targets --all-features -- -D warnings`：最终退出码 0，19.34s。
- `git diff --check -- crates/vw-acp docs/shiju/acp-full-access`：退出码 0。
- 未运行测试；有效测试数 0。未宣称供应商原生权限实机通过。
- Cargo 子进程全部结束，已删除自己的 owner 和槽位锁，编译缓存保留。
