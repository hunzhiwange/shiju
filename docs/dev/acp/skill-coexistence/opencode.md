# OpenCode ACP 技能共存验收

2026-11-20，Darwin arm64，状态为已阻塞。受管 `opencode-ai` 1.19.5 v1 的 `reviewed/opencode` 通过严格码签校验，自报 1.19.5，SHA-256 为 `1a1f10028d3edebddd6df10a465cfa79b58d710875173dbd32137a8b115daa2c`。沿用受审 ACP 启动参数 `acp`、原生 stdio MCP 和私有回环 Chat Completions。[原 MCP 验收](../mcp-acceptance/opencode.md)

## 生产网关基线

探针从 `defaults.skills`、`defaults.mcp` 和宿主 `.vwacprc.json` 解析选集，发送 `full_access=false`，指定 `session_model` 和 `coexistence-fixture/deepseek-v4.1-flash-1017`。复用槽位 10 中已验的网关，SHA-256 为 `b2a05ed113477712c36de9dd731517d27f151d4e51f7c8096c0e80ae2e27d276`。两个 Cargo 缓存路径均指向该槽位。基线和复跑均退出 1，证据位于 `target/acp-skill-coexistence/opencode-before.json` 和 `target/acp-skill-coexistence/opencode.json`。

基线会话在启动 OpenCode 前触发 `requires_full_access`。收到一个 `chat.error`、零个 `chat.done`、零条模型请求和零次 MCP 下游调用。正式网关未进入挂载、撤除或拒绝对照，未取得模型及权限回读。既有桌面门禁要求有效的全访问授权，因为宿主审批无法约束 OpenCode 的本机原生工具。本次隔离会话以受限权限发起。[桌面权限门禁](../../../../crates/libs/acp-client/src/client/process.rs)

## 原生 ACP 对照

同一 v1 二进制在私有目录执行 `acp --pure`，空 `SHIJU_OPENCODE_SKILL_ROOT` 使模型首轮目录缺失项目原生技能。这个断点与现行扫描补丁一致，早于技能激活。原生证据位于 `target/acp-skill-coexistence/opencode-production-v1.json`。

上游原版 1.19.5 的 SHA-256 为 `feaafead8cd563af0c2dcb386b0757e4355e327ebb6bd1aaad5440fe2f83596c`，与固定安装合约相符，严格码签校验通过。私有 ACP 对照保留 `.agents/skills` 的原生发现，使用原生 `skills.paths` 追加独立的会话视图，以 `coexistence_*=ask` 请求 MCP 单次审批。探针逐个验证原生 `skill` 的正文、返回路径规范化后的身份、原生 `read` 的随机附件、未选技能缺席和首次模型请求无正文。MCP 放行通过工具调用 ID 核对实际续轮结果。MCP 拒绝通过同 ID 的原生失败事件核对，OpenCode 随即结束提示。

| 轮次 | ACP 终态 | 模型请求 | MCP 审批 | 下游调用 |
| --- | --- | ---: | --- | ---: |
| 基线 | `end_turn` | 3 | 无 | 0 |
| 挂载 | `end_turn` | 6 | 单次批准 | 1 |
| 撤除 | `end_turn` | 3 | 无 | 0 |
| 拒绝 | `end_turn` | 5 | 拒绝 | 0 |

四轮独立会话及关联调用 ID 位于 `target/acp-skill-coexistence/opencode-upstream.json`。私有追加扫描原型也通过四轮，另用 `--pure debug skill` 确认原生同名技能在原型中优先。原型采用固定输入摘要和等长替换，重签后通过严格码签验签，入口 SHA-256 为 `bf892198c2ca6f7d698215b4924c7e3cfec7abe76cf79825bbe39811b0a2c88d`，原型回执位于 `target/acp-skill-coexistence/opencode-prototype.json`。私有模型配置省略 `ask` 时，OpenCode 直接执行该 stdio 调用，未向客户端发起审批。

当前生产路径仍需可约束原生工具的受审启动方式和与既有策略兼容的逐次 MCP 审批。保留现有受管制品和门禁。检查通过共享 Python 判定用例 5 项及探针语法检查。初次构建在串行锁上排队，取消后改用已有槽位网关复验。未修改 Rust，未执行 Rust 单测或桌面构建。

取得受审受限启动路径后，从仓库根按计划持有共享编译锁及槽位复跑生产探针：

