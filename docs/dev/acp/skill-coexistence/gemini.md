# Gemini ACP 技能共存验收

Darwin arm64 上使用 Gemini CLI 0.66.0 的固定原生入口 `gemini --experimental-acp`。ShiJu 通过 `session_model` 将模型绑定到本机 Google GenerateContent 夹具 `coexistence-fixture/gemini-2.5-flash`。网关、ACP、技能发现、激活、文件读取和只读 stdio MCP 均运行真实进程。私有项目配置启用 Gemini 原生目录信任，仅登记该验收工作区。会话采用 `full_access=false`，技能和 MCP 分别经宿主单次确认。

## 修复前

受审修订 2 的七个文件摘要均匹配合约。复用槽位 3 的已构建网关，SHA-256 为 `77e0305d944a592e6f5756753527358d6af45a3bd160c3b0f943fe3817c49058`。探针退出 1。基线第一条真实模型请求缺少原生技能目录，回合产生零条 `chat.done`、一条 `chat.error`、零次 MCP 下游调用。基线沿用 Gemini 的默认信任配置，修订 3 复验显式启用目录信任。脱敏回执保存在 `target/acp-skill-coexistence/gemini-before.json`。

修订 2 将会话视图作为唯一技能来源。修订 3 在 Gemini 工作区信任检查之后追加会话视图，空选择继续扫描原生目录。重复名称会明确拒绝。供应商原生策略仍决定是否允许、拒绝或请求激活确认。宿主只接收由受审 Gemini 在活动提示中发出的结构化激活确认，并提供单次批准及拒绝。现有 MCP 服务校验和工作流门禁继续生效。

## 制品与定向检查

受管修订目录为 `gemini-0.66.0-model-source-v3`。从固定 npm 包验证 SHA-512 后重新应用仓库补丁，七个文件 SHA-256 与合约全部匹配。原生入口文件 `bundle/gemini.js` 的 SHA-256 是 `f48ad00f929e699361d442440e89a70ab08342f9042ea22f99f0e9ff4bf60ba6`。六个改动的 JavaScript 文件均通过语法检查。[制品摘要](../../../../patches/acp-model-source/gemini/hashes.json)

直接调用受审 ACP 进程验证了原生和挂载技能激活、附件读取及 MCP 批准与拒绝。原生信任文件对工作区显式拒绝时，项目技能、挂载技能和试验 stdio MCP 均未进入模型目录。给原生和挂载技能设置相同名称时，实际进程在模型回合之前拒绝初始化。

槽位 3 的 `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均指向仓库内的 `target/rust-slot-3`。客户端定向测试退出 0，18 项通过。其中包括受审技能确认、异会话拒绝、权限提示、只读工作区边界和已有 MCP 单次审批。运行时定向测试退出 0，2 项通过，覆盖空选配置和未受审制品拒绝。Python 验收证据判定的 3 项定向测试通过。

```sh
python3 scripts/cargo_slot.py cargo test --locked -j 1 -p acp-client --lib gemini -- --test-threads=1
python3 scripts/cargo_slot.py cargo test --locked -j 1 -p llm-runtime --lib acp_skill_coexistence_gemini -- --test-threads=1
python3 -m unittest discover -s scripts/tests -p test_verify_acp_skill_coexistence.py
```

## 正式网关复验

槽位 3 的四次正式网关构建均退出 101。前两次分别遇到并行任务的 `task-scheduler` 存储方法和返回类型错误，以及 `gateway-agent-scheduler` 的 `uuid` 正式依赖缺失。第三次构建期间并行任务新增任务属性接口，`AgentSchedulerTaskPropertyChanges` 导出与 `property_changes` 字段发生错配。等待修复后第四次构建仍遇到并行接口变动，`task-scheduler/src/store/tasks.rs:690` 报 `E0609`，编译产物中的 `AgentSchedulerTaskPropertyOption` 只有 `id` 和 `name`。前序修复后均复用了槽位 3 的缓存。

任务属性接口提交后，槽位 3 的正式网关构建与回环探针均退出 0。网关 SHA-256 为 `30588d82fe4e615867e46dd555990e42eeae49d5a0ba79e8cb81e67ab791e244`，脱敏结构化结果见 `target/acp-skill-coexistence/gemini.json`。下列命令可复验。

```sh
export GEMINI_COEXISTENCE_RUNTIME="$PWD/target/acp-skill-coexistence/gemini-0.66.0-model-source-v3"
python3 scripts/cargo_slot.py sh -c 'cargo build --locked -j 1 -p vw-cli --bin shiju && python3 scripts/verify_acp_skill_coexistence.py --agent gemini --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/gemini.json'
```

| 轮次 | 新会话 | 模型请求 | `chat.done` / `chat.error` | MCP 审批 | 下游调用 |
| --- | --- | ---: | ---: | --- | ---: |
| 基线 | `ses_e07da4bebffe2sXgYQWg1ECDre` | 3 | 1 / 0 | 无 | 0 |
| 挂载 | `ses_e07da3838ffeD2Y1kE1QtAkDQf` | 6 | 1 / 0 | 一次放行 | 1 |
| 撤除 | `ses_e07da210bffeOhesiKDy9A5Z4m` | 3 | 1 / 0 | 无 | 0 |
| 拒绝 | `ses_e07da0fe1ffeB1YzufkiheWlXP` | 6 | 1 / 0 | 拒绝 | 0 |

挂载轮原生激活为 `call_84eb0ac339f9df03910ba4a5`，原生附件读取为 `call_7cd028e71ebff5b7f0a04630`。选中技能激活为 `call_6e5229b38f34b5fd65f5c12b`，从会话视图读取附件为 `call_0d9f225836456844ea930254`。MCP 结果按 `call_f09571259a6aaf62ca4da424` 返回模型，拒绝轮按 `call_992eb63282d7243040089469` 读回取消。结构化回执保留四轮工具调用 ID。

四轮均回读 `gemini` 0.66.0、`session_model`、`coexistence-fixture/gemini-2.5-flash` 和 `full_access=false`。每轮首次模型请求均无技能正文或附件，未选中的 ShiJu 技能始终留在目录之外。撤除轮仍可激活并读取原生技能。修订 3 的 `cli_local` 生命周期和其他平台需要独立复验。
