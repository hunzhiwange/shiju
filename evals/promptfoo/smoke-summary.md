# 单题端到端冒烟

2026-10-25，唯一真实模型任务为 dev/edit-01，模型为
`token-plan/deepseek-v4-flash-0731`。闭环取证完成，题目未通过：24 步、27 次工具完成事件后耗尽预算，目标字节不符合 oracle，非目标文件保持不变。不得把此结果计为基线通过或20题通过。

## 执行与证据

从仓库根目录执行：

```sh
node evals/promptfoo/freeze-accuracy.mjs verify
PYTHONDONTWRITEBYTECODE=1 python3 evals/promptfoo/accuracy-smoke_test.py
node --test evals/promptfoo/*.test.mjs
PYTHONDONTWRITEBYTECODE=1 node evals/promptfoo/accuracy-smoke.mjs --preflight
PYTHONDONTWRITEBYTECODE=1 node evals/promptfoo/accuracy-smoke.mjs --run
node evals/promptfoo/audit-smoke.mjs
```

冻结摘要校验退出0，digest为
`30beee04a8c0a7a2aa6814a0fc8100389ee1e311760c0293f10328bb3b1f5213`。
Python离线测试6/6；Node测试94/94；最终预检退出0，13/13 canary，无模型调用。
额外原生 `posix_spawn` canary返回EPERM；与fork拒绝共同验证无法派生脱离进程组的写入者。
运行 `cargo build -p vw-cli --bin shiju`（默认features）退出0；
`cargo clippy --all-targets --all-features -- -D warnings`退出0。未修改Rust或运行全量Rust测试。

真实运行退出100，Promptfoo记录1条错误，仍保留原评分。SSE第24步后监督端
拒绝第25次模型调用，停止请求与下一轮之间的竞争表现为本地HTTP403；这是保护预算的终态，
不是正常完成。上游成功接收24次调用、真实模型任务1次，未重试真实模型任务。
usage未完整报告，显式标记missing；不得把SSE中的零usage当成零费用。

原始结果、轨迹、产物及实际会话/路径仅存忽略目录
`results/accuracy-smoke/`：`result.json`、`attempt.json`、`environment.json`、
`gateway.log`、`artifacts/`、`wrong-artifact-replay.json`与`audit.json`。
离线审计从本次独占Gateway日志恢复唯一session ID，核对attempt/workRoot/请求目录/endpoint/模型/证明一致，
记录`capability-budget-exhausted`；不修改终态、预算、oracle或Promptfoo评分。
边界验收通过，Gateway及写入线程已结束、端口关闭、工作目录清理、槽位锁释放。
原始错误路径的Provider未保留顶层session ID，故审计明确记录日志来源，不伪造SSE终态。

构建、预检、真实运行和最终Clippy均使用动态领取的槽位1：
`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`。
缓存保留；Clippy是静态检查，不计为行为测试。最终检查发现仓库的build-dir配置另指共享目录，
新包装亦设置同槽位的`CARGO_BUILD_BUILD_DIR`；该设置经最终Clippy验证，不重新调用模型。

## 监督边界与恢复记录

监督端只读取本地已有目标模型endpoint和API鉴权；密钥仅保存在监督进程内存。
Gateway继承默认拒绝Seatbelt策略，只能读写本次workRoot和独占状态目录，
不能读取仓库、oracle、监督证据、宿主凭据或共享tmp。它只可连接一个本地模型转发端口；
转发端固定HTTPS上游、模型、路径、24次请求上限与单并发，拒绝重定向，
上游错误立即关闭通道。Bash/子进程能力被明确拒绝，不声称完整终端能力已验收。

第一次Gateway任务请求因租户配置不一致未解析到模型，上游调用0次，
证据单列于`zero-upstream-failure/`。收到明确恢复授权后，才归档其预留标记并恢复。
将独占环境的请求、全局配置和项目配置统一到运行时默认租户10000/100000000，
再通过只读Provider目录检查确认active模型及精确relay路由，随后执行上述唯一真实模型任务。
初次失败不计有效模型尝试，不掩盖其Provider退出验收参数错误；该路径已有离线回归测试。

冻结的score.sh、配置、Provider、manifest、题目、oracle及权限规则均保持原字节。
由于score.sh固定通用配置，单题使用独立监督配置调用同版本Promptfoo及相同
suite/split/category/pattern/repeat筛选参数；确切底层命令记录于`eval-command.json`。
不能直接用通用score.sh命令替代本次受监督运行。
`--run`的持久预留标记拒绝后续自动重复；不提供删除标记或重试的自动流程。
本任务没有执行holdout模型任务、修改Agent、放宽评分或启动后续任务。
