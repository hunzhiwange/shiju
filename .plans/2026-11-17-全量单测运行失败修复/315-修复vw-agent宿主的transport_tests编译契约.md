# 任务315：修复vw-agent宿主的transport_tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：继续执行宿主单测；诊断编号33（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/libs/llm-runtime/tests/host_suites/transport_tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断33：E0425

位置：`crates/libs/llm-runtime/tests/host_suites/transport_tests.rs:51`。

```text
error[E0425]: cannot find function `summary_output_budget` in module `crate::session::processor::context_governance`
  --> crates/vw-agent/src/session/llm/../../../../libs/llm-runtime/tests/host_suites/transport_tests.rs:51:77
   |
51 |                 let budget = crate::session::processor::context_governance::summary_output_budget(&input.model.limit);
   |                                                                             ^^^^^^^^^^^^^^^^^^^^^ not found in `crate::session::processor::context_governance`
```

当前源码片段：

```rust
47:                 let fixture = RecoveryConfig::new(&format!("http://{address}/v1"), Some("fixture-key")).await;
48:                 let mut input = test_input("model-recovery");
49:                 input.model = provider::get_model("model-recovery", "original").await.unwrap();
50:                 input.model.limit = provider::ModelLimit { context: 1_048_576, input: None, output: 1_048_576 };
51:                 let budget = crate::session::processor::context_governance::summary_output_budget(&input.model.limit);
52:                 input.agent.options.insert("max_output_tokens".into(), json!(budget));
53:                 input.agent.options.insert("disable_tools".into(), json!(true));
54:                 let result = stream(input, |_| {}).await;
55:                 assert!(result.is_ok(), "{result:?}");
56:                 let payload = server.join().unwrap();
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib compaction_output_budget_reaches_openai_compatible_request -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。

## 本次执行记录

- 根因：`context_governance::summary_output_budget` 只在 session-engine 自身 cfg(test) 重导出，vw-agent 宿主编译依赖库时不可见。改用现有公开 `session_engine::compaction_budget::summary_output_budget`，保留真实流请求和 20_000 token 断言，不改生产 API。
- 已核对测试入口为 `#[test] fn compaction_output_budget_reaches_openai_compatible_request`，父模块注册在 vw-agent `session/llm/stream.rs`。
- 修复前命令：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib compaction_output_budget_reaches_openai_compatible_request -- --nocapture`；槽位3，两个缓存路径均 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`；退出101，编译51条错误，包含本任务 E0425；有效测试数0，未通过。完整输出 `/tmp/shiju-315-red.log`。
- 曾排队等待其他 Cargo/rustc 退出后运行相同定向命令；等待时其他编译持续占用，排队入口已停止（退出130，尚未启动第二次 Cargo），无本任务构建子进程或槽位遗留。宿主仍有51条编译错误，未取得行为通过证据；保留任务文件，待兄弟修复解除阻塞后复验相同命令。
- 轻量检查：本次源码 `git diff --check` 通过。
