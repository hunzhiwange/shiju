# 任务35：上游关口复验

2026-11-05，Darwin arm64。五项均未取得完整双来源验收；缺少共享脚本配方的 unavailable 报告不是阻塞的最终依据。没有借用用户账号、模拟成功登录、替换 Agent 或调用付费模型。

| CLI | 真实证据 | 无法仅靠本仓库集成解除的关口 |
| --- | --- | --- |
| Auggie | 本轮重跑固定 0.36.0 与 1.x 四项探测；[报告](auggie/upstream-gates-35.json)，模型请求0 | 0.36.0 拒绝扩展参数；1.x 已加载的扩展明确拒绝 Provider 注册；Pi 配置替代也失败。需要正式 Provider 入口或适用的修改授权及源码，不能移除厂商限制。 |
| Cursor | 已下载的 2026.10.22-27dcf5f `--version`、普通/ACP/Bedrock help 和真实 ACP loopback 探测；[报告](upstream-gates-35.json) | `--endpoint`/`CURSOR_API_ENDPOINT` 发出 `/auth/exchange_user_api_key` 和两个 `aiserver.v1` 控制面 RPC；不是 Chat/Messages/Responses/Gemini 模型端点。session/new 返回 -32603，模型请求0。需要正式任意 Provider 契约及平台授权；不能伪造成功的账号交换或托管服务。 |
| Droid | 真实1.3.0 双向 ACP initialize成功，指定 custom model 的 session/new 返回 -32000/authentication required；[报告](upstream-gates-35.json) | 自定义模型入口已存在，ACP 不是单向输出；当前门槛是获授权的 Factory 登录/API key。不能把模型 key 充当平台 key。专属目录候选已写入临时目录，但完整隔离/恢复仍需登录后验证。 |
| Qoder | 原任务30恢复调查：真实1.1.82已安装，官方SDK公开BYOK CRUD，原生路径先检查个人登录及Custom Provider权限 | 需要获授权的真实个人账号/权限或正式免平台认证契约。不能继续称为“没有程序化入口”，也不能使用内部开关伪造权限。 |
| TRAE | 原任务32恢复调查：真实0.121.3可加载、枚举、ACP选择自定义模型；两条候选 session/prompt 均失败且各模型请求0；PAT探测调用平台ExchangeToken | 需要获授权的登录/PAT及该版凭据存储；仍缺专属运行时根隔离。`TRAE_HOME`在此版本未证明有效，不能套用2.0文档或模拟成功登录。 |

Cursor 当前官方[认证说明](https://cursor.com/docs/cli/reference/authentication)要求从 Cursor Dashboard 获取用户 key。固定二进制的 `bedrock --help` 提供存储凭据、启用和后端验证命令，不构成任意模型地址入口。API endpoint 本地替换实验中的3个 fixture错误是对不支持控制面请求的真实拒绝，不能视作模型服务故障或成功接管。

Droid 的[设置文档](https://docs.factory.com/droid-cli/settings)确认原生配置路径；本轮使用 `FACTORY_HOME_OVERRIDE` 私有目录及 `FACTORY_DISABLE_KEYRING=1`，未读取真实用户账号。所测二进制 SHA256 与已下载官方 `.sha256` 相符。Cursor报告记录启动脚本摘要及实际版本；不把脚本摘要当成完整归档校验。

复验命令（只运行已下载真实发行物，不安装、不调用Cargo）：

```sh
python3 docs/vibewindow/acp-model-source/evidence/auggie/probe.py --output target/task35/auggie-upstream-gates.json
python3 docs/vibewindow/acp-model-source/evidence/probe_upstream_35.py --output target/task35/upstream-gates.json
```

两条命令退出0表示调查完成，不是来源验收通过。Cursor/Droid进程在观测关口后清理；Cursor退出143是主动SIGTERM，Droid退出0不改变其session/new认证失败。报告不保存账号交换正文、设备码、模型key、原始供应商输出或个人路径。
