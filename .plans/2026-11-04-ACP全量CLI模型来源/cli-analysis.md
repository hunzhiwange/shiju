# 22 个 CLI 的模型接管调查

调查日期：2026-11-04。对象严格对应 `crates/vw-acp/src/agent_registry.rs` 的 22 个 key，以及 `crates/vw-config-types/src/acp_install.rs` 的安装包。本文是规划时的只读调查快照，不是运行验收报告。

## 结论与证据边界

- A：13 个，已有原生模型服务配置入口，可以优先通过子进程环境、参数或专属配置文件适配；仍需实际版本的 ACP 黑盒验收，不能把“可配置”写成“已经通过”。
- B：4 个，需要处理 ACP 适配器、独立 Gateway、原生协议或配置隔离，其中 Pi 有明确的适配器源码修改点。
- C：5 个，已查公开资料不能证明可无交互注入任意客户端 Provider/model；部分只有厂商模型目录或受限 BYOK。需要上游接口、可验证扩展或源码改造，当前不能承诺仅靠本仓库代码实现。
- 本轮未安装或启动这些 CLI，未读取用户登录凭据，未调用付费模型，未执行 Cargo 或运行测试。
- A/B/C 表示实现路线，不代表许可证、操作系统、版本或模型能力已经全面兼容。没有证据的能力一律不展示为受支持。
- “22/22 已支持”必须指同一 CLI 的真实运行时使用客户端模型成功；明确报错、切到本地配置、换成其他 Agent 都不计作支持。

## 逐项对照表

