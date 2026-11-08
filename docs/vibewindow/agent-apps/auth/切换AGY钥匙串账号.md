# 切换 AGY 钥匙串账号

母任务 6／批次 29。前置批次 1–28 已按交付提交核对。直接前置是[完成 AGY 浏览器授权](完成AGY浏览器授权.md)（`57f9c55ee`），复用其 `shiju.agy.oauth` 托管记录及作用域账号 ID。来源为 cc-switch 4.0.25（`ca58c0313360eae7c851fbd7553c1614c7af03e7`）的 `services/agy_account/{keychain,mod,tests}.rs` 和 `commands/agy_account.rs`。

`vw_agent_apps::auth::agy_account::switch_default_account` 在 macOS 使用执行用户的 `gemini`／`antigravity` 项。调用方须从受信策略解析执行用户的 HOME、独占槽所有者及作用域账号。主机侧复核作用域、HOME、账号身份与凭据状态。其他平台的切换返回 `unsupported`。

原生项用 `go-keyring-base64:` 加 Base64 编码的 JSON。沿用主机钥匙串适配器，在定位到的钥匙串检查旧值、原地更新并回读。凭据仅经 `security -i` 的标准输入传递，完整交互行超过 4032 字节时拒绝。钥匙串目录上的进程间锁协调 ShiJu 的浏览器授权提交和切换。当前账号和授权一致时直接返回。切走前仅收录可绑定到本作用域且刷新令牌一致的较新凭据，保留原生 JSON 的未知字段。无法确认旧登录归属、同一身份持有不同授权或凭据代际冲突时，保留原项并返回 `conflict`。

`agy_account_is_running` 在 Unix 以进程可执行文件名识别 `agy`，供调用者提示旧进程可能写回。外部 AGY 进程可以在切换后继续使用旧授权。主动刷新与旧进程写回处理归批次 30，默认账号 HTTP／SDK 接线归批次 43。

## 验证

macOS 27.0（26A428），arm64。测试通过系统钥匙串创建、覆盖、回读及删除唯一命名的测试项，测试进程使用同名的本地短命进程验证运行风险。覆盖切回原账号、收录同一授权的较新代际、当前账号幂等、独占域和 HOME 拒绝、需重登、错绑、不同授权、原生项冲突和交互行超长。真实 AGY 账号验收归批次 62。

红灯时，授权成功用例预期 `Ok(())`，独占域缺失用例预期 `Forbidden`。两个断言实际均收到 `Unsupported`。

| 阶段 | 槽位 | CARGO_TARGET_DIR |
| --- | ---: | --- |
| 红灯、初轮绿灯 | 2 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-2` |
| 最终验证 | 1 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-1` |

`CARGO_BUILD_BUILD_DIR` 与各阶段的 `CARGO_TARGET_DIR` 一致。格式化在源码快照运行，只回写本批 Rust 文件。

| 阶段 | 命令 | 退出码 | 结果 |
| --- | --- | ---: | --- |
| 红灯、初轮绿灯、最终验证 | `cargo fmt --all` | 0／0／0 | 格式化成功 |
| 红灯 | `cargo test -p vw-agent-apps --lib auth6_agy_switch -- --nocapture` | 101 | 0 通过、2 失败、770 过滤 |
| 初轮绿灯、最终验证 | `cargo test -p vw-agent-apps --lib auth6_agy_switch -- --nocapture` | 0／0 | 每轮 2 通过、0 失败、770 过滤 |
| 相邻回归 | `cargo test -p vw-agent-apps --lib auth6_agy_login -- --nocapture` | 0／0 | 每轮 2 通过、0 失败、770 过滤 |
| 相邻回归 | `cargo test -p vw-agent-apps --lib auth6_keychain -- --nocapture` | 0／0 | 每轮 2 通过、0 失败、770 过滤 |
| 初轮绿灯、最终验证 | `cargo clippy --all-targets --all-features -- -D warnings` | 0／0 | 全工作区通过 |
| 差异检查 | `git diff --check`、`git diff --cached --check` | 0／0 | 通过 |

各阶段均已释放本任务的槽位锁并保留缓存。
