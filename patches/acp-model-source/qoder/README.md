# Qoder 1.1.88 私有会话模型入口

固定 npm 制品只在独立受管目录打补丁，全局 Qoder 安装及账号状态不变。
没有 `SHIJU_QODER_MODEL_CONFIG` 时仍进入上游原有 Qoder ACP 框架。

显式私有绑定选择同一 bundle 内的本地 ACP 框架 `P7e/ahe`，保留
`e5/O1` 会话执行、原生 AgentLoop、工具与权限控制。没有引入其他引擎，
不使用 fakeResponses，不构造账号身份，也不让官方 BYOK 权益检查恒真。
云端 Qoder 模型 Provider `Tk`、账号 ACP 层 `FVt` 和官方服务插件不会在该分支构造。

Rust 请求拥有者写入 0700 目录内的 0600 绑定文件和 settings.json，传入会话引用、
当前选定模型、回环中继端点和短期令牌；供应商 API 密钥不传给 CLI。
只接受固定 ACP 参数与该私有配置根，忽略项目配置，只注册绑定的一个本地 Provider。
每次准备推理再次检查绑定与模型；删除私有配置或撤销中继令牌后不能继续推理。
原生 `mode` 控制仍由网关根据当前完全访问授权同步，可恢复为 default。

入口初始化前限制 Node TCP 连接为该会话的准确回环端口，并拒绝外部 `dns.lookup`。
这也阻止上游启动时的后台遥测/HTTPDNS/更新网络尝试，不将官方服务模拟为可用。
原生工具需要网络时也可能明确失败；当前验收只覆盖本地文件编辑。
历史位于请求私有配置根，本轮不承诺跨进程恢复。

`patch.json` 固定上游 bundle 哈希与唯一工厂锚点；受管安装契约校验 npm
归档完整性、补丁后 bundle 和绑定模块哈希。当前仅审查 Darwin arm64。

边界验证：

```sh
node --test patches/acp-model-source/qoder/session-model.test.mjs
```

真实 ACP initialize/session/new 和真实模型 HTTP 请求证据独立于这些边界测试；
探针的 HTTP 503 只验证到达本机端点，不作为指定模型成功或编辑器产物证据。
