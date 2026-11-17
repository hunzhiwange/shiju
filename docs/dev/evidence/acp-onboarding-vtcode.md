# VT Code 接入实现记录

本轮按用户覆盖指令仅写代码：未运行 Cargo、单元测试、CLI、实机或最终回归；未领取 Cargo 槽位，CARGO_TARGET_DIR 未设置，未提交 Git。下面是源码调查与待运行测试，不是验收通过证据。

## 来源与依赖

- 第 16 项已在工作区提供 stakpak 注册、安装发现与目录/网关验收代码，编号文件已删除；本项复用其当前拆分架构，不改动其它接入。
- 官方源码 https://github.com/vinhnx/VTCode ，官方发行元数据 https://api.github.com/repos/vinhnx/VTCode/releases/latest ，本次读取返回 `0.182.4`。
- 固定版本文档 https://github.com/vinhnx/VTCode/blob/0.182.4/docs/guides/zed-acp.md ：入口 `vtcode acp`，环境 `VT_ACP_ENABLED=1`、`VT_ACP_ZED_ENABLED=1`。
- 发行资源列出 macOS aarch64/x86_64、Linux aarch64 GNU 与 x86_64 GNU/musl、Windows x86_64；附 sha256 文件。不将平台列表视为本机运行证据。本项不添加未经下载回读审核的自动安装映射，仅发现宿主官方 CLI，不使用同名 npm 包或移动安装脚本。

## 双重交付

- ACP：在 `acp-agent-registry` 添加稳定 key `vtcode`、本地发现及结构化 argv/env，环境仅随子进程注入。复用 `acp-client` 的初始化、会话、取消、权限回调及 MCP 序列化。
- 内置智能体：由 `agent-config::catalog::merged_agent_catalog` 派生 `acp-vtcode`；默认停用、不允许子代理，提供名称/说明/推荐提示词，默认项继承现有配置。复用设置保存与网关会话选择，不维护第二份桌面目录。
- 新增目录元数据、保存重读和后端绑定断言；网关验收通过生产配置和流入口，仅替代外部协议进程。协议测试覆盖两轮、取消后继续与拒绝宿主文件写入。均未运行。

## 能力边界及后续验证

- 保留 CLI 本地模型配置。会话模型来源仍由既有 `UnsupportedCli` 门控拒绝，不能静默回退；没有运行版本证据，不注册模型中继驱动。
- 所选技能的隔离挂载仍由既有门控拒绝；来源提及 `.vtcode/skills`、`.agents/skills` 并不证明会话隔离，因此未添加驱动。CLI 自身目录发现不受此声明覆盖。
- 完全访问仍按通用规则报告 ClientOnly/Unavailable；不把上游模式名称当作原生授权。官方文档说明本地命令执行、工具策略与持久化信任：宿主拒绝回调只证明宿主边界，不能保证 CLI 本地工具受控。
- MCP 沿用既有 session/new 配置；VT Code 实际服务消费、恢复、认证、版本变化、模型请求与工具拒绝均待统一验证，未宣称支持或上游不支持。
- 后续运行 `onboarding_vtcode` 集成目标与 `acp_onboarding_vtcode` 单测过滤（实际包为 vw-acp、config-acp-install、agent-config、vw-agent），以及真实 CLI 隔离验收。需确认有效测试数、两轮/取消/恢复、默认项持久化、身份固定及权限边界。

按用户要求删除第 17 项编号任务文件，公共 plan.md/prompt.md 保持原样。
