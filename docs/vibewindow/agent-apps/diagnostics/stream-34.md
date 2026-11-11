# 任务 34：真实流式检测（待回归）

2026-11-11，macOS 27.0 / arm64，rustc 1.99.1。状态：实现已交付，验证被共享工作区的无关编译错误阻断。未验证真实服务商，不宣称行为绿灯。

## 实现与边界

- `gateway/agent_apps_diagnostic_executors/stream_check.rs::StreamProbe::prepare/run` 复用任务 33 的 `ModelTarget`，按保存的供应商、模型目录、授权作用域和账号代次执行单次请求。使用固定最小提示，无用户对话、工具定义或工具执行；标记可能产生费用，不改变生产熔断状态。
- `model_relay/stream_observer.rs::StreamObserver` 统一处理 Chat、Messages、Responses、Gemini 的 SSE 分帧、有效文本增量、上游实际模型和合法终态。生产 Messages/Responses/Gemini 解码器、relay Chat 与常驻 Chat 入口使用同一校验；既有客户端模型映射保留。诊断不复制协议解析器。
- Chat 要求 finish_reason 后的 `[DONE]`；Messages 要求开始、内容块及停止顺序，区分文本、工具与思考块；Responses 检查开始及 completed/incomplete 状态；Gemini 检查 finishReason。坏 JSON、坏 UTF-8、非法顺序及缺终态不能成功。生产帧限 4 MiB，诊断响应限 1 MiB。
- 只有有效文本增量、合法结束和上游实际模型同时存在才产生成功证据。记录首个事件时间、最大事件间隔；10 秒首事件/空闲、30 秒总 deadline，保活不延长预算，缓冲字节不能绕过已过期的 deadline。取消或合法结束释放连接。
- 独立测试位于 `stream_check_tests.rs`、`stream_observer_tests.rs`。新增受控真实 TCP/HTTP/SSE 夹具契约覆盖四协议有效/无文本终态、HTTP200 空正文、保活、坏帧、缺结束、断线、401/404、取消关闭连接、超时、字节碎片及客户端模型映射；尚待整体执行通过。

## 已运行证据

夹具定义（不是通过结果）：应用 Claude 按保存的 apiFormat 选择四协议；请求模型 fixture，返回模型 fixture-v1；Chat/Messages/Responses 使用 /v1 路径，Gemini 使用 /v1/models/fixture:streamGenerateContent?alt=sse，Messages 版本头为 2023-06-01。

所有 Cargo 阶段均动态按 1–20 原子领取槽位；CARGO_BUILD_BUILD_DIR 与 CARGO_TARGET_DIR 相同，临时设置 CARGO_INCREMENTAL=0、CARGO_PROFILE_TEST_CODEGEN_UNITS=256。仅 macOS，默认 vw-agent features，Clippy 按规定全部 features。

| 命令 | 实际槽位 | 退出码 / 有效测试数 | 结果 |
| --- | --- | --- | --- |
| `cargo fmt --all -- --emit stdout` | 4、1、8 | 0 / 不适用 | 输出到临时文件；随后 rustfmt 的 skip_children=true 仅写本任务负责文件 |
| `cargo test -p vw-agent --lib diagnostic_stream_check_requires_valid_completion -- --nocapture`（红灯） | 1 | 101 / 1 项失败 | HTTP200 的有效 Chat SSE 触发 `valid increment and legal completion must succeed: openai_chat`，实际为 FirstEvent/InvalidResponse；属于预期行为断言，非编译红灯 |
| 同一精确命令（第一次通过尝试） | 1 | 101 / 0 项执行 | `session/llm/aisdk/driver_tests.rs:178` 的 E0382：endpoint 移入闭包后再次使用；该改动随后由工作区所有者修复 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 8 | 101 / 不适用 | `vw-api-types/src/computer_control_tests.rs:1` 的 E0432：computer_control 未导出 |
| `cargo clippy -p vw-agent --lib --all-features -- -D warnings` | 8 | 0 / 不执行测试 | 本任务所在库全部 features 通过；14 个负责 Rust 文件检查前后 SHA-256 一致 |
| 同一精确行为命令（最终代码重试） | 8 | 101 / 0 项执行 | `vw-api-types/src/voice.rs:3` 的 E0432/E0433：引用 base64，但该 crate 未注册依赖 |

红灯构建 38m53s，运行 0.08s；出现既有 macOS `__eh_frame` 大于 16MB 的链接器提示。库 Clippy 用时 4m17s。更早槽位 5 的接线编译错误已修复，另一次被共享 Pending.project_preimage 测试字段缺失阻断，均未计作红灯。中断时仅停止自己的遗留编译进程。

实际 CARGO_TARGET_DIR：

- 槽位 1：`/Users/xiangmin/worker/code/shiju/target/rust-slot-1`
- 槽位 4：`/Users/xiangmin/worker/code/shiju/target/rust-slot-4`
- 槽位 5：`/Users/xiangmin/worker/code/shiju/target/rust-slot-5`
- 槽位 8：`/Users/xiangmin/worker/code/shiju/target/rust-slot-8`

所有本任务 Cargo/产物进程已结束，自己的锁已释放，缓存保留。未创建或启动工作树，plan.md 未修改。

## 回归与合入范围

由相关所有者恢复上述共享编译，再按任务卡依次执行精确测试、取消/超时和 shared_stream_observer_ 测试，以及现有 Responses/Gemini/ModelProbe 回归，最后全仓 Clippy。除红灯外，本次没有通过的行为测试，不能用库 Clippy 替代。通过后将任务 34 标为已完成并删除编号卡。

`model_relay/transport.rs` 的其他协议路由改动、`model_relay/mod.rs` 的 routing 注册，以及 Voice、Computer Control、AISDK 改动均保留在各自所有者范围；本任务仅合入流观察器相关 hunk。
