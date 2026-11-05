# Auggie 模型来源调查证据

状态：**上游阻塞，未支持**。2026-11-04，Darwin arm64、Node v26.13.0。
本记录没有将版本探测、扩展加载或拒绝路径当作模型验收。

## 固定包与真实探测

| 官方 npm 包版本 | CLI 提交 | 结果 |
| --- | --- | --- |
| `0.36.0`（公共 manifest 基线） | `7c61e5bb` | ACP 参数解析拒绝 `--extension`，退出 1 |
| `1.2.0-prerelease.202611031510`（独立调查） | `2c02c0c37a9ce983fd62e407026f441855626f94` | 扩展已加载，`registerProvider` 存在，但调用明确拒绝；ACP、print 均退出 1 |

1.x 的真实错误是 `Extension provider registration is not supported`。扩展在调用前
写出独立标记，证实没有把扩展路径错误、未加载或缺函数误判为能力限制。
追加探测 Pi 的 `models.json` 文件形式（专属缓存根及 `agent/` 子目录，并指定
`PI_CODING_AGENT_DIR`）仍报 `No models available`，退出 1。此探测仅排查替代方向，
不声称穷举未公开配置。未使用 Augment 账号或认证环境。

四项结果见 [upstream-gates.json](auggie/upstream-gates.json)，有效模型请求 **0**。
该 JSON 的 `probe_completed` 仅表示关口探测复现成功，`model_source_verified=false`。
探测未到达会话模型生成，无法证明主/辅助模型、工具回合、恢复、取消或无回退。
原 driver 继续默认拒绝，未启用公共能力。

## 复现

从官方 npm registry 下载两个**确切版本**，分别解包到
`target/auggie-inspection/package` 和 `target/auggie-inspection/preview/package`。
不修改全局安装或公共 manifest；脚本先核对包名、版本及 CLI 文件 SHA-256。
两包 tarball SHA-512 均与对应官方版本元数据核对一致：

| 版本 | tarball integrity |
| --- | --- |
| 0.36.0 | `sha512-jb4kq97pGzG0i5UyISc//LjTVlFzIywwD+VfPemiSIl58KjLfxry3KDgxWYnr1vmV8yUkySVzjLnj4ewUEbT/Q==` |
| 1.2.0-prerelease.202611031510 | `sha512-E6dMbPcDa5KeWAiT1ug3FMRJHEDsFJR+3RPGT31Txz8V8XG8SakQ0HAuN7HgpIAZj3m4voV252R5S2Jovf/bqA==` |

```sh
python3 docs/vibewindow/acp-model-source/evidence/auggie/probe.py --output target/acp-model-source-evidence/auggie/upstream-gates.json
```

本次命令退出 0，复现 4 项预期拒绝，loopback fixture 收到请求 0。
子进程均使用随机独立缓存目录和工作目录，认证环境不继承，HOME 不替换；
1.x 显式使用 `--offline --no-cloud --no-extensions`，仅加载给定扩展。
不保存账号、请求正文或原始供应商日志，子进程组在退出及超时后清理。
fixture 只记录实际到达的路径；没有用它证明其他主机的网络流量不存在。

任务 10 的两种来源脚本首轮均退出 1：缺少经审查的原生离线配方。
该结果发生在真实模型调用前，不能作为双来源支持证据。本次未重复已知失败的验收，
也未为越过入口限制编造配置配方。

## 替代方案与补丁边界

- `--model` 选择模型不等于注册模型服务；`--augment-cache-dir` 仅提供隔离目录。
- 官方 1.x 文档说明 pi 扩展可能与原 pi 行为不同；真实包证明了 Provider 注册差异。
- `AUGMENT_SESSION_AUTH`/session JSON 描述 Augment 服务认证，不是已确认的
  Chat、Messages、Responses 或 Gemini 模型接口。不能只替换服务 URL 来伪装接入。
- 固定包许可证禁止修改其 CLI；本任务不具备改动受限实现的授权证据。
  不以二进制字符串替换、运行时猴子补丁或替换 Agent 绕过该限制。
  具体版本及后续接口要求见 [补丁审查记录](../../../../patches/acp-model-source/auggie/README.md)。

后续需要上游提供 ACP 可用的模型服务注入入口，或提供允许修改对应实现的授权及
可审查源码，再验证真实主/辅助模型均不回退。仅增加共享 fixture 能力不能解决上述
Provider 注册拒绝，因此当前无需占用共享文件串行窗口。

首轮 `cargo clippy --all-targets --all-features -- -D warnings` 退出 0，槽位 1，
`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，自有锁已释放。
本轮只新增独立 Python/文档证据，无 Rust 变更，未重复 Cargo 或全量格式化；
首轮结果不代表后来并行工作区的统一回归通过。

官方依据：[CLI reference](https://docs.augmentcode.com/cli/reference)、
[1.x preview guide](https://github.com/augmentcode/auggie/blob/main/docs/1.x-preview-guide.md)、
[0.36.0 元数据](https://registry.npmjs.org/@augmentcode/auggie/0.36.0)、
[1.x 固定版本元数据](https://registry.npmjs.org/@augmentcode/auggie/1.2.0-prerelease.202611031510)。
补丁限制来自这两个已下载包各自的 `LICENSE.md`，未将 GitHub 文档仓库许可证
误用为 npm CLI 的修改许可。
