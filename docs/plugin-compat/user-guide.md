# 本地插件与公开 MCP 使用指南

本指南对应 [来源基线](baseline.json)：2026-10-30 初始冻结，2026-11-04 经用户授权重新固定六个参考文件，依据见 [兼容报告](compatibility-report.md)。它覆盖已验证的入口，不承诺任意插件、未来 schema、三平台或真实 OAuth 账号完整兼容。

## 安装前

准备已安装的 `shiju` CLI，或识矩桌面端，以及插件声明的 Python、Node、Git 等宿主依赖。CLI 使用当前加载配置，桌面端使用其连接的 Gateway；两者配置不同会看到不同的插件目录。保留原包，不把账号令牌写入清单或脚本。

支持旧版 `.codex-plugin/plugin.json`、`.claude-plugin/plugin.json`、`.cursor-plugin/plugin.json`（按此顺序）与便携根 `plugin.json`。便携入口使用其声明的受支持 schema，固定发现 `skills/` 和 `mcp.json`；未知版本失败，不降级冒充旧版。legacy 技能递归，portable 只查直接子目录。顶层 `agents/` 不意味着自动创建代理。

公开 MCP 已有 stdio 与 Streamable HTTP 证据；portable SSE 被明确拒绝。参考初始化请求使用 `2025-06-18`，连接池的 `2026-07-28` 注释只限定其共享条件，不是所有服务的协商版本。协议来源及宿主证据分开登记于报告。

## 本地导入

从仓库根目录可使用任务 61 已验证的原包夹具体验安装（需要本机 `shiju` 已安装且指向当前版本）：

```sh
shiju plugins audit ./crates/vw-agent/testdata/plugin_compat/t61/package
shiju plugins install ./crates/vw-agent/testdata/plugin_compat/t61/package
shiju plugins list
```

真实插件将上述路径替换为原包目录。`audit` 检查清单和路径，不代表依赖、网络或工具调用已验收。安装结果会打印准确身份与目录；后续操作使用该身份，不凭显示名猜测。安装不会自动运行 onboarding 或信任 Hooks；只有用户请求时才使用提示的 onboarding 技能。

## 本地市场、更新与卸载

以下是任务 53 实际通过的命令序列。可在临时工作目录建立同形本地市场，避免覆盖已有同名插件：

```sh
mkdir -p catalog/.agents/plugins catalog/pkg/.codex-plugin
cat > catalog/.agents/plugins/marketplace.json <<'JSON'
{"name":"catalog","plugins":[{"name":"fixture","source":"./pkg"}]}
JSON
cat > catalog/pkg/.codex-plugin/plugin.json <<'JSON'
{"name":"fixture","version":"1"}
JSON
shiju plugins marketplace add ./catalog
shiju plugins marketplace list
shiju plugins install fixture@catalog
shiju plugins read fixture@catalog
shiju plugins list
```

登记的是现有本地市场目录，不是远程目录同步命令。市场条目可声明经安全物化的 local/Git/npm 来源；Git 子目录/固定 revision、npm 版本与完整性证据分别见任务 48–50。不要把远程 Git URL 直接当 `marketplace add` 的本地目录。

将例中市场原包清单的 version 改成 `2`，再执行以下更新。只改分发源，不修改安装包：

```sh
shiju plugins update fixture@catalog
shiju plugins disable fixture@catalog
shiju plugins enable fixture@catalog
shiju plugins remove fixture@catalog
```

非市场来源更新支持 `shiju plugins update ID --source PATH`，其中 ID 取自安装输出，PATH 为经审计的新包目录。更新会准备并激活事务；CLI 会直接激活，需先审查来源，想先看能力差异可使用桌面弹窗。卸载插件不等于撤销第三方账号授权，不假定所有包外数据或凭据随卸载删除。

## 桌面入口与信任

在系统设置的技能/插件页面检索插件，查看来源、版本、启用状态与依赖检查。使用安装入口填写来源，在弹窗审阅能力再确认；更新同样审阅版本与能力变化，可取消，失败可重试。详情提供启停及删除入口。任务 64 已实际验证 macOS 明暗主题、中文长路径、滚动、安装更新及取消；图示见 [平台证据](platform-evidence.md)。

Hooks 在设置的 Hooks 页面单独管理：先查看命令、参数、工作目录，再按当前声明信任或撤销，查看审计记录。包内容或声明变化后重新审阅，安装/启用不能代替信任。`prompt`/`agent` handler、观察型模型事件及 SessionEnd 的限制见 [范围说明](scope.md)，不能把跳过的 handler 计为执行成功。CLI 没有 `plugins trust` 子命令。

