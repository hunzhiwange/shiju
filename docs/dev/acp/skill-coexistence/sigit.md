# siGit 原生技能与会话技能共存验收

## 修复前

2026-11-20，Darwin arm64。槽位 3 的现有网关使用受管 siGit 1.7.10 v3。无挂载回合 `ses_e06cf99f8ffeU8CAxurjDwcBS1` 向指定的回环模型发出一次正式推理请求，收到一个 `chat.done`、零个 `chat.error`，MCP 调用为零。探针退出 1。首个行为差异是模型工具清单没有原生 `skill`，与 v3 补丁主动清空发现结果一致。该轮的结构化证据为 `target/acp-skill-coexistence/sigit-v3-baseline.json`。

同摘要网关与 v3 入口的独立挂载会话 `ses_e06a68807ffeKdijG1TLCRZ6vH` 在宿主能力门禁返回 `sigit: no verified skill directory mount branch`。一个 `chat.error`、零个 `chat.done`，模型请求与 MCP 调用均为零。挂载复测用槽位 9 运行快照，退出 1，报告位于 `target/acp-skill-coexistence/sigit-v3-mounted.json`。

| 制品 | SHA-256 |
| --- | --- |
| 固定 1.7.10 源码 | `088809cfc43ceda419913488390a65e830ac8ff66f1b5dd14a8d833d6b46369e` |
| v3 受管补丁 | `3884ccd9580594a612ebf06de5636b62bd749ed1098b89b0c4c8a7e5c2440c4e` |
| v3 原生入口 | `322287c3fc4b2b88434f66464f56ea55f086d151b0c2682a44633defe90d34ea` |
| 基线网关 | `808b01432ed6f30664f849b0130a97a320b952d395aa34c6723e6ec19be2edf8` |

## v4 接线

`sigit-1.7.10-model-source-v4` 在 Darwin arm64 的私有 `session_model` 路径扫描工作区的原生 `.sigit/skills` 和 `.claude/skills`，再追加经宿主核验的会话选集。同名条目遵循原生工作区优先级。发现仅解析有界元数据，并拒绝经符号链接打开原生条目。选集仅接收会话视图中仍匹配注册目标的链接。原生技能激活和附件读取使用受宿主权限约束的 ACP 文件回调。用户级扫描继续禁用，stdio 服务沿用原有只读沙箱和单次审批。[受管补丁](../../../../patches/acp-model-source/sigit/README.md)

只读 MCP 夹具复用现有 stdio 服务与 FIFO 审计。系统 Python 框架复制到私有工作区，以满足受管沙箱对系统开发工具目录的读取限制。使用原沙箱独立校验 `tools/list` 和 `tools/call`，退出 0，成功读取随机夹具的数据路径并产生一条调用审计。

受管安装入口从固定归档重新构建。v4 补丁 SHA-256 为 `419c0cc2e4572084c6479cfcaea7f84cd467d362ed97d6bf3037bdb963849ab8`，原生入口 SHA-256 为 `d637fbe162d3b27a6a4c10906b92252f791111aedc4d0a0293209a51c99e486d`，与安装收据一致。原生 ACP 握手回报 1.7.10。首次构建退出 101，定位到上游 headless 调用仍使用旧签名。为该调用保留空选集后，定向安装构建退出 0。

直接 ACP 挂载诊断退出 0。回环端收到六次模型请求，并按调用 ID 收到原生技能正文及附件、选中技能正文及附件、实际调用的 stdio MCP 结果。五次调用均经过 ACP 单次允许，MCP 审计为一条，原生提示终态为 `end_turn`。该诊断使用合成宿主文件回调，另由生产网关探针检查真实宿主接线。

| 原生聚焦诊断 | 调用 ID |
| --- | --- |
| 原生技能 | `call_bd653ded8670ba4833bf222f` |
| 原生附件 | `call_08d855ae024356d1da59b497` |
| 选中技能 | `call_233fbf6ab1adaf9dca7fd1aa` |
| 选中附件 | `call_2bb3188a9bfe44e5e2c0a25a` |
| 已授权 MCP | `call_917dd6f0b31ba3d0f2ac204d` |

