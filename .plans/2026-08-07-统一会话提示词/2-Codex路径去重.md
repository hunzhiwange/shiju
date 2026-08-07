# 任务 2: Codex 路径去重

批次：【批次 2】 <依赖批次 1>

状态：未开始

目的：统一提示后，Codex OAuth 路径不再把同一份系统提示追加第二次。

来源任务：无

预计会话范围：只改 `build_system_messages` 的 `is_codex` 分支与对应消息测试。

## 文件

- 修改：`crates/vw-agent/src/session/llm/messages.rs`
- 修改：`crates/vw-agent/src/session/llm/messages_tests.rs`

## 上下文

- `build_system_messages` 总会 `extend(provider(...))`；当 `is_codex` 再 `push(instructions())`。
- 统一后 `provider` 与 `instructions` 是同一正文，`is_codex=true` 会整文重复。
- `stream.rs` 中 `is_codex` 仍用于去掉 options 里的 `instructions` 字段与 token 逻辑，那些行为本任务不改；只去掉提示正文二次追加。
- 依赖任务 1：统一装载已完成，否则去重语义不稳定。

## 测试计划

- 行为：对同一输入，`build_system_messages(..., true)` 与 `false` 含相同一份统一提示，不再因 `is_codex` 使统一提示出现两次。
- 失败验证测试：改写 `build_system_messages_adds_codex_instructions_when_requested` 为去重语义，例如 `build_system_messages_does_not_duplicate_unified_prompt_for_codex`。
- 失败验证命令：`cargo test -p vw-agent build_system_messages_does_not_duplicate_unified_prompt_for_codex -- --nocapture`
- 预期失败原因：实现前 `with_codex` 仍更长且可能含两份相同大段正文。
- 通过验证命令：同上命令，以及 `cargo test -p vw-agent build_system_messages_ -- --nocapture`
- 模拟策略：沿用 `messages_tests` 的 `test_input` / `test_model`；不 mock 网络。

断言建议：

- `with_codex` 与 `without_codex` 都包含统一提示中的稳定标记片段一次（可用 `matches().count() == 1`）。
- `is_codex` 不再要求 `with_codex.len() > without_codex.len()`。

## 步骤

1. 改写/新增失败测试证明 `is_codex` 不重复统一提示。
2. 运行失败验证确认当前会重复。
3. 删除或改写 `if is_codex { parts.push(instructions()); }`：统一后该分支对提示正文无操作；保留函数参数与 `stream.rs` 其它 `is_codex` 用途。
4. 更新 `build_system_messages` 文档注释，说明 Codex 不再二次注入系统提示正文。
5. 跑通过验证。

## 验证

- 运行：`cargo test -p vw-agent build_system_messages_ -- --nocapture`
- 预期：去重测试通过；其它 `build_system_messages_*` 仍通过。
- 所需证据：失败时重复计数 >1 的输出；通过后计数 ==1；退出码 0。

## 完成

完成后：OpenAI OAuth/Codex 流不再双倍系统提示；其它 `is_codex` 选项处理保持不变。