```sh
python3 scripts/cargo_slot.py sh -c 'cargo build --locked -j 1 -p vw-cli --bin shiju && python3 scripts/verify_acp_skill_coexistence.py --agent opencode --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/opencode.json'
```

## 受限传输复查

共享工作区新增了 OMP 的逐次审批中继及 Cortex 的隔离 stdio 夹具。当前 OpenCode 的桌面门禁、受管入口和技能扫描仍与基线相同。首个正式网关断点与并行构建无关，本次复用已有失败证据。

使用同一固定上游 1.19.5、固定并验签的 Darwin arm64 `rg` 和私有 Seatbelt 对照。OpenCode ACP 启动时会监听自己的 HTTP 端口。禁止监听时在握手前返回 `ServeError`。限定为一个认证回环端口后可以握手。禁止启动文件枚举程序时，原生 `skill` 激活在 `posix_spawn rg` 返回 `EPERM`。最终对照只允许执行两个按摘要和码签核对的二进制，保留工作区只读、私有状态可写，并将出站连接限制在本轮回环端口。

原生 stdio 控制轮仍向 OpenCode 传入宿主定义的绝对 stdio 命令。模型收到首轮技能目录，但目标 MCP 工具缺席。没有审批或下游调用。另一路复用现有 OMP 中继，由 OpenCode 通过原生 Streamable HTTP 连接中继，后者在收到宿主单次许可后调用同一个隔离 stdio 服务。

| 中继对照 | ACP 终态 | 模型请求 | 审批 | stdio 调用 |
| --- | --- | ---: | --- | ---: |
| 基线 | `end_turn` | 3 | 无 | 0 |
| 挂载 | `end_turn` | 6 | 单次批准 | 1 |
| 撤除 | `end_turn` | 3 | 无 | 0 |
| 拒绝 | `end_turn` | 6 | 拒绝 | 0 |

挂载及拒绝结果均按调用 ID 回到模型。新会话 ID、技能和 MCP 调用 ID、入口及 `rg` 摘要记录在 `target/acp-skill-coexistence/opencode-sandbox.json`。以下命令复跑私有对照，使用原版 OpenCode 二进制及摘要匹配的 `rg`。

```sh
PYTHONPATH=scripts python3 -m acp_skill_coexistence.opencode_sandbox \
  --upstream "$HOME/.local/share/shiju/acp-runtime/opencode-1.19.5-model-source-v1/node_modules/opencode-darwin-arm64/bin/opencode" \
  --rg "$(command -v rg)" --output target/acp-skill-coexistence/opencode-sandbox.json
```

本卡以 OpenCode 已受审的原生 stdio 路径为验收条件。继续生产接线需要确认能否采用上述受限 HTTP 中继及入包 `rg` 的审定方式，或取得支持受限原生 stdio 的受审 OpenCode 制品。现有全访问门禁继续保护未受限入口。私有中继对照不产生正式网关 SSE 或模型和权限回读。

## v2 受限生产入口

2026-11-21，用户确认可使用现有回环中继作为内部传输。Darwin arm64 的固定 OpenCode 1.19.5 v2 入口 SHA-256 为 `6749a41bbb0c4625cc171705587b6e37a6f1d1068329f238efc2054f83c4b519`。安装补丁校验原始制品与三个定长锚点，重签后再次校验码签及目标摘要。

受限 Chat Completions 会话由宿主创建独立 Seatbelt 入口。OpenCode 使用认证本机端口和请求所有的 HTTP 中继，真实 MCP 下游仍为宿主选中的隔离 stdio 服务。MCP 工具过滤沿用宿主解析的 allow/deny，调用需逐次 ACP 决策。拒绝与取消都撤销调用许可。原生工作区和选中的技能来源仅允许读取。OpenCode 的附件枚举使用 Bun 原生 Glob，受限项目发现跳过 Git 子进程。Seatbelt 只允许执行固定 OpenCode，生产路径无需 Git 或 ripgrep 辅助制品。旧权限门禁仍适用于未进入受限路径的桌面会话。

