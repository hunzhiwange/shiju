# 脱敏账号与流程 DTO

母任务 6／子任务 4。按[端点契约](接口契约.md)在 `vw_api_types::agent_apps_auth` 定义共享类型。

## 开工边界

批次 1–3 由 `cd0dfb217`、`14acbba54`、`c7d8474f1` 交付。固定来源提交的授权 Rust 命令和 TS API 与仓内快照逐字节匹配。核对了已注册的 `vw-agent-apps` 原生包、主机内 `AuthAccountStore`、认证与主机合同、存储适配和供应商事务。

继续执行时以已验收的底座交付作为本批开工条件。母任务 2 的完整验收本身依赖授权迁移，母任务 4 另有实机验收缺项。本批只消费固定协议与现有共享类型。网关装配、主机策略、运行时流程及实机交互各由后续批次验证。[底座交付映射](../底座交付映射.md) · [供应商验收](../../../shiju/agent-apps/provider-macos-acceptance.md)

## 协议与脱敏

- `AuthAccount`、`AuthStatus` 和 `AuthCredentialState` 只表达公开元数据、能力和凭据状态。时间字段复用 UTC Unix 毫秒类型 `TimestampMs`。
- `AuthStartResponse` 单独发放授权 URL、设备用户码或 Claude 回填能力。轮询 `AuthFlow` 按状态限定负载。待定含重试间隔，批准含脱敏账号，失败含稳定码、安全说明和关联 ID。
- 服务和登录方式使用闭合枚举。未知值反序列化失败。已知但不支持的组合返回 `unsupported`，不适用的选项与缺地区的 Kimi 新登录返回 `invalid_input`。域策略、重登目标与地区校验由执行主机负责。
- 请求与响应拒绝未知字段。访问令牌、刷新令牌、ID token、上游设备码、兑换授权码和 PKCE verifier 都没有公开输出字段。启动 URL、用户码、不透明流程 ID 与主动回填值在 `Debug` 中遮盖。网关仍须遮盖原始请求与解析错误，并在生成响应前投影安全错误。

## 验证

共享槽位 1 的 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1` 用于红灯。槽位 2 的 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2` 用于绿灯及 Clippy。仓库另设 `build.build-dir`，首次红灯的测试可执行文件落在公共 `target/debug`，因此将 `CARGO_BUILD_BUILD_DIR` 同步指向领取的槽位并复跑。下表红灯只计合规复跑。格式化在源码快照运行，只回写本批 Rust 文件。

| 命令 | 退出码 | 结果 |
| --- | ---: | --- |
| `cargo fmt --all` | 0 | 红灯与绿灯源码均格式化 |
| `cargo test -p vw-api-types --lib auth6_dto -- --nocapture`（红灯） | 101 | 0 通过，2 失败，127 过滤 |
| 同命令（绿灯） | 0 | 2 通过，0 失败，127 过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区通过 |
| `git diff --check`、本任务暂存差异检查 | 0 | 无空白错误 |

红灯中 `auth6_dto_commits_expected_scoped_result` 在调试脱敏断言失败。 `auth6_dto_rejects_invalid_or_stale_input` 实际接受 xAI 浏览器登录，断言要求 `Err(Unsupported)`。绿灯验证启动和轮询的 JSON 字段、公开账号与错误形状、调试脱敏、禁止凭据字段、服务方式矩阵及过期状态拒绝待定字段。

验证结束后释放本任务的两处槽位锁，保留构建缓存。