直接 ACP 补测从工作区外的受信选集读取正文和附件，调用 ID 分别为 `call_787686283aee2320e3a36cc5` 和 `call_0634d126d44cd24fd33cec41`。模型收到 MCP 拒绝结果 `call_a2edbb53a6ed841316bb5745`，下游调用为零，退出 0。

## 生产网关回环验收

网关通过 `cargo build --locked -j 1 -p vw-cli --bin shiju` 定向构建，退出 0，耗时 21 分 47 秒。网关 SHA-256 为 `571e680ccc0c8a7f607881e9b772a5d18c42ce4762a13a4f6c4a97f8273d3b04`。构建、私有原生制品安装和探针均使用槽位 3。两个 Cargo 缓存变量同为 `target/rust-slot-3` 的绝对路径。

四轮用实际网关和 v4 入口，仅替换指定模型的响应。新建会话均回读 `acp-coexistence / sigit`、`session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017`、`full_access=false` 及 `access_status.requested=false`。访问状态的 `support` 为 `unavailable`，工作区权限由宿主审批。挂载轮从 `defaults.skills` 和工作区 MCP 配置解析真实选集。首个模型请求仅出现允许的技能元数据，始终排除未选技能和越过原生目录的符号链接；正文和附件均于对应调用后出现。撤除轮仅保留原生技能。

| 轮次 | 会话 ID | `chat.done` / `chat.error` | 模型请求 | MCP 决策 | 下游 MCP |
| --- | --- | --- | ---: | --- | ---: |
| 基线 | `ses_e06827e92ffe3xyaaHM79G20A4` | 1 / 0 | 3 | 无 | 0 |
| 挂载 | `ses_e06825ed1ffeTz1YrzUTvrk6nO` | 1 / 0 | 6 | 单次允许 | 1 |
| 撤除 | `ses_e06823a8effeSB780mDbuSSGWe` | 1 / 0 | 3 | 无 | 0 |
| 拒绝对照 | `ses_e06821c8effenrDCUBKkYTtvGW` | 1 / 0 | 6 | 拒绝 | 0 |

| 轮次 | 原生技能 / 附件调用 ID | 选中技能 / 附件调用 ID | MCP 调用 ID |
| --- | --- | --- | --- |
| 基线 | `call_937f31b0317ecde7ae37f8af` / `call_169fefae30f2bd4114c05912` | — | — |
| 挂载 | `call_bbd406733751953fdf3fcebe` / `call_6c93ed812820c60bedb99688` | `call_8027a3233173b0e945c4d327` / `call_721bcc8110e266a6ac4c1a7d` | `call_3a3c613e1c9f65ef0d88466b` |
| 撤除 | `call_9da557fc01b928c881604858` / `call_d8f598b81e9d671342c34ebc` | — | — |
| 拒绝对照 | `call_ce6d3047800f1503a299f235` / `call_cc3529460ed8c9268b189b61` | `call_4c45451db4c63737b593b88f` / `call_720b4d12d41320277fbb0c31` | `call_cd19e40792822b20f7ddf03b` |

`python3 scripts/verify_acp_skill_coexistence.py --agent sigit --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/sigit.json` 退出 0。报告保留脱敏会话、模型、权限及调用证据。拒绝对照的模型续轮收到拒绝结果，MCP 附件未进入模型请求。

## 定向单测

使用槽位 3 运行 `cargo test --locked -j 1 -p llm-runtime --lib acp_skill_coexistence_sigit -- --test-threads=1`，退出 0。两项通过，零项失败，521 项过滤。覆盖空选集、仅注册已验证的会话视图，以及源变更后的拒绝。生产网关、临时运行时与 MCP 子进程已退出，槽位锁已释放。
