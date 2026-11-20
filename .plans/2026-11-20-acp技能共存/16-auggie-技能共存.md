# 任务 16：auggie 技能与原生技能共存

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 16】依赖：批次 1、15

状态：未开始

目的：让 `auggie` 在同一会话使用 Codex 验收所示的原生技能与 ShiJu 选中技能，并同时完成指定模型下的已授权 MCP 回合。

来源任务：无。以[本适配器 MCP 验收](../../docs/dev/acp/mcp-acceptance/auggie.md)和[工具审计矩阵](../../docs/dev/acp/mcp-policy-audit/README.md)对应条目为基线。

预计会话范围：固定版 0.37.1 的一个原生加载入口、宿主接线和三轮只读验收。复用批次 1 的验收入口，skill_mounts/mod.rs 的能力分派按前一批次交接。

## 文件

- 新建本项真实回环探针：`scripts/acp_skill_coexistence/auggie.py`。
- 新建回执：`docs/dev/acp/skill-coexistence/auggie.md`。
- 参考：`crates/libs/llm-runtime/src/acp/model_source/drivers/auggie.rs` 的启动配置及相邻测试。
- 修改候选：`crates/libs/llm-runtime/src/acp/skill_mounts/auggie.rs` 及相邻独立测试文件。
- 修改：`crates/libs/llm-runtime/src/acp/skill_mounts/mod.rs`，仅接通本 key 的能力分派，保留未审入口拒绝。
- 制品变化时修改：`crates/libs/acp-agent-registry/src/managed_runtime/contracts.json` 的 `auggie` 条目和 `managed_runtime.rs` 的安装修订。
- 现行技能制品引用变化时修改：`docs/vibewindow/acp-skill-mounts/agents.json` 的 `auggie` 条目。
- 适配候选：`patches/acp-model-source/auggie/` 的固定补丁。

## 上下文

当前技能门禁拒绝。历史候选 0.36.0，现行受管 MCP 以 0.37.1 为基线。

## 实施步骤

1. 依据本适配器的 MCP 回执和受审启动驱动选择固定入口，核对版本、平台及摘要。沿该适配器已审传输运行只读服务，以 session_model 绑定回环。本卡按目标驱动观察实际模型协议和工具结构。让业务请求从 `defaults.skills` 和宿主 MCP 配置解析选择。
2. 先查固定 0.37.1 的原生技能加载或显式目录能力，再接受审视图。保留私有项目配置及原生 MCP，重新取得版本证据。
3. 验证原生发现、激活和读取均沿已证实的供应商路径。完成本 key 的选集、启动、权限及制品接线。无法安全接入时保留门禁并记录首个断点。

## 测试计划与验证

- 失败验证：先建当前适配器真实探针，在修复前执行下方同一命令。先证明无挂载基线可运行。挂载预期在宿主能力门禁或供应商目录注册明确失败，记录实际首个失败层。
- 通过验证：复跑三轮和一次 MCP 拒绝对照。仅替换供应商模型响应。验收从仓库根运行，构建、制品及探针共用持有的 Cargo 槽位。已有已验构建且实现无变化时复用该产物。

```sh
python3 scripts/cargo_slot.py sh -c 'cargo build --locked -j 1 -p vw-cli --bin shiju && python3 scripts/verify_acp_skill_coexistence.py --agent auggie --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/auggie.json'
```

- 若改动 Rust 技能配置或能力门禁，在 `crates/libs/llm-runtime/src/acp/skill_mounts/auggie_tests.rs` 增加独立的 `acp_skill_coexistence_auggie` 用例，先复现旧的拒绝或配置替换，再检查只追加受信视图及缺选时的配置。运行 `python3 scripts/cargo_slot.py cargo test --locked -j 1 -p llm-runtime --lib acp_skill_coexistence_auggie -- --test-threads=1`，要求至少一项实际运行。仅修改供应商实现时以审定制品及真实进程验收作为红绿验证。
- 完成证据：退出 0、三次新会话各一个 `chat.done` 和零 `chat.error`、实际原生与挂载读取及 MCP 的调用 ID、模型/权限回读、MCP 计数 0/1/0 和拒绝后的零次调用。按公共规则记录修复前失败或实际通过基线、平台和制品摘要。结构化证据位于 `target/acp-skill-coexistence/auggie.json`。