| key | 本仓库启动入口 | 分类 | 模型配置入口与实施判断 |
| --- | --- | --- | --- |
| auggie | `auggie --acp` 或 `npx @augmentcode/auggie --acp` | C | 官方 1.x 有模型选择、独立缓存目录和 pi 扩展入口，但未找到任意 Provider 的官方无交互配置契约；扩展能否接管模型和是否仍需 Augment 登录必须验证。 |
| claude | `claude-agent-acp`，包范围 `^0.26.0` | A | 独立 `CLAUDE_CONFIG_DIR`，Anthropic 地址、认证和模型环境覆盖；必须按 Agent SDK 适配器验证，不能照搬原生 `claude --settings` 参数。 |
| codex | `codex-acp`，包为 `@agentclientprotocol/codex-acp@latest` | A | 自定义 Provider、独立 `CODEX_HOME`；适配器当前 README 声明 `CODEX_CONFIG`/`MODEL_PROVIDER`。实际安装版本必须确认，不能盲传原生 CLI 的 `-c`。 |
| codebuddy | `codebuddy --acp` | A | `CODEBUDDY_BASE_URL`/`API_KEY`/`MODEL` 及轻量、推理、子 Agent 模型覆盖；原生支持 OpenAI 格式。 |
| goose | `goose acp` | A | `GOOSE_PROVIDER`/`GOOSE_MODEL`、Provider host/key 和 `GOOSE_PATH_ROOT`；核对当前版本 `acp`/`serve` 命令差异，不能顺手替换为另一种传输。 |
| vibe | `vibe-acp` | A | `VIBE_HOME` 下 TOML Provider/model 配置；可选择 OpenAI、Responses、Anthropic 等原生样式；专属配置覆盖全局和辅助模型。 |
| hermes | `hermes acp` | A | `HERMES_HOME`、`model.provider/base_url`、OpenAI key 等；必须覆盖 auxiliary、delegation、fallback 路径，不能只设置主模型。 |
| snow | `snow --acp`，安装包 `snow-ai` | A | 原生 config 有 `baseUrl/apiKey/requestMethod/advancedModel/basicModel`，源码有 `SNOW_CONFIG_DIR`；用专属 config，禁止误认同名 ServiceNow/Snowflake CLI。 |
| copilot | `copilot --acp --stdio` | A | 官方已有 BYOK：`COPILOT_PROVIDER_*` 与 `COPILOT_MODEL`；新版本 `providers.json` 优先于旧环境变量，应使用独立 `COPILOT_HOME`/`COPILOT_PROVIDER_CONFIG`。原有 GitHub token 注入不是模型 BYOK。 |
| cursor | `cursor-agent acp` | C | CLI API key 是 Cursor 用户 API key；官方参数中的 model 选择不证明可注入第三方 Provider。IDE BYOK 不能外推到 CLI ACP。 |
| droid | `droid exec --output-format acp` | B | 官方 `customModels` 能接模型，但需要确认应用专属配置根、完整 ACP 双向传输和模型 ID 契约。未证实的 `FACTORY_HOME` 不写成现成能力。必要时加启动封装或获得上游配置路径入口。 |
| gemini | `gemini --experimental-acp ...` | B | `GEMINI_API_KEY`/`GEMINI_MODEL`/`GOOGLE_GEMINI_BASE_URL`；后端仍发 Gemini generateContent 协议。网关要提供 Gemini 入站及跨协议转换，并核对模型目录、辅助请求与认证选择。 |
| iflow | `iflow --experimental-acp` | A | 官方声明任意 OpenAI-compatible；所有设置可通过 `IFLOW_` 覆盖，包括 `apiKey/baseUrl/modelName/selectedAuthType`。隔离 OAuth、设置写回和摘要模型需确认。 |
| kilocode | `npx -y @kilocode/cli acp` | A | 当前 Kilo CLI 原生自定义 Provider，`KILO_CONFIG`/`KILO_CONFIG_CONTENT`；使用 kilo 配置，不把它当同名 VSCode 插件或直接复制 `OPENCODE_*`。 |
| kimi | `kimi acp`，安装包 `kimi-cli` | A | 显式独立 `--config/--config-file` 与模型别名；Provider 支持多个接口。新旧文档类型名不同，应按固定版本生成。不能只 export 常见 API_KEY 就认为必然生效。 |
| kiro | `kiro-cli acp` | C | 公开模型目录和默认模型设置不等于任意 endpoint/key 接管。当前未确认官方第三方模型服务注入契约；需厂商接口。 |
| opencode | `opencode acp` | A | `OPENCODE_CONFIG_CONTENT`/配置路径和独立 XDG 状态；Provider、model/small_model 及 Agent 覆盖。原生有多接口适配能力。 |
| openclaw | `openclaw acp` | B | ACP 程序把请求发到已有 OpenClaw Gateway；模型由该 Gateway 配置决定。必须管理应用专属 Gateway 或修改桥接以传递会话级配置；仅注入桥接进程环境不足。 |
| pi | `pi-acp`，包范围 `^0.0.22` | B | pi 支持 models.json 和 `PI_CODING_AGENT_DIR`，但 0.0.22 适配器 `getPiAcpDir()` 固定 `homedir()/.pi/pi-acp`；需补丁隔离 ACP session-map，核对 provider/model RPC 选择与子进程继承。 |
| qoder | `qodercli --acp ...` | C | 官方 CLI BYOK 是当前账号目录驱动的交互向导，文档明确不要手写 settings.json 配置 BYOK；IDE 自定义 Base URL 不能证明 CLI 可注入任意 Provider。需要官方可编程入口。 |
| qwen | `qwen --acp ...` | A | OpenAI 认证、base URL、model 参数及独立 `QWEN_HOME`；项目 modelProviders 优先级较高，参考项目使用随机别名并由代理还原真实模型。 |
| trae | `traecli acp serve` | C | 当前官方 2.0 文档提供 `-c model=...`、`TRAE_HOME`、PAT 和 `TRAECLI_HOST`；该 host 是 Trae 服务入口，不证明是 OpenAI 模型入口。旧 trae-agent 开源项目不是此二进制。需核实原生 Provider 插件或上游接口。 |

## 各 CLI 的证据与验证关口

### 1. Auggie：C，扩展可行性尚未证实

