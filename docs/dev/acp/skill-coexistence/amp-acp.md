# Amp ACP 技能共存验收

## 受管 v4 失败基线

受管 v4 的首个流式模型请求没有项目原生技能目录。启用 ShiJu 选集时，宿主在推理前拒绝挂载。

## 制品与隔离

macOS 27.0 arm64，Node v24.22.3。固定制品为 `amp-acp@0.12.0`、`@sourcegraph/amp@0.0.1778732631-gc5875c`，安装修订 `amp-acp-0.12.0-model-source-v4`。从已有受审安装复制到独立私有配置目录，核对包版本、`node_modules/.bin/amp-acp` 的解析目标及以下 SHA-256。未修改受管安装。[制品契约](../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json) · [MCP 审计](../mcp-policy-audit/coexistence/amp-acp.md)

| 文件 | SHA-256 |
| --- | --- |
| Amp ACP `dist/index.js` | `d56eae64eb5cf820fd6ea982e8298cedca182a1b52fe104965ee89fa19ed40c9` |
| 控制桥 `dist/shiju-local.mjs` | `8abbb35b9ef50e087f328c1aca06aec25e4878bec902e94213857759eefb8845` |
| Worker 启动器 `dist/amp-cli` | `f3076f4f083423e1215e366e18ec7808a4405e973cfe66d0135b1bd10592a1d5` |
| Worker `dist/main.js` | `e726b67e5a1d6d1332b3726a220e2e619e5408e3922755cfd999d37fe6abd987` |

使用生产网关、Amp Anthropic Messages 中继和既有只读 stdio MCP 夹具。以项目 `.agents/skills` 为原生入口，从 `defaults.skills` 与宿主 MCP 选择生成请求。模型绑定为 `session_model` 和 `coexistence-fixture/deepseek-v4.1-flash-1017`，使用 `full_access=false`。初次冷装在槽位 3 超过 180 秒，未进入网关。正式复跑复用槽位 6 的现有网关，两种 Cargo 缓存路径均为仓库的 `target/rust-slot-6`。网关 SHA-256 为 `8ee6357b62005376c74ef75d26aa03e9379af73f81e9e8b3a63f5552ec768c52`。本次改动限于探针与回执，未重新构建或运行 Rust 单测。

## 实测结果

| 检查 | 新会话 | chat.done / chat.error | 模型流 / MCP 调用 | 结果 |
| --- | --- | --- | --- | --- |
| 正式无挂载 | `ses_e0729d223ffe8f3gC4qono3Sga` | 0 / 1 | 1 / 0 | 首个目录缺少原生技能，探针拒绝请求 |
| 独立普通 Read 对照 | `ses_e072b06a1ffetQYFqE07mkEAcL` | 1 / 0 | 2 / 0 | 原生 `Read` 经宿主读取私有附件，随机值按 `toolu_bc5200100c4fcd15fdc8f6e2` 进入续轮 |
| 独立挂载诊断 | `ses_e072afb01ffey8Fl7dEEDJP3ZG` | 0 / 1 | 0 / 0 | 网关返回 `no verified skill directory mount branch` |

独立诊断仅在无挂载轮以普通 `Read` 验证网关和模型协议。该轮回读了上表所述后端、模型来源及 `full_access=false`，读取无交互审批。挂载轮未产生 ACP 工具调用或审批。正式探针和独立诊断均退出 1。撤除与 MCP 拒绝对照未执行，未取得技能激活、共存或 MCP 计数 0/1/0 的证据。

v4 失败证据在 `target/acp-skill-coexistence/amp-acp-v4.json`，独立诊断与冷装证据分别在同目录的 `amp-acp-diagnostic.json` 和 `amp-acp-cold-install.json`。以下命令使用提交 `49de24bfe` 时的 v4 探针，`SHIJU_AMP_PROBE_RUNTIME` 指向已有受审 v4 安装。

```sh
PYTHONPATH=scripts python3 -c 'import sys; from cargo_slot import run; sys.exit(run(sys.argv[1:], number=6))' \
  sh -c 'python3 scripts/verify_acp_skill_coexistence.py --agent amp-acp --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/amp-acp-v4.json'
```

## 固定版断点

从本机 npm 缓存核对了未打补丁的固定 Worker，SHA-256 为 `458435712c9c4e222822847fc9e70080cdf023ffb3c820e959a30425a22a5836`。原生加载器 `s70` 同时处理工作区及祖先目录、用户技能、插件缓存、toolbox 和 `skills.path`。`skills.path` 只是该扫描器中的附加分支。底层扫描会跟随目录链接，读取技能正文和随附的 MCP 配置，并遍历附件。受管 v4 在扫描器入口返回空目录，所以直接传入 `skills.path` 无法注册选集。工具执行同样有独立边界，桥接白名单未包含 `skill`，不能直接放行原生激活。历史审计验证的是宿主 `Read` 和已选 stdio MCP。[Worker 补丁](../../../../patches/acp-model-source/amp-acp/apply.py) · [控制桥](../../../../patches/acp-model-source/amp-acp/local.mjs)

后续需要可审计的供应商扫描入口或受管补丁，将目录限制在项目原生技能和当次受信视图，约束符号链接与配置读取，并保留激活和附件读取的宿主审批。取得该入口后重跑正式三轮及 MCP 拒绝对照。

## 受管 v5 复验

在固定版 Worker 内恢复 `skill` 原生解析和激活，目录限制为当前工作区的 `.agents/skills` 直属技能，以及宿主为当次请求验证的选集链接。项目同名技能优先，显式禁用的项目技能也保留其名称优先级。扫描器检查规范路径、文件类型和身份，仅加载限长的 `SKILL.md`。附件继续由宿主 `Read` 路由处理。技能激活经当轮工具目录、Worker 技能目录和宿主审批检查。沿用隔离的 MCP 选择和审批；技能目录本身不注册 MCP、插件或 toolbox。

受管修订为 `amp-acp-0.12.0-model-source-v5`。校验入口和启动器的摘要与 v4 相同，新控制桥 SHA-256 为 `1d9ca86954f98dc40c789eefdc42966612049b106dca79337e36c232ef858ac6`，新 Worker SHA-256 为 `d1b64661b5fc170f1650a29bc2ffba398710656e345df56bea16adf1d4f4b02a`。在私有制品副本中用本机缓存的固定供应商入口重打补丁。未更新正式安装。

`node --test patches/acp-model-source/amp-acp/local.test.mjs` 退出 0，9 项通过。覆盖目录隔离、禁用技能、重定向链接、目录声明和独立审批。另以真实受管 ACP 入口直连协议宿主，三轮加拒绝对照均正常结束。只读 ACP 文件回调从磁盘取值，使用真实 stdio MCP 和本地 Anthropic Messages 模型响应夹具。四轮推理请求数为 3/6/3/6，下游 MCP 次数为 0/1/0/0，拒绝结果以同一调用 ID 进入模型续轮。选集轮的调用 ID 如下。脱敏证据为 `target/acp-skill-coexistence/amp-acp-native.json`。

| 操作 | 调用 ID |
| --- | --- |
| 项目原生技能激活 | `toolu_50df7f8e93903f62745ba4fc` |
| 项目原生附件读取 | `toolu_69ba6eefe93fd1b4157ddca7` |
| 选中技能激活 | `toolu_179ff9c074352e417c1a31e0` |
| 选中技能附件读取 | `toolu_d6a628fe64bec0686b88d7f6` |
| 批准的 MCP 调用 | `toolu_ad2912b3c57e2ad7337b52e3` |
| 拒绝的 MCP 调用 | `toolu_2cc0094c843a68f3c9c9885c` |