## 鉴权、交互与应用面板

匿名服务可直接连接；任务 62 使用锁定的 Context7 原包安装、发现并调用公开服务，记录见 [公开 MCP 证据](public-mcp-evidence.md)。服务限流或网络变化需重新诊断，不保证未来可用。

静态或环境变量形式的 HTTP 头、bearer 凭据必须通过宿主连接配置提供；包里的 auth 声明不是凭据。凭据端点要求 HTTPS（受控 loopback 除外），不转发到插件重定向目标。缺值、非法头或不支持的私有鉴权会明确失败。

OAuth 的发现、PKCE、回调、加密存储、刷新与退出已有分层证据（任务 26–28）；服务运行时发起的 elicitation 使用当前会话交互，拒绝或取消会终止对应流程。现有 `PluginCommands` 没有 `plugins login/logout` 命令，MCP 设置页的“刷新预览”也不是专用 OAuth 登录按钮。完整真实账号宿主链验收已获用户豁免，不能承诺可从设置页完成任意服务授权；遇到需要授权但没有可用交互入口时，应停止并报告该限制。详见 [OAuth 证据与豁免](public-mcp-oauth-evidence.md)。退出本地 grant 与第三方服务端即时撤销是不同结果。

MCP Apps 在真实工具返回受支持的 UI 资源时使用原生面板，执行初始化与消息桥接；可取消和关闭。只允许受审计的资源/CSP/来源，不给页面任意文件系统权限。表单、面板与真实 OAuth 各有独立验收，不能互相替代。

## 数据与依赖诊断

安装根为当前 `config_path` 父目录下的 `plugins/`；来源身份、禁用标记与安装元数据放在包外。MCP 数据位于该配置目录的 `plugin-data/<来源与身份散列>/`，`${PLUGIN_ROOT}` 指向包，`${PLUGIN_DATA}` 指向隔离数据目录，不允许插件覆盖这两个变量。OAuth grant 在配置目录 `plugin-mcp-oauth/` 加密存储并绑定所属会话/服务。派生技能、宿主状态和缓存均不得写回原包；不要通过手工移动加密文件共享授权。

依赖检查是只读探测，不执行包内脚本或 import。文件存在、解释器可找到不等于模块、远端服务或账号就绪。在 MCP 设置页使用“刷新预览”查看服务错误与工具目录；插件详情显示结构化依赖状态。缺运行时先安装插件明确声明的公开依赖，再重试；未知模块路径、动态工作目录或未验证服务保持可见未验证状态。

| 现象 | 检查与下一步 |
| --- | --- |
| 清单/路径审计失败 | 检查 schema、入口、目录穿越或符号链接；不要绕过审计、改包伪装版本 |
| 市场找不到/同名冲突 | `marketplace list`，再核对 `name@marketplace`；`read` 查看登记或安装身份 |
| CLI 有、桌面没有 | 核对配置目录和桌面连接的 Gateway；刷新目录，按会话边界更新快照 |
| 解释器或脚本缺失 | 看依赖项区分宿主可执行程序与包内文件；存在性通过后仍需真实调用 |
| HTTP 401/授权取消 | 检查宿主凭据与当前会话交互；不复制 Codex 令牌，不把 401 当授权成功 |
| Hook 不执行 | 核对总开关、插件开关、事件/handler 支持与当前内容信任，再看审计 |
| Apps 页面空白/拒绝 | 保留资源 URI、MIME、CSP/桥接错误与取消状态；不放宽来源策略 |
| 更新失败 | 保留错误及旧版身份/版本，核对新来源；不要手工覆盖已安装目录 |

反馈附上平台、版本、来源身份、脱敏错误和复现步骤，不附原始令牌、授权码、敏感载荷。需要逐项定位时查 [兼容报告的任务映射](compatibility-report.md)。

## 范围外与验收边界

OpenAI 托管连接器、私有目录/账号后端、专属运行时和私有白名单在用户排除范围内。`.app.json` 不证明托管工具可用，也不妨碍同包的独立本地技能/公开 MCP；纯托管 `.app-only` 部分不提供替代接口。macOS 之外的平台及真实 OAuth 豁免项均不计为通过；任务 66 仍需独立覆盖关闭，本文不是完全兼容声明。
