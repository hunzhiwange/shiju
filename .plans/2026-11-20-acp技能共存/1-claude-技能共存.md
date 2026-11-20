# 任务 1：claude 技能与原生技能共存

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 1】依赖：无

状态：未开始

目的：让 `claude` 在同一会话使用 Codex 验收所示的原生技能与 ShiJu 选中技能，并同时完成指定模型下的已授权 MCP 回合。

来源任务：无。以[本适配器 MCP 验收](../../docs/dev/acp/mcp-acceptance/claude.md)和[工具审计矩阵](../../docs/dev/acp/mcp-policy-audit/README.md)对应条目为基线。

预计会话范围：固定版 0.26.0 / SDK 0.2.96 / Code 2.1.96 的一个原生加载入口、宿主接线和三轮只读验收。交付 Claude 行为及后续任务的共享探针入口。

## 文件

- 新建本项真实回环探针：`scripts/acp_skill_coexistence/claude.py`。
- 新建回执：`docs/dev/acp/skill-coexistence/claude.md`。
- 参考：`crates/libs/llm-runtime/src/acp/model_source/drivers/claude.rs` 的启动配置及相邻测试。
- 修改候选：`crates/libs/llm-runtime/src/acp/skill_mounts/claude.rs` 及相邻独立测试文件。
- 现行技能制品引用变化时修改：`docs/vibewindow/acp-skill-mounts/agents.json` 的 `claude` 条目。
- 历史技能回执：[claude](../../docs/vibewindow/acp-skill-mounts/evidence/claude.json)。
- 新建共享验收入口：`scripts/verify_acp_skill_coexistence.py`、`scripts/acp_skill_coexistence/__init__.py`。
- 新建独立测试：`scripts/tests/test_verify_acp_skill_coexistence.py`。

## 上下文

当前由 SDK 本地插件注册会话视图。

## 实施步骤

1. 依据本适配器的 MCP 回执和受审启动驱动选择固定入口，核对版本、平台及摘要。沿该适配器已审传输运行只读服务，以 session_model 绑定回环。本卡按目标驱动观察实际模型协议和工具结构。让业务请求从 `defaults.skills` 和宿主 MCP 配置解析选择。
2. 在 launch settings 和 SDK 插件选项定位项目技能与会话插件覆盖关系。分别用原生技能名和插件限定名激活，优先以原生 Messages 取证。转换模型路径须有单独证据。
3. 验证原生发现、激活和读取均沿已证实的供应商路径。完成本 key 的选集、启动、权限及制品接线。无法安全接入时保留门禁并记录首个断点。

## 共享验收入口

- 复用 `scripts/verify_acp_integration.py` 的进程和隔离准备模式及 `scripts/acp_mount_integration.py` 的真实 MCP。入口接受 `--agent`、`--gateway-bin`、`--output`，按注册 key 加载独立脚本 `scripts/acp_skill_coexistence/<下划线 key>.py`。
- 由供应商模块提供固定入口、现行模型协议及传输、原生技能来源和根据观察决定工具动作的夹具。公共入口负责私有网关及项目设置、三次业务请求、许可回复与证据判定。写明供应商文件的定义及可复用接口。夹具不增加供应商能力，禁用未实现的 key。
- 运行 `python3 -m unittest discover -s scripts/tests -p test_verify_acp_skill_coexistence.py`。用 `test_rejects_missing_or_uncorrelated_results` 等覆盖缺少正文、错配 ID 与零调用。Claude 若被外部条件阻塞，仍可新增 `scripts/acp_skill_coexistence/codex.py`，在下方同一命令改用 `--agent codex --output target/acp-skill-coexistence/codex-baseline.json`，用已验制品验证和交付公共入口。保留 Claude 自身未完成项。

## 测试计划与验证

- 失败验证：先建当前适配器真实探针，在修复前执行下方同一命令。若现行 Claude 行为已正确，用独立测试先证明验收器拒绝错配结果，再取实际三轮正向证据。
- 通过验证：复跑三轮和一次 MCP 拒绝对照。仅替换供应商模型响应。验收从仓库根运行，构建、制品及探针共用持有的 Cargo 槽位。已有已验构建且实现无变化时复用该产物。

```sh
python3 scripts/cargo_slot.py sh -c 'cargo build --locked -j 1 -p vw-cli --bin shiju && python3 scripts/verify_acp_skill_coexistence.py --agent claude --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/claude.json'
```

- 若修改 SDK 启动元数据，范围限定 `acp-client` 的独立 `crates/libs/acp-client/src/client/session_meta_tests.rs`。扩展现有 Claude 挂载选项用例，运行 `python3 scripts/cargo_slot.py cargo test --locked -j 1 -p acp-client --lib acp_skill_mount_claude_ -- --test-threads=1`，确认至少一项实际运行。
- 完成证据：退出 0、三次新会话各一个 `chat.done` 和零 `chat.error`、实际原生与挂载读取及 MCP 的调用 ID、模型/权限回读、MCP 计数 0/1/0 和拒绝后的零次调用。按公共规则记录修复前失败或实际通过基线、平台和制品摘要。结构化证据位于 `target/acp-skill-coexistence/claude.json`。
