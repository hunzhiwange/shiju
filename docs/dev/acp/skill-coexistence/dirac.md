# Dirac 0.5.24 技能共存验收

2026-11-20，Darwin arm64。受管修订 `dirac-0.5.24-model-source-v2` 消费 `session/new._meta["shiju.skillMounts"]`。Dirac 使用原生项目扫描器追加当前会话视图，并继续使用 `use_skill` 和 `read_file`。视图按会话绑定，同名技能显式拒绝。挂载限新 ACP 会话和 `session_model`，恢复与分叉入口拒绝挂载。MCP 沿用受审绝对路径 stdio 及逐次宿主审批。

| 制品 | SHA-256 |
| --- | --- |
| npm 0.5.24 `dist/cli.mjs` | `c88e043f590a520da3345050e32bbe6930a68cefad60945ef4a7d14648217429` |
| v1 `dist/shiju-dirac.mjs` | `330dc9f7ad7bd68b2255bea99410daab3b22ed70605e9177001fd1cbfd09797d` |
| v2 `dist/shiju-dirac.mjs` | `abbc5f72264b5bf4da8692a93bdcc27e4490d95a9a230632885d77da8fed2096` |
| v2 `dist/session-mcp.mjs` | `4e2362bfa6661564693b06ddd0eb85bc4d5edc1e6720ea758589c1234e6455ad` |

## 修复前

槽位 10 的既有网关与 v1 制品运行同一探针。基线完成一个 `chat.done`，零 `chat.error` 和零 MCP 调用，三个模型请求均走 Chat Completions，回读 `session_model`、`coexistence-fixture/selected-model`、`full_access=false`。原生激活及读取模型调用 ID 为 `call_4226ddf678ebc9d02ce216da`、`call_5799ee94e52197d96afc0390`。挂载轮在宿主能力门禁报 `no verified skill directory mount branch`，模型请求及 MCP 调用均为零。证据位于 `target/acp-skill-coexistence/dirac-red.json`。

## 受管制品直连

直接启动 v2 制品，在同一隔离进程依次创建基线、挂载、同数量技能替换、撤除和拒绝会话，五次均返回 `end_turn`。模型请求数为 3/6/6/3/6，MCP 读盘次数为 0/1/1/0/0。替换轮确认旧技能不可发现。Dirac 原提示缓存只按技能数量区分，未加入会话标识的补丁在该轮误回显旧目录。三个带挂载的会话均收到五项已关联的模型调用结果，尝试以空 MCP 列表加载时明确拒绝。模型夹具声明推理能力，以接受 Dirac 在实际请求中发送的 `reasoning_effort=medium`。

## 网关回归与定向测试

首次定向 Cargo 测试受并行开发的 Grok 模块 `Sha256: std::io::Write` 编译错误影响，退出 101。该错误修复后，复用槽位 3 生成的 `llm_runtime-85f23fc8970681f8` 测试产物，按 `acp_skill_coexistence_dirac --test-threads=1` 执行，退出 0，1 项通过、514 项过滤。测试产物 SHA-256 为 `3447807cbd2ed8e36ea5cabc691c960e47cd81cb9c8de1220323f71236f1e052`。

同一工作区的标准网关构建已在 [Grok 回执](grok.md) 记录退出 101。当前 `task-scheduler/src/store/postgres/task_plan.rs:247,249` 仍访问私有字段 `PlanMember.id`，触发 E0616。Dirac 停止了等待 Cargo 共享锁的重复构建。新版网关的四轮结果、模型与权限回读和 `target/acp-skill-coexistence/dirac.json` 待正式网关构建恢复后验证。

## 复验

从仓库根在私有目录准备固定 npm 包。探针校验 npm 锁记录的版本及完整性、上游入口、补丁入口和辅助文件摘要。验收使用隔离配置和私有回环模型。

```sh
umask 077
npm install --prefix target/acp-skill-coexistence/dirac-npm --ignore-scripts --no-audit --no-fund --save-exact dirac-cli@0.5.24 @modelcontextprotocol/sdk@1.32.2
python3 scripts/cargo_slot.py sh -c 'cargo build --locked -j 1 -p vw-cli --bin shiju && python3 scripts/verify_acp_skill_coexistence.py --agent dirac --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/dirac.json'
```

## 2026-11-21 正式网关回归

在 Darwin arm64 持有 Cargo 槽位 6 时复用共享网关。构建提交为 `03e5fc747ed4571184fbb6a156bb48d093b9fee0`，源码树为 `d05a1a28464c987967ad7e08792b2139bf02f9a5`。构建范围内的工作区差异摘要与共享回执一致，变更仅为其他后端的声明和分派顺序。网关 SHA-256 为 `c59878516f6a88bb8f60ce78955654b9859bca9b82af6711fc14c83d0a572eba`。探针校验 Dirac 0.5.24 的上游入口及两份 v2 文件摘要，均与上表一致。

两个 Cargo 缓存路径均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-6`。网关探针退出 0，四个独立会话各有一个 `chat.done` 和零个 `chat.error`。每轮均回读 `dirac`、`session_model`、`coexistence-fixture/selected-model` 与 `full_access=false`。

| 场景 | 会话 | 原生激活及附件调用 ID | 选中技能激活及附件调用 ID | MCP 模型调用 ID | 审批与执行数 |
| --- | --- | --- | --- | --- | --- |
| 基线 | `ses_e05f87674ffe1kQUGXgSkeUHpM` | `call_9bf6e46de461f841fdc74fd9`、`call_a76216791358730c14f7dada` | 不可发现 | 无 | 无 / 0 |
| 挂载 | `ses_e05f86a20ffe3YdIH2e9Rpn3or` | `call_4a1b757bbe50006101e20de3`、`call_10edaf7c59e7aad01887ed79` | `call_ac9fd36f9332d393ea285398`、`call_0c214457f951bb094e3a7d95` | `call_f67fa1755de215b14d784af4` | 单次批准 / 1 |
| 撤除 | `ses_e05f85a51ffe092KEIwC5d4EYo` | `call_4b208b8ab7d137781ee6192a`、`call_63f4c089ab62b1504e01b05b` | 不可发现 | 无 | 无 / 0 |
| 拒绝 | `ses_e05f84bdaffepsQDqU6e7k3uFi` | `call_9c53d005c893b98283775405`、`call_921054364be26821256613aa` | `call_4bbe35a859d56dc8acac92a1`、`call_46947574ece8159b77df4678` | `call_a0e883bd5a5f7f6810f47377` | 拒绝 / 0 |

模型收到实际读取的正文和附件。挂载轮收到实际 MCP 读盘结果，拒绝轮收到对应调用 ID 的拒绝结果。结构化证据位于 `target/acp-skill-coexistence/dirac.json`。
