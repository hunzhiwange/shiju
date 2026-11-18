# 任务305：修复vw-agent宿主的tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：继续执行宿主单测；诊断编号30, 31（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/libs/agent-gateway/src/webhook_ingress/tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断30：E0425

位置：`crates/libs/agent-gateway/src/webhook_ingress/tests.rs:381`。

```text
error[E0425]: cannot find value `RATE_LIMIT_WINDOW_SECS` in this scope
   --> crates/vw-agent/src/gateway/webhook_ingress/../../../../libs/agent-gateway/src/webhook_ingress/tests.rs:381:37
    |
381 |     assert_eq!(body["retry_after"], RATE_LIMIT_WINDOW_SECS);
    |                                     ^^^^^^^^^^^^^^^^^^^^^^ not found in this scope
```

当前源码片段：

```rust
377:             .expect("second request should be rate limited"),
378:     );
379:
380:     assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
381:     assert_eq!(body["retry_after"], RATE_LIMIT_WINDOW_SECS);
382: }
383:
384: #[test]
385: fn enforce_rate_limit_scopes_limits_to_each_peer_address() {
386:     let state = state_with(
```

### 宿主诊断31：E0425

位置：`crates/libs/agent-gateway/src/webhook_ingress/tests.rs:628`。

```text
error[E0425]: cannot find value `RATE_LIMIT_WINDOW_SECS` in this scope
   --> crates/vw-agent/src/gateway/webhook_ingress/../../../../libs/agent-gateway/src/webhook_ingress/tests.rs:628:37
    |
628 |     assert_eq!(body["retry_after"], RATE_LIMIT_WINDOW_SECS);
    |                                     ^^^^^^^^^^^^^^^^^^^^^^ not found in this scope
```

当前源码片段：

```rust
624:     );
625:
626:     assert_eq!(first_status, StatusCode::OK);
627:     assert_eq!(second_status, StatusCode::TOO_MANY_REQUESTS);
628:     assert_eq!(body["retry_after"], RATE_LIMIT_WINDOW_SECS);
629:     assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
630: }
631:
632: #[tokio::test]
633: async fn handle_webhook_inner_returns_auth_error_before_provider_call() {
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib enforce_rate_limit_returns_retry_after_after_limit_is_exceeded -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib handle_webhook_inner_returns_rate_limit_before_provider_call -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。

## 本轮执行证据（2026-11-18）

- 根因：库父模块私有导入的常量无法经宿主的 `agent_gateway::webhook_ingress::*` 带入共享测试；直接从 `gateway_types` 显式导入，不改生产逻辑或安全断言。
- 两个过滤名均已核对为真实 `#[test]` / `#[tokio::test]` 入口。
- 修复前后均执行：`python3 scripts/cargo_slot.py sh -c 'cargo test -p vw-agent --lib enforce_rate_limit_returns_retry_after_after_limit_is_exceeded -- --nocapture && cargo test -p vw-agent --lib handle_webhook_inner_returns_rate_limit_before_provider_call -- --nocapture'`。
- 两轮均为槽位3；`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。
- 修复前退出101，70条编译错误，包含本任务两处 E0425；修复后退出101，剩68条其他文件错误，本文件诊断消失。日志：`/tmp/shiju-task305-before.log`、`/tmp/shiju-task305-after.log`。
- 有效测试数0，尚无行为通过证据；第一条编译失败，第二条因 `&&` 未执行。剩余阻塞包括 `agent/loop_.rs` 的 core 导入、llm host_suites 的 acp 导入、compaction fixture 导入等，归兄弟任务处理。
- 两轮入口均已退出并释放自有槽位锁，保留缓存。待兄弟宿主编译阻塞解除后重跑上述两条命令；保留编号文件，不声称完成。

## 复用公共产物回归（2026-11-18）

- 核对了提交 `6e6253b81` 已包含在任务304公共编译开始版本 `3382a35f8` 中。公共编译结束版本 `17e53b009`，退出0，凭据 `/tmp/shiju-host-common-304-LYrH28/receipt.md`。
- 领取槽位1，直接运行凭据中与 Cargo JSON 一致的 `target/rust-slot-1/debug/deps/vw_agent-e6b6e74ae78b3318`，SHA-256 `5548b4e2c673a564f9d12af91061c0e01e0334f68881fd8cf360a1cbf5933e78`。未重编。
- 参数为 `--exact --nocapture gateway::webhook_ingress::tests::enforce_rate_limit_returns_retry_after_after_limit_is_exceeded gateway::webhook_ingress::tests::handle_webhook_inner_returns_rate_limit_before_provider_call`。有效2项，1 passed、1 failed，退出101。日志及命令凭据见 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-305-host-r63iwqsb/` 下的 `tests.log`、`tests.exit`、`command.json` 和 `artifact.txt`。
- 用例使用内存夹具和本地 provider 桩。宿主提示词可扫描工作区，因此 HOME、CODEX_HOME、XDG、应用配置、状态根和工作目录都指向隔离目录，另用 macOS 沙箱拒绝读写真实 Codex 配置。
- 限流响应断言通过。宿主入口用例首次请求构建提示词时，`session-engine/src/prompt.rs:64` 的 `block_in_place` 在默认单线程测试运行时 panic。将该用例切换到 Tokio 多线程运行时，保留原断言。旧产物无法验证这项改动，待公共编译窗口释放后更新宿主产物并隔离重跑两条用例，任务保留待回归。
