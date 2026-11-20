# Qoder 技能与原生技能共存

2026-11-20，Darwin arm64，固定 `@qoder-ai/qodercli@1.1.88`。使用受管 `qodercli --acp`，以 `session_model` 绑定回环 Chat Completions。在原生默认权限下验证 stdio 的只读 `read_probe`。模型夹具只根据实际目录和工具声明发出调用。技能正文、附件和 MCP 返回值来自独立私有磁盘夹具。

| 制品 | 核验摘要 |
| --- | --- |
| npm 归档 SHA-512 | `3UkODiq9eNtbQPzE1DN6UbcwCjNcY5XKeO0htxzzm5ZZjs2gE2yY8xzTjwEU6Hb8Dhyiy7XdcqSyKwWudxmh+w==`（Base64） |
| 原始 `bundle/qodercli.js` SHA-256 | `1858e47ccf930ea91f30cfb9d2889e3cfb55c34beb1cf7b66a9a5e1a18efacb3` |
| v2 `bundle/qodercli.js` SHA-256 | `a1f407619fbc3fcf771a2512c385ec318680b407dd1c113d45ec713cd20bd225` |
| v2 `bundle/shiju-session-model.mjs` SHA-256 | `f9381ff8c6870ba7d45a3b59116e978a37c50c41fdfc9806920d8d1ad7c811de` |

## 修复前

复用已有槽位 3 网关制品，SHA-256 为 `808b01432ed6f30664f849b0130a97a320b952d395aa34c6723e6ec19be2edf8`。早期项目目录诊断发现 `--setting-sources user` 保留原生用户技能。将基线夹具置于隔离 HOME 的 `.agents/skills` 后，`ses_e076c90e2ffe9T1Td3s2cP5yf3` 收到一次 `chat.done`，零 `chat.error`，原生 `Skill` 和 `Read` 的调用 ID 分别为 `call_6908a44444d1f57ab1d5fcb2` 和 `call_45c9e52e8c3d2dbf6f537ff7`。挂载轮 `ses_e076c76dcffei7r1VRk58f7n8D` 在旧宿主门禁结束。零模型请求、零 MCP 下游调用、一次 `chat.error`，进程退出 1。比对文件为 `target/acp-skill-coexistence/qoder-red.json`。基线复用了既有网关，证据的槽位环境字段为空。

## 接入与验证

宿主把 `defaults.skills` 解析出的会话视图发布到请求私有配置的 `skills` 入口。Qoder 原生用户扫描器同时保留 HOME 下的 `.agents/skills`。真实同名对照中，私有配置的技能按原生优先级胜出。挂载视图只保留技能发现、激活和读取所需的元数据。内联命令返回原生禁用提示，钩子、工具授权及技能级选模等执行声明不生效。原生技能、项目来源策略、BYOK 绑定校验、权限与 MCP 工具直接声明沿用原有实现。

修订 v2 真实进程的独立 ACP 对照四轮均返回 `end_turn`。每轮的原生 `Skill` 和附件 `Read` 都回到对应调用 ID。基线及撤除各 3 次模型请求，挂载及拒绝各 6 次。MCP 下游计数为 0、1、0、0。批准调用 `call_ab66318aceeb5c62c5cb9875` 返回磁盘夹具，拒绝调用 `call_281326443f14fc0d67d520d2` 将拒绝结果回传给模型。挂载正文含内联命令禁用标记。Node 绑定及执行边界测试通过 4 项，退出 0。

## 网关回归

槽位 10 的 `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为仓库下的 `target/rust-slot-10`。本次 `cargo build --locked -j 1 -p vw-cli --bin shiju` 已编过 `acp-agent-registry` 与 `llm-runtime`。共享的 `task-scheduler` PostgreSQL 工作计划代码随后报告 E0271（调度记录从 7 列改为 9 列）和 E0616（`PlanMember.id` 的可见性），构建退出 101。网关绿测的实际会话数为 0，首个断点在构建。日志见 `target/acp-skill-coexistence/qoder-build.log`。

在线安装器尝试超时。已核对原始 npm 归档与候选制品摘要，可用 `QODER_COEXISTENCE_PACKAGE` 向探针提供核验后的私有副本。Rust 定向验证复用了槽位 3 刚编译的 `llm-runtime` lib-test 产物，重新领取槽位后执行 `"$CARGO_TARGET_DIR/debug/deps/llm_runtime-85f23fc8970681f8" acp_skill_coexistence_qoder --test-threads=1`。两个缓存路径均为仓库下的 `target/rust-slot-3`，退出 0，2 项通过。结果见 `target/acp-skill-coexistence/qoder-warm-test.log`。网关复验应生成 `target/acp-skill-coexistence/qoder.json`。本项覆盖新会话、撤除及拒绝对照。跨进程恢复和热更新留待各自验收。
