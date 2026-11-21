# 任务 41 适配 qwen

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：未开始

来源任务：无

目的：完成 qwen 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/qwen.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/qwen.rs`、`crates/libs/acp-access-policy/src/access_policy/qwen.rs`、`patches/acp-model-source/qwen`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_qwen_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/qwen.md`、`docs/dev/acp/skill-coexistence/qwen.md`。

## 上下文

审计基线版本为 0.26.7。MCP 现状｜原生搜索链已执行及回传，短名漏 node；私有关搜索后三项齐全。模型来源现状｜原生 OpenAI 配置；受审；通用。原生工具现状｜普通/挂载默认可用 ToolSearch；关搜索仅私有取证设置，服务转发器独立。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（选模与认证锁、私有状态、技能根，合入管理员安全配置），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_qwen_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_qwen_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。
