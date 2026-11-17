# 任务332：修复vw-agent宿主的controls_tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：继续执行宿主单测；诊断编号34（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/vw-agent/src/session/llm/acp/controls_tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断34：E0425

位置：`crates/vw-agent/src/session/llm/acp/controls_tests.rs:30`。

```text
error[E0425]: cannot find value `ACP_CLIENT_CACHE` in this scope
  --> crates/vw-agent/src/session/llm/acp/controls_tests.rs:30:21
   |
30 |     let mut cache = ACP_CLIENT_CACHE.lock();
   |                     ^^^^^^^^^^^^^^^^ not found in this scope
```

当前源码片段：

```rust
26:     )
27:     .await;
28:     assert!(!Arc::ptr_eq(&first.client, &second.client));
29:     assert_ne!(first.client.tenant, second.client.tenant);
30:     let mut cache = ACP_CLIENT_CACHE.lock();
31:     cache.remove(&first.key);
32:     cache.remove(&second.key);
33: }
34:
35: #[tokio::test]
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib acp_session_options_follow_capabilities_isolate_tenant_connections -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。

## 本轮执行记录

- 根因：`controls_tests` 注册在 `controls::tests`，`use super::*` 不会引入祖父模块的缓存导入。直接导入运行时公开的 `llm_runtime::acp::cache::ACP_CLIENT_CACHE`，不改生产接口和租户隔离断言。
- 已核实过滤名为 `#[tokio::test]` 真实入口。
- 修改前后均运行 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib acp_session_options_follow_capabilities_isolate_tenant_connections -- --nocapture`；均槽位1，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。
- 修改前退出101，包含当前E0425及7条兄弟VideoContext E0277；修改后退出101，当前E0425消失，仅剩7条兄弟错误，涉及 prepare_tests、entry_test_transport、e2e_tests，归任务335—337。有效运行测试0，不能记为行为通过。复查日志 `/tmp/shiju-task332-test.log`。
- `git diff --check -- crates/vw-agent/src/session/llm/acp/controls_tests.rs` 通过；自有入口及子进程均已退出，槽位已自动释放、缓存保留。未运行全量或桌面构建。
- 保留本卡；兄弟编译阻塞解除后须重跑上述定向命令，取得有效数>0、failed=0、退出0再完成清理。