原生扫描保留工作区和内置目录，追加验证过的会话视图。受限入口空选集保留原生技能。私有直连使用真实 v2 及生产 launcher，在有 `.git` 的夹具中验证同名原生技能优先。五轮退出 0。基线、挂载、撤除和拒绝为 `end_turn`，审批等待期间取消返回 `cancelled`。MCP 下游计数为 0、1、0、0、0，迟到的允许结果未发往 stdio。独立会话和调用 ID 位于 `target/acp-skill-coexistence/opencode-production-direct.json`。另以 v1 和 v2 对照旧入口，空选择和仅挂载目录相同。

```sh
PYTHONPATH=scripts python3 -m acp_skill_coexistence.opencode_sandbox --managed \
  --upstream "$HOME/.local/share/shiju/acp-runtime/opencode-1.19.5-model-source-v1/node_modules/opencode-darwin-arm64/bin/opencode" \
  --output target/acp-skill-coexistence/opencode-production-direct.json
```

第一次正式构建使用槽位 6，退出 101。同期 Pi 的模块可见性错误及 Auggie 的未完成接线阻止了 `llm-runtime` 编译。第一次定向单测用槽位 10，退出 101，实际运行零项。编译期间共享工作区加入 Auggie 和 Droid 沙箱，但该轮已编译的注册库缺少对应校验入口。两次均未进入网关探针，属于共享构建失败。

共享编译接线恢复后，在槽位 10 串行执行 `cargo test --locked -j 1 -p llm-runtime --lib acp_skill_coexistence_opencode -- --test-threads=1`。`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均指向该槽位，退出 0，实际运行 1 项且通过。复用的 OMP MCP 桥接用例 4 项及共享探针判定用例 5 项也全部通过。

首次复用正式网关曾在初始化时退出。Bun 需要在两个请求私有配置目录初始化 `.gitignore`，直连旧夹具曾把 XDG 配置放在可写的私有 home，未覆盖网关的分离布局。现按网关布局运行直连，约束目录必须属于当前 launch，Seatbelt 只额外允许创建指定配置子目录及写两个固定的私有 `.gitignore`。工作区仍只读。修正后的生产启动器直连五轮退出 0。另将同一启动策略注入隔离网关作诊断，四轮均得到一个 `chat.done`、零个 `chat.error`、MCP 0/1/0/0，确认问题定位。此诊断报告为 `target/acp-skill-coexistence/opencode-gateway-policy-diagnostic.json`。

## 正式网关复验

2026-11-21，Darwin arm64，正式探针退出 0。复用槽位 9 新建的网关，重新领取该槽位后运行验收。两个 Cargo 缓存变量均指向 `target/rust-slot-9` 的绝对路径。网关 SHA-256 为 `0991a07bf3ef6f9cf52f179979d0aee8552e611af5860a7f69b9f778d9731499`。结构化报告位于 `target/acp-skill-coexistence/opencode.json`。槽位 6 重复构建在该结果通过后取消，退出 130。

| 轮次 | 会话 ID | 模型请求 | `chat.done` / `chat.error` | MCP 决策 / 下游次数 |
| --- | --- | ---: | --- | --- |
| 基线 | `ses_e03f07f04ffejVHF3jEfyEbIBH` | 3 | 1 / 0 | 无 / 0 |
| 挂载 | `ses_e03f05cc6ffe95wqcgSJbFy3QV` | 6 | 1 / 0 | 单次允许 / 1 |
| 撤除 | `ses_e03f03421ffebwx4IrwUjuaW2i` | 3 | 1 / 0 | 无 / 0 |
| 拒绝 | `ses_e03f0124bffeffs93scVO9cDjV` | 6 | 1 / 0 | 拒绝 / 0 |

挂载轮原生技能的激活和附件读取为 `call_3bff0fb7b63c60e5aac2407b`、`call_3deaa47760793c75e6dae9e4`，选中技能为 `call_295454163af7020479c1f067`、`call_b8310c4f222ebcc5611d80a5`。MCP 允许和拒绝分别对应 `call_e446bf3e2b2d8a630de7782e`、`call_ffaafb76a5bb040f913d81ca`。四轮均回读后端 `opencode`、`session_model`、指定模型和未申请全访问的权限状态。撤除后保留原生目录，旧视图与 MCP 未列出。

Rust 定向单测 1 项通过。取消使用真实受限启动器的直连对照验证，取消后迟到的允许决策没有下游执行。验收覆盖固定版 1.19.5 v2、Darwin arm64、受限 Chat Completions 和宿主逐次审批。
