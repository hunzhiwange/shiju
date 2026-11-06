# Kiro 完全访问接入

日期：2026-11-06。批次 10 依赖提交 `0d224984a` 已存在。

## 实现与边界

分类为部分支持，不宣称已验证原生免审批。用户要求跳过测试和账号实机验收，
本任务未新增或运行测试，未请求账号、密码、BYOK 或受控上游；有效测试数 0。

`access_policy::kiro::session_policy` 在 `full_access && ApproveAll` 时生成
`{"policyPreset":["allow-all"]}`，否则生成 `{"policyPreset":[]}`。
`mounted_session_meta` 将其放入 `_meta.kiro`，保留现有技能及模型元数据。
`new_session_info` 与 `load_session_info` 均发送当前意图；
`resume_session_info` 对 Kiro 使用同一 sessionId 的 load，不新建逻辑会话。
不添加权限 config 或切换整个 agent，不写全局权限或工作区信任。

`DenyAll` 在 prompt 入口拒绝原生执行；进程级 `--trust-all-tools`、
`--trust-tools`（包括等号形式）明确拒绝，避免脱离会话授权的放行。
普通与托管入口、cli_local/session_model 均复用 ACP 客户端路径。
现有 session_model 驱动选择 V3，普通入口不强制替换用户 harness。
旧 harness 若忽略扩展，仍使用现有客户端审批；未知预设的上游错误正常返回。

权限开启和关闭继续使用现有应用生命周期：`acp_client_cache_key` 包含 fullAccess
和 permissionMode，`stop_incompatible_cached_clients` 关闭同一 runtime_identity
的旧进程，之后用当前授权恢复同一会话。预设按官方文档仅驻留内存；关闭时不主动
申请 allow-all，并发送空列表。调用方须先停止旧活动 turn/进程再替换客户端，
不承诺同进程热撤销或撤销用户自己保存的权限规则。该生命周期和空列表在供应商中的
实际撤销效果未实机测试；不把请求发送当成权限回读确认。

## 来源与未验证项

- [官方权限说明](https://kiro.dev/docs/permissions/)：公布 new/load 的
  `_meta.kiro.policyPreset` 数组。allow-all 不覆盖 sandbox_network，
  deny/ask 的优先级仍高于 allow，敏感路径依然可能询问。
- [官方 ACP V3 迁移](https://kiro.dev/docs/cli/v3/acp-migration/)：
  V3 拒绝旧 trust 工具参数，模式代表 agent 配置，不是通用权限开关。
- [官方 ACP 入口](https://kiro.dev/docs/cli/acp/)：普通入口为 `kiro-cli acp`。

以上是官方文档与仓库源码静态确认（E1），不是固定发行源码 E2 或工具实测 E3。
本机 PATH 未找到 kiro-cli；按用户要求未安装、运行或登录第三方 CLI。
未取得二进制版本/哈希、initialize/new/load/config/mode 报文、审批计数、
文件/终端结果和跨平台验收。矩阵中的 3.0.0/3.1.0 沿用既有版本基线，
不代表本任务执行版本。底层用户、工作区、企业 deny 与沙箱仍由 Kiro 执行。

## 静态检查

动态领取槽位 2；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。
- `cargo fmt --all`：退出码 0。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出码 0，22.02s。
- `git diff --check -- crates/vw-acp docs/shiju/acp-full-access`：退出码 0。
- 未运行测试，有效测试数 0（用户明确跳过）；未声称红绿测试或实机通过。
- Cargo 子进程结束后已释放槽位 2 的 owner 和锁目录，保留缓存。

审查仅涉及 Kiro 权限构造、会话 meta、恢复、启动保护、能力分类及本证据/矩阵；
其他工作区变更不纳入提交。