[CLI reference](https://docs.augmentcode.com/cli/reference) 声明 1.x 基于 pi，提供 `--extension`、`--augment-cache-dir` 和模型选择；[Settings](https://docs.augmentcode.com/cli/settings) 未给出任意 Provider 的配置契约。这给出了“应用自有扩展注册模型服务”的调查方向，但不是已证实方案。先确认扩展 API 是否暴露 Provider 注册、ACP 是否读取该注册、是否能关闭厂商模型回退；有源码和请求证据才落地扩展。不能以修改 Augment 服务 URL 冒充 OpenAI 接口适配。

### 2. Claude：A，按 ACP 适配器接管 SDK

[适配器仓库](https://github.com/agentclientprotocol/claude-agent-acp)、[环境变量](https://code.claude.com/docs/en/env-vars)。已只读检查 npm `@agentclientprotocol/claude-agent-acp/0.26.0` 的 `dist/acp-agent.js` 与 `dist/settings.js`：配置根读取 `CLAUDE_CONFIG_DIR`，设置文件从该目录读取。参考项目原生 CLI 的 `--settings` 不应直接追加给 ACP 包。生成专属 settings，进程环境注入短期代理令牌，主模型/快模型/子 Agent 全部明确映射。必须验证 SDK 的项目 settings 优先级及是否仍访问原有 OAuth。

### 3. Codex：A，现有适配器配置入口优先

[OpenAI 官方高级配置](https://learn.chatgpt.com/docs/config-file/config-advanced)、[codex-acp README](https://github.com/agentclientprotocol/codex-acp)。本轮 npm 元数据 latest 为 2.5.0；README 是可变 main，不代表已安装版本。当前 README 列出 `CODEX_CONFIG`、`MODEL_PROVIDER`、`CODEX_PATH`；Provider 使用 `base_url/env_key/wire_api`，模型上下文限制可明确配置。先固定适配器与其底层 Codex 版本；选官方已有入口，无法透传时才做适配器补丁。使用独立 CODEX_HOME，不复制参考项目的全局最小窗口，而取所选模型的实际窗口。

### 4. CodeBuddy：A，官方环境变量覆盖

[环境变量](https://www.codebuddy.ai/docs/cli/env-vars)、[models.json](https://www.codebuddy.ai/docs/cli/models)。地址和模型可通过进程环境覆盖；模型文件的 URL 要完整 `/chat/completions`，环境 BASE_URL 的语义必须分别验证。覆盖 lite/reasoning/subagent 等分支，确认 ACP 不启动登录向导，确认全局及项目配置不会抢路由。目录隔离若该版本缺少入口，只能加无侵入封装或转为 B，禁止直接改用户 models.json。

### 5. Goose：A，原生 Provider 与路径根

[官方环境变量源码](https://github.com/aaif-goose/goose/blob/main/documentation/docs/guides/environment-variables.md)。有 Provider/model/host/key 及 `GOOSE_PATH_ROOT`；隔离其 config/data/state。不同发布版的 `goose acp`、`goose serve` 命令须按当前入口验证 stdin/stdout JSON-RPC，不因为文档出现 serve 就切换成远程服务。子 Agent recipe 可能优先于环境，需验证并限定当前模型路线。

### 6. Vibe：A，专属 TOML

[配置](https://docs.mistral.ai/vibe/code/cli/configuration)、[配置参考](https://docs.mistral.ai/vibe/code/cli/configuration-reference)。`VIBE_HOME` 可隔离配置和状态；Provider 定义 api_base、api_key_env_var、api_style，model 定义 name/provider/alias。所有辅助 Agent 的 active_model 应遵守会话绑定，能力不匹配时错误而非使用内置 Mistral 模型。

### 7. Hermes：A，主/辅助模型共同接管

[环境变量](https://hermes-agent.nousresearch.com/docs/reference/environment-variables)、[运行时解析](https://hermes-agent.nousresearch.com/docs/developer-guide/provider-runtime/)、[配置](https://hermes-agent.nousresearch.com/docs/user-guide/configuration/)。主、ACP、辅助任务共享运行时解析，但 auxiliary/delegation/fallback 可以有独立来源，须在专属 HERMES_HOME 显式设置并验证。配置一个 OPENAI_BASE_URL 不能自动证明全部 LLM 请求遵从会话选择。

### 8. Snow：A，识别正确包和配置根

[Snow 官方仓库](https://github.com/MayDay-wpf/snow-cli)、[配置说明](https://github.com/MayDay-wpf/snow-cli/blob/main/docs/usage/en/02.First%20Time%20Configuration.md)。已读取提交 `a9eba5f9062afbf639829b4992e90a986924f519` 的 `source/utils/config/envConfig.ts`、`apiConfig.ts`、`source/utils/acp/acpManager.ts`：有 `SNOW_CONFIG_DIR`，配置含 advancedModel/basicModel，ACP 执行读取高级模型。生成专属 JSON，清理 OAuth 元数据和其他模型路线，验证 requestMethod 及目录覆盖确实适用于所装版本。

### 9. Copilot：A，BYOK 与 GitHub 登录是两件事

[BYOK 文档](https://docs.github.com/en/enterprise-cloud%40latest/copilot/how-tos/copilot-cli/customize-copilot/use-byok-models)、[配置目录](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference)。当前官方文档同时存在 providers.json 和旧 COPILOT_PROVIDER_* 两代入口；providers.json 有声明时优先于旧环境。通过明确版本选择一种契约；本地 Provider 配置不能盖过本次会话；ACP 权限流程照旧。GitHub key 不能填入自定义模型认证位置。

### 10. Cursor：C，用户 API key 不等于模型 API key

[CLI 认证](https://cursor.com/docs/cli/reference/authentication)、[CLI 参数](https://cursor.com/docs/cli/reference/parameters)、[IDE BYOK](https://cursor.com/help/models-and-usage/api-keys)。CLI 文档认证 key 来自 Cursor Dashboard；本轮未确认可无交互设置模型 Provider URL/key。旧文档中的 endpoint 也不能证明接受 Chat Completions，必须核对线协议。没有官方契约时需要厂商接口，不能用通用 HTTP 代理、TLS 解密或把另一个 Agent 命名为 Cursor 来实现。

### 11. Droid：B，模型支持明确，隔离和 ACP 路径仍有缺口

[BYOK](https://docs.factory.com/model-independence/byok)、[设置](https://docs.factory.com/droid-cli/settings)、[企业约束](https://docs.factory.com/enterprise/hierarchical-settings-and-org-control)。支持 customModels、环境引用及多 Provider 样式，但自定义模型运行 ID 在不同文档示例有差异；需根据实际目录/输出确定。必须找到官方隔离配置路径或建立受支持启动封装；不能假设存在 FACTORY_HOME。先验证现有 `exec --output-format acp` 真正支持本仓库的双向 ACP 交互，再谈切模恢复。企业 policy 禁止 BYOK 时不得绕过。

### 12. Gemini：B，协议网关工作为主

[官方配置](https://geminicli.com/docs/reference/configuration/)。支持 Google Gemini base URL 与 API-key 路径，但并不等于发送 OpenAI 请求。优先为中继实现 generateContent/streamGenerateContent、工具函数调用及模型元数据请求，接到客户端 Provider；必要时给 Gemini ACP 层补充任意别名与会话模型设置。后者必须由固定版本代码证明，不能先宣称必需 fork 整个 CLI。覆盖压缩、辅助模型、默认 model 校验，Vertex/OAuth 不得抢回认证。

### 13. iFlow：A，环境覆盖有明确文档

[固定提交设置文档](https://github.com/iflow-ai/iflow-cli/blob/4642808afbc6580ac117d930f6c64ac0d84955c7/docs_en/configuration/settings.md)、[README](https://github.com/iflow-ai/iflow-cli)。官方声明所有设置可经 IFLOW_ 前缀覆盖，优先于系统/项目/用户配置，并支持任意 OpenAI-compatible。用 camelCase 规范键避免多个别名互相覆盖。覆盖 selectedAuthType、摘要及辅助模型；如果进程会自动写入本地凭据且无专属根，则先补可控启动入口，不能用 HOME 替换规避。

### 14. Kilocode：A，原生 Provider 配置

[CLI](https://kilo.ai/docs/code-with-ai/platforms/cli)、[自定义模型](https://kilo.ai/docs/code-with-ai/agents/custom-models)。有 `kilo acp`、provider 模型表、可信配置路径 `KILO_CONFIG/KILO_CONFIG_CONTENT`；本轮 npm latest 7.8.18 仅作调查锚点。必须核对安装包 bin 和本仓库候选 `kilocode` 是否相符，保留注册 key，不顺手增加名称别名。生成与版本相符的 Provider 配置和环境引用，明确模型/small_model/Agent 覆盖。

### 15. Kimi：A，固定版本的配置文件优先

[Provider](https://moonshotai.github.io/kimi-cli/en/configuration/providers.html)、[新文档 Provider](https://moonshotai.github.io/kimi-code/en/configuration/providers)、[配置文件](https://moonshotai.github.io/kimi-code/en/configuration/config-files)。两代文档中 openai_legacy/openai 等类型不同；新文档明确普通 API_KEY 不自动从进程环境取值，而 api_key_env 或临时 KIMI_MODEL_* 是显式入口。以所装版本为准生成独立配置并传 `--config`，不要混用两代字段。关闭内置 Kimi 服务模型回退，保留非模型工具的原权限。

### 16. Kiro：C，上游任意模型接口未确认

[模型文档](https://kiro.dev/docs/models/)、[CLI 设置](https://kiro.dev/docs/cli/reference/settings/)。可选厂商模型和 chat.defaultModel 只是厂商目录选择；文档未证明注入任意 Provider/key。不能用 HTTP_PROXY、默认模型 ID 或 Bedrock 配置类推得出可接管。若厂商新增 harness/provider API，先做最小本地请求证明，再加入接管；否则该项保持明确阻塞，CLI 本地来源仍可用。

### 17. OpenCode：A，成熟配置入口

[Config](https://opencode.ai/docs/config/)、[Providers](https://opencode.ai/docs/providers/)。生成专属 Provider、model/small_model、环境引用和 XDG 状态，内联配置不放真实上游密钥。项目配置、Agent frontmatter 模型及 compaction 分支可能影响路由，必须用冲突 fixture 验证。

### 18. OpenClaw：B，配置必须到执行推理的 Gateway

[ACP bridge](https://docs.openclaw.ai/cli/acp)、[Provider](https://docs.openclaw.ai/concepts/model-providers)。文档明确 `openclaw acp` 是 stdio→WebSocket Gateway bridge。优先由网关主机管理应用专属 OpenClaw 实例，配置客户端模型中继；如果采用已有用户 Gateway，就必须有经鉴权的会话级覆盖且不能改全局模型。后者未证明前不复用用户实例。生命周期包含 bridge 与 Gateway 两个进程，取消、恢复、模型来源命名空间都必须验证。

### 19. Pi：B，明确需要适配器隔离补丁

[实际 npm 包对应适配器](https://github.com/svkozak/pi-acp)、[固定版本路径源码](https://github.com/svkozak/pi-acp/blob/v0.0.22/src/acp/paths.ts)、[Pi 模型配置](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/models.md)。只读 npm 检查确定 0.0.22 的仓库是 svkozak，不是另一个同名实现。getPiAcpDir 固定用户主目录，需增加明确的应用专属 ACP 状态目录入口，原生 pi 用 PI_CODING_AGENT_DIR。将 Provider/model 选择映射到 RPC set_model，确认独立目录中的模型可枚举，不把 --model 假定为适配器透传参数。

### 20. Qoder：C，受限 BYOK 不是任意模型注入

[CLI BYOK](https://docs.qoder.com/cli/custom-models)、[CLI 参数](https://docs.qoder.com/cli/cli-reference)。CLI 文档明确以当前账号 Custom 向导目录为准，不要手写 settings.json 配置 BYOK。`--model` 可以选 BYOK key，但不能创建任意 Base URL/key/model。中国版 IDE 的 custom models 页面属于另一产品面。必须取得官方程序化 Provider 注册接口或已验证 SDK 入口；保留账户/企业限制。

### 21. Qwen：A，借鉴参考项目的冲突隔离

[模型 Provider](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/model-providers/)。modelProviders 的优先级和原子覆盖意味着仅加 OPENAI_BASE_URL 可能无效。参考项目给模型加随机别名并在代理还原，避免命中项目同名 Provider。本项目应使用同样目的的会话私有别名与唯一 Provider 身份，明确把上下文、输出限制、能力和思考配置传入适用层。独立 QWEN_HOME/运行态以固定版本源码核对。

### 22. TRAE：C，服务 host 不能当模型 endpoint

[ACP](https://docs.trae.cn/cli_agent-client-protocol)、[配置](https://docs.trae.cn/cli_config-file)、[环境变量](https://docs.trae.cn/cli_environment-variables)。已明确全局 -c 必须在 acp serve 前；TRAE_HOME 能隔离，TRAECLI_PERSONAL_ACCESS_TOKEN 是平台 PAT，TRAECLI_HOST 只标为 API base URL，没有证据表明是标准 LLM 接口。需要官方 Provider 配置或模型 RPC；不能把开源 trae-agent 的配置能力归给 traecli。

## 全量目标的真实边界

1. 默认客户端模型与 CLI 本地配置两个来源，对全部 22 项都明确建模；不支持默认来源的 CLI 展示原因，用户可以显式选择本地来源，不能自动回退。
2. A 项优先实现原生配置；B 项完成对应适配器、封装和协议工作；C 项逐一取得上游能力证据。22/22 的最终验收依赖 C 项上游问题解决。
3. 如果 C 项无法提供入口，当前仓库内的完整实现仍不足以达成“22 个实际 CLI 都用任意客户端模型”。应报告具体阻塞，不能把显式拒绝计为已支持。
4. 本地配置是 CLI 执行主机的配置；远程 Gateway 时不是桌面用户电脑的配置。本地模型服务也必须从 CLI/Gateway 主机可达。
