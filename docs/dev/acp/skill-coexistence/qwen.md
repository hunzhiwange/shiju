# Qwen ACP 技能共存验收

在 Darwin arm64 上验收 Qwen Code 0.26.7。原生入口为受审 npm 包的 `cli-entry.js --acp`，模型由 ShiJu `session_model` 绑定到本机 OpenAI Chat Completions 夹具 `coexistence-fixture/qwen3-coder-plus`。技能发现、激活、文件读取和 stdio MCP 走真实进程。验收采用 `full_access=false`。

## 修复前

旧版 `qwen-0.26.7-model-source-v1` 通过六个文件摘要校验，入口 SHA-256 为 `89c392458627a27448ffaed32102554f0cb72a70f4ec1e60527fa56068afb387`。槽位 3 的 `vw-cli` 构建退出 0，回环探针退出 1。首轮首次请求中的原生 `available_skills` 缺少项目夹具，最终 0 条 `chat.done`、1 条 `chat.error`，MCP 下游调用 0 次。反复请求属于上游失败重试。脱敏结果见 `target/acp-skill-coexistence/qwen-baseline.json`。

旧扫描器把会话视图作为唯一目录，空选择也会清空全部原生层级。修订 2 把受信会话视图追加到 Qwen 的用户级原生扫描目录末尾。项目、用户、扩展和内置层级保持供应商优先级，保留原生禁用设置、默认 ToolSearch 及现有 ACP 审批路径。Qwen 同级重复项由先扫描的原生目录优先，项目层仍优先于用户层。

## 修订 2 制品与检查

受管目录为 `qwen-0.26.7-model-source-v2`。从已校验 npm SHA-512 的原包恢复补丁输入，应用仓库受管脚本，再核对合约的六个输出摘要。入口摘要与修订 1 相同，扫描器 SHA-256 为 `6dc6bdf67a2e646a7fb86199f264d103b3cbc79777cb700cb7db1a62d2dbfeb2`。用户目录辅助模块恢复为原包内容，SHA-256 为 `8fe6c68b997e56c3dcdec5e76f4771e9a84a4b8d8a477d3d1f9e8f47322d2690`。两处 JavaScript 均通过 `node --check`。验收复制制品到隔离网关配置目录。

槽位 3 的 `CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。空选配置的回归断言在修复前失败，测试退出 101，实跑 2 项、通过 1 项。修复后定向测试退出 0，实跑 2 项并全部通过。

```sh
python3 scripts/cargo_slot.py cargo test --locked -j 1 -p llm-runtime --lib acp_skill_coexistence_qwen -- --test-threads=1
```

共存探针默认查找已安装的受管修订 2。此次用 `QWEN_COEXISTENCE_RUNTIME` 指向按合约准备的私有制品目录。私有项目对单项试验工具显式要求审批，并保留默认 ToolSearch。探针只有从完整名 `mcp__coexistence__read_probe` 的原生搜索结果取得 schema 后，才通过 `tool_call` 提交调用。Qwen 的分类器侧请求只对本探针已发出的只读调用返回匹配结果，ACP 仍单独处理审批。

```sh
export QWEN_COEXISTENCE_RUNTIME="$PWD/target/acp-skill-coexistence/qwen-archive/qwen-0.26.7-model-source-v2"
python3 scripts/cargo_slot.py sh -c 'cargo build --locked -j 1 -p vw-cli --bin shiju && python3 scripts/verify_acp_skill_coexistence.py --agent qwen --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/qwen.json'
```

## MCP 宿主接线

仅更新扫描器后的网关回合已经回读原生与选中技能正文和附件。随后 Qwen 的原生 MCP 权限请求因 `kind=other` 被未分类工作流门禁拒绝，审批未发布，下游调用为 0。脱敏结果见 `target/acp-skill-coexistence/qwen-before-mcp-approval.json`。

宿主现在对受审 Qwen 在当前活动会话发出的 MCP 请求核对协议工具标识及该会话已挂载的服务。受限模式使用该标识发布一次性宿主审批。未挂载服务、工作流确认、失效会话和仅靠标题声明的请求继续被拒绝。

`acp-client` 的 Qwen 定向测试退出 0，20 项全部通过，覆盖挂载服务的一次放行、拒绝、未挂载服务和未分类工具，以及选中技能的作用域读取边界。

```sh
python3 scripts/cargo_slot.py cargo test --locked -j 1 -p acp-client --lib qwen -- --test-threads=1
```

## 真实网关结果

2026-11-20，网关构建与复验分别退出 0。网关 SHA-256 为 `77e0305d944a592e6f5756753527358d6af45a3bd160c3b0f943fe3817c49058`。槽位 3 持有期间复用已编译网关，脱敏结构化结果见 `target/acp-skill-coexistence/qwen.json`。

| 轮次 | 新会话 | 模型请求 | `chat.done` / `chat.error` | MCP 审批 | 下游调用 |
| --- | --- | ---: | ---: | --- | ---: |
| 基线 | `ses_e082af877fferjf79SrHiCPe4a` | 5 | 1 / 0 | 无 | 0 |
| 挂载 | `ses_e082ae826ffem6YQf2O5MhbFM6` | 8 | 1 / 0 | 一次放行 | 1 |
| 撤除 | `ses_e082ad7d1ffeTBV8ONG8y75hCh` | 5 | 1 / 0 | 无 | 0 |
| 拒绝 | `ses_e082ac9d3ffetwgndKmk21sbZr` | 6 | 1 / 0 | 拒绝 | 0 |

挂载轮的原生技能激活和附件读取为 `call_c956571f3980bacfa2c6d610` 和 `call_7b220d62b248d97027af2f46`，选中技能为 `call_83035de04271541068b38465` 和 `call_68e7e088fffc11fda2beaf09`。MCP 读取 `call_d2696a554f06b54cc7c10f07` 的随机结果与两项技能附件均按调用 ID 回到模型续轮。

拒绝轮按 `call_d51a4f80acdcb85a9e82dd40` 核对 Qwen 返回的原生工具取消事件。四轮均通过模型来源和 `full_access=false` 回读。首次请求中的技能信息限于目录与描述，四轮均排除未选中的 ShiJu 技能，撤除轮保留原生读取且无法搜索到已撤除服务。修订 2 的 `cli_local` 生命周期及其他平台未复验。
