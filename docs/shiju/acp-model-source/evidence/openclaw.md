# OpenClaw 2026.10.10 模型来源验收

2026-11-05，Darwin arm64，Node v26.13.0，官方 npm 包 `openclaw@2026.10.10`；实际版本为 `OpenClaw 2026.10.10 (7dc3350)`，包 integrity 与版本清单一致。未使用替代 CLI、真实账号或付费模型；只模拟不可确定的外部模型服务。

## 生产实现

`model_source/drivers/openclaw.rs::configure` 为客户端来源生成独立配置，连接本仓库 Chat 中继；主模型、辅助代理模型和 imageModel 引用均指向绑定 Provider/model，fallbacks 为空。模型中继继续校验当前模型状态、客户端凭据、身份、过期和撤销，不进入另一层会话工具循环。无法转换的请求由现有中继明确拒绝。

ACP stdio 仅为 WebSocket Bridge；真正推理发生在 profile 所有的前台 Gateway。`model_source/gateway.rs::OwnedGateway` 管理启动、就绪等待、独立进程组及 kill/wait 回收；profile 先撤销中继令牌，再回收 Gateway，最后删除私有文件。Bridge 沿用现有 ACP 客户端进程所有权。启动期间取消会丢弃准备中的 profile；运行期间取消仍经过原生 ACP cancel。

Gateway 仅绑定随机回环端口；Bridge 总是显式传入该 URL、独立令牌文件及会话 key，不复用用户 Gateway、远程地址或全局模型。OpenClaw 不支持继承监听 socket，因此启动前释放端口预约；端口竞争或认证失败不会回退其它实例。没有使用 `--force`、服务安装或用户 HOME 替换。

Gateway 环境从白名单重建，不继承账号、代理和注入选项。配置/状态使用 OpenClaw 原生 `OPENCLAW_CONFIG_PATH`、`OPENCLAW_STATE_DIR`；临时目录0700、配置及令牌文件0600。模型令牌通过环境变量引用，argv 中只有 URL 和文件路径。OpenClaw 会生成模型缓存，因此其状态、认证和模型缓存位于请求临时目录；只把会话存储放入按租户/项目/会话/来源散列隔离的持久目录。重新请求轮换令牌和运行目录，保留相同来源的历史；切换来源使用不同会话锚点。

关闭非必要插件、浏览器、控制界面、发现和心跳，不扩大工具允许列表。该版本通过惰性工具发现提供 `openclaw:core:sessions_spawn`，fixture 经真实 `dispatch_action` 执行它，并观察原生辅助代理读取两个独立文件及请求模型。非空会话级 MCP servers 被上游明确拒绝；本任务没有伪造透传或放宽该约束。

`cli_local` 完全绕过 managed profile，保留原命令、参数和环境。验收脚本只为该来源启动并拥有一个原生临时 Gateway，模型和凭据由其本地配置决定；客户端模型切换不覆盖它。这个 Gateway 管理逻辑只存在于离线验收，不改变产品的原生启动行为。

## 证据

- 配置行为测试先在原占位 driver 上失败：具名 `acp_model_source_openclaw_config_routes_only_owned_gateway_and_model` 返回“launch has not been verified”；红灯退出101，2项中1项失败。
- 最终 `cargo test -p vw-agent --lib --all-features acp_model_source_openclaw -- --nocapture`：6项通过、0失败、退出0，涵盖配置/模型/argv隔离、冲突与远程拒绝、历史保持及令牌轮换、真实子进程组回收、启动失败和启动取消回收。
- `cargo test -p vw-agent --lib --all-features acp_model_source_routing_ -- --nocapture`：10项通过、退出0，包含缺失凭据、禁用模型与来源路由验证。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出0。
- `python3 -m unittest discover -s scripts -p '*acp_model_source*test*.py'`：28项通过。
- 两条真实脚本使用正式 `do_stream_request_acp`、profile、Bridge、Gateway、模型中继和原生文件工具，不以 profile 单元测试代替生产链请求。

| 来源 | 真实请求 | 实际模型 | 辅助代理请求 | 结果 |
| --- | ---: | --- | ---: | --- |
| [session_model](openclaw/session-model.json) | 10 | primary 9，switched 1 | 3 | 1项真实验收，退出0 |
| [cli_local](openclaw/cli-local.json) | 10 | 始终为原生 primary | 3 | 1项真实验收，退出0 |

两种来源的错误请求均为0，实际工具结果最多2份。辅助请求标记排除携带历史任务说明的父会话，避免重复计数。取消由合成模型 fixture 的鉴权控制面显式延迟下一请求，解决原生完成事件夹在用户输入后的快速回复竞态，不改变产品取消逻辑。

真实命令与每项结果见同目录报告；新建、第二轮、取消、重启/恢复、切模和隔离全部通过。客户端来源原生配置故意使用冲突模型和错误令牌，实际请求只使用绑定模型；原生来源实际模型始终保持本地配置。报告仅保留模型、路由、鉴权布尔值、工具回合计数和闭合诊断，不保存令牌或原始载荷。

共享 Cargo 槽位按每阶段原子领取，使用过2、3、4、5；最终检查及真实验收的确切路径、命令和退出码保留于本地 `target/acp-model-source-evidence/openclaw/*-result.json`，阶段结束释放自己的锁并保留编译缓存。仓库报告省略主机绝对路径。

本轮仅验收 Darwin arm64 和上述固定版本；Linux、Windows及其它 OpenClaw 版本未验证。本任务不修改 registry 或安装器，能力发布由任务33集成。

官方参考：[ACP Bridge](https://docs.openclaw.ai/cli/acp)、[Model providers](https://docs.openclaw.ai/concepts/model-providers)。固定版本随包实现还核对了 ACP 路由、原生配置 schema、会话存储路径模板和惰性工具动作。
