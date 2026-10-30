# Codex 插件兼容计划

目标：在冻结的 Codex 行为基线上，让本地与公开 MCP 插件原包导入后具备一致的可观察行为；逐项证据齐备才宣布完全兼容。

范围：旧版与便携清单、技能与命令及实际受支持的代理声明、公开 MCP 传输与鉴权/交互/应用界面、Hooks、来源身份、安装更新与本地/Git/npm 市场、CLI/Gateway/桌面入口、会话刷新、安全诊断与跨平台验收。以原包不改写、运行可验证为完成标准，覆盖目标版本的全部范围内字段与执行分支。

范围外：用户已明确排除 OpenAI 托管连接器、私有目录、账号后端及专属托管运行时；不复制登录令牌，不改品牌，不承诺未知未来版本或所有模型输出逐字一致。

假设：VibeWindow 已有 plugins、skills、rmcp、Hooks、Gateway 与 Iced 插件页；codex-main 为无 Git 元数据的本地源码快照，可按文件散列冻结。原包仍需其公开声明的外部可执行依赖及合法账号授权。

## 设计决策

- 选择现有模块上的兼容适配；整套移植耦合过重，安装时改包破坏原包兼容。安全策略优先，差异必须公开且评审，不伪称完全一致。
- 完全兼容限定为用户确认的本地/公开 MCP 范围及冻结基线；新增规范不得静默扩大目标。

## 架构说明

- 清单归一化→路径审计→来源身份→事务安装→技能/MCP/Hooks 适配→会话快照；入口复用既有 CLI/Gateway/Iced。
- 应用界面与凭据隔离，原包只读、状态数据分离，所有执行沿宿主审批。

## 开发策略

- 对会改变行为的代码使用失败验证—通过验证—必要重构；先证明失败来自目标差异，再实现最小行为。
- 原有独立测试文件继续保留；新增测试显式注册，过滤后有效测试数不能为零。
- 每个编号任务围绕一个窄行为，约一个聚焦会话；依赖未齐不得启动。新发现独立能力不得塞入既有任务无限扩大。
- 编号任务是唯一进度来源；本文件生成后只读。步骤、依赖和执行提示分别在编号文件与 prompt.md。
- 共享源码接线频繁涉及 plugins/mod.rs、内部清单类型、MCP 连接、会话生命周期与 UI 状态；本计划保守串行，每个批次仅一个任务。独立 Cargo 槽位不证明源码可并行。
- 仓库已有其他计划与 staged/unstaged/untracked 工作；不重置、不改动、不提交无关内容。执行前复查 git status 与文件占用。

## 成功标准

- 清单正例及所有支持形态可经真实入口加载；非法形态与基线拒绝行为有明确结果，不能只验证 serde 成功。
- 锁定来源的原包在安装/运行/升级后字节散列不变；技能、公开服务与 Hooks 产生可证明的真实行为。
- 基线覆盖表每行有源码/规范依据、对应任务、独立失败测试、通过测试与当前证据；范围内没有被悄悄排除的必需依赖。
- 缺失宿主工具的包显示依赖不可用；诊断本身不等于该包完全兼容。若属于范围内必需能力则阻止最终验收并拆独立补充任务。
- 未实际使用的配置面不新增；Codex 只解析但不执行的声明不据枚举推断支持，严格对照实际路径。
- 权限、超时、取消、内容信任、路径/CSP 边界和日志脱敏有效；不复制 Codex 私有例外名单。
- macOS/Linux/Windows 与公开 OAuth/Apps 的实际证据分别记录；缺环境保持待回归，不可用外部前提保持阻塞。
- 允许单任务提交为“已完成，待回归”，但最终兼容验收不允许残留范围内待回归或阻塞项。

## 源码与规范基线

- VibeWindow 规划时 HEAD：`e776003397c032a4e17a3640acf5b4a74adf3e1c`；工作区存在其他任务变更，因此执行时以现有内容复核。
- Codex 参考根：`/Users/Shared/work/dir/data/codes/codex-main`；该目录没有 Git 元数据。以下 SHA-256 防止误用漂移快照；若改变，先核对差异并询问是否更新来源基线，不能改本文件。
  - `codex-rs/core-plugins/src/manifest.rs`：`cb6cc5e8e2004d1533141a3f11f3460a1aa8c2e0295c9583ae9726c6a75b2802`
  - `codex-rs/core-plugins/src/agent_plugin_manifest.rs`：`31f9efd68ad2a94e45d1596123d49066886c4592aa7588935170913ced1cc660`
  - `codex-rs/codex-mcp/src/agent_plugin_config.rs`：`08a59340a5264849033ffaf0945152cadf09fb8ca98ca7ee44727e7e7918954a`
  - `codex-rs/core-plugins/src/app_mcp_routing.rs`：`1a4b2517031f60faba87d35877b385c1406e5fcf9745c0cd87ab16415a8d2859`
- [OpenAI 插件打包规范](https://developers.openai.com/plugins/build/plugins)：根 plugin.json、mcp.json、skills/ 与 com.openai overlay；旧 .codex-plugin/plugin.json 保持支持。
- [OpenAI 插件架构](https://developers.openai.com/plugins/concepts/plugins)：技能、MCP 服务、生命周期与可选应用界面。
- MCP Apps 等外部规范在对应实现任务开始前核对其官方规范版本，并登记到 baseline.json；不把变动的网页当固定协议。
- 现有代码参考：crates/vw-agent/src/plugins/、tools/mcp_common/、hooks/；crates/vw-gateway-client/src/client/desktop_settings_api.rs；crates/vw-desktop/src/app/components/system_settings_skills/plugins.rs。
- 已核查 Cargo workspace 与 vw-agent/vw-acp/vw-desktop 配置；vw-agent 的 autotests=false，行为测试走 --lib，桌面走 --bin shi-ju，不以未注册 tests/ 文件取得零测试成功。

## 验证约束

- 本轮只生成计划，没有改生产代码，不运行 Cargo；实施任务在各自编号文件遵守共享槽位。
- Rust 修改后先 `cargo fmt --all`，审查并排除无关格式差异，再运行带过滤的行为测试和 `cargo clippy --all-targets --all-features -- -D warnings`。
- 不运行无过滤的全量 cargo test、fmt --check、wasm 检查或 Makefile 命令。
- 真实服务的网络/账号不可用不能放宽断言；确定性测试只替换网络、provider 和用户响应边界，安装/解析/进程/权限仍走真实实现。
- 最终证据保存在 docs/plugin-compat/，不要写入只读 plan.md；最终报告保留命令、退出码、测试数量、平台、实际槽位和绝对 CARGO_TARGET_DIR。
