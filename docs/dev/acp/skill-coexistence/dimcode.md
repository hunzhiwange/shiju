# DimCode ACP 技能共存验收

修复前，固定 DimCode 0.5.28 的原生扫描器在只读会话中发现项目技能，但供应商权限层拒绝 `skill` 激活。在同一权限下，原生 Streamable HTTP 完成 MCP 初始化和目录读取，工具调用同样在 ShiJu 审批前被拒绝。

## 制品与直接 ACP 对照

Darwin arm64 上复用受管 `dimcode-0.5.28-model-source-v2/dim acp`。归档 SHA-256 为 `71634686a50bf9510862ec18a8381c49f5caf8df510c67085f2efac1773f04de`，受管入口 SHA-256 为 `ca470dad0e7663b669769ff8d22bc260339eb46fbff08b9dd7fc553ea225f746`。副本通过严格签名检查并返回 `dimcode 0.5.28`。归档冷下载超时，沿用摘要匹配的缓存制品。[已受审的 HTTP/SSE 路径](../mcp-policy-audit/coexistence/dimcode.md)

在隔离 HOME 和项目中，以 `session-selected/selected-model` 绑定私有 Chat Completions 回环。用既有仓库直接 ACP 夹具做只读对照，保持 `read-only`，增加项目 `.agents/skills` 中带随机正文的技能及原生 `skill` 调用。两次真实 ACP prompt 均返回 `end_turn`，进程退出 0。模型请求都列出了技能，原生 `read` 按同一调用 ID 读回项目普通文件。

| 对照 | 关联工具调用 | 原生结果 | 宿主审批 / 下游 MCP |
| --- | --- | --- | --- |
| 无挂载 | `call_before_skill` | 原生技能激活失败 | 0 / 0 |
| 只挂 HTTP MCP | `call_mounted_skill`、`call_mounted_coexistence__read_probe` | 原生技能激活失败，MCP 返回 `Tool call denied: source: default` | 0 / 0，服务实收 `initialize` 和 `tools/list` |

受管入口的 `read-only` 预设明确许可普通 `read`，对其它未许可工具默认拒绝且禁用交互审批。ShiJu 当前只在已有完整访问批准时同步 `workspace-write`。历史正对照在该模式下取得 MCP 允许一次及取消的结果。任务 28 沿用 `full_access=false`，现有技能挂载能力门禁和 stdio 拒绝继续有效。

## 网关探针

新增 `--agent dimcode` 验收入口，从 `defaults.skills` 和宿主 MCP 配置选择夹具。探针核对实际 Chat Completions 声明，要求原生和选中技能的激活、附件读取以及 MCP 均有同 ID 续轮。目标模型为 `coexistence-fixture/selected-model`，来源为 `session_model`。

本轮复用槽位 10 的已验网关，网关 SHA-256 为 `b2a05ed113477712c36de9dd731517d27f151d4e51f7c8096c0e80ae2e27d276`。Cargo 两条缓存路径均为仓库的 `target/rust-slot-10`。设置 `ACP_DIMCODE_REVIEWED_BIN` 可指定摘要和签名相符的受管入口副本。

```sh
PYTHONPATH=scripts python3 -c 'import sys; from cargo_slot import run; sys.exit(run(sys.argv[1:], number=10))' \
  sh -c 'python3 scripts/verify_acp_skill_coexistence.py --agent dimcode --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/dimcode.json'
```

- 正式无挂载轮退出 1，零 `chat.done`、一次 `chat.error`，未到达带工具的模型请求。只出现无工具的辅助模型请求。网关报告供应商会话在 `session/resume` 与 `session/load` 中均找不到，切换到私有项目作为网关工作目录仍复现。
- 独立挂载轮退出 1，零有效模型请求、审批及 MCP 执行，首个断点为 `dimcode: no verified skill directory mount branch`。脱敏报告为 `target/acp-skill-coexistence/dimcode-mount-gate.json`。
- Python 语法检查通过。无 Rust 改动，未启动 Cargo 构建或 Rust 单测。撤除、宿主审批拒绝对照和成功模型及权限回读均未取得。

继续接入需要在原只读边界内可调用的原生技能激活及可提交宿主逐次审批的远程 MCP 执行入口。先验证这两条原生路径，再评估追加会话发现根。

## 受管 v3 接线与定向验证

固定二进制的 `additionalDirectories` 对照通过初始化和原生普通文件读取，但其项目外技能未出现在原生目录。固定二进制的 `read-only` 带默认拒绝且会把普通 `ask` 折成无交互许可，直接配置 `ask` 无法给远程调用增加逐次审批。私有进程级发现根原型在撤除轮仍暴露上轮技能，因此弃用该方式。

v3 在受管 `session_model` 的 ACP prompt 元数据中追加当前校验通过的 ShiJu 技能视图。原生扫描器先保留供应商目录，再扫描视图，同名沿原生优先级。视图只在活动 prompt 内按供应商会话 ID 可见，退出即清理，宿主文件回调只追加所选技能的读取根。原先由默认权限拒绝的内置 `skill-loader` 激活可以运行，显式禁用和已有命名过滤继续生效。当前会话注册的原生 HTTP/SSE MCP 遇到默认拒绝时，直接请求宿主逐次批准，通过一次允许才继续原生执行。stdio 门禁、显式拒绝、原有文件读写和终端权限均沿用。

真实安装路径用已校验的缓存归档代替下载。固定输入摘要检查、解包、补丁、签名、入口摘要与严格签名检查均通过。受管 v3 `dim` SHA-256 为 `3bdc0e2ab4b04cfe366a0df3b5c997ae594816466d3a556e75176d3d9d14ebec`。

使用脚本生成的随机正文和附件，Darwin arm64 直接 ACP 四个独立只读会话均返回 `end_turn`，程序及探针均退出 0。挂载和拒绝轮由真实 ACP 文件读取回调获取视图内附件。

| 轮次 | ACP 会话 | 模型续轮 | 下游 HTTP MCP |
| --- | --- | --- | --- |
| 无挂载 | `sess_1795191163279_wgtgbhvamwn` | 原生技能激活，选集未声明 | 0 |
| 挂载 | `sess_1795191163768_x79pqmo9ckk` | 原生与选中技能激活，`call_mounted_3` 读回附件 | 1，`call_mounted_coexistence__read_probe` 获一次审批并回传结果 |
| 撤除 | `sess_1795191164625_2qrcvdxe1vr` | 原生技能激活，选集未声明 | 0 |
| 拒绝 | `sess_1795191164203_o2f16a0lis` | `call_cancel_3` 读回附件，`call_cancel_coexistence__read_probe` 回传取消 | 0 |

直接证据位于 `target/acp-skill-coexistence/dimcode-artifact/v3-host-reader.log`。接线前另复用槽位 10 的较新网关 `bb626e51a5235a6db6037d93b0242e945e573eea8aeb0471fc17da5c49d4fdc2`，仍复现无挂载轮先发生会话恢复错误，报告为 `target/acp-skill-coexistence/dimcode-current.json`。

`llm-runtime` 定向用例 `acp_skill_coexistence_dimcode_projects_only_a_valid_view` 实际运行 1 项并通过。复用同一源码的槽位 9 libtest `llm_runtime-85f23fc8970681f8`，SHA-256 为 `333ad5b390bafe16c48793d98ff13873c191958d6c070867044ba6786c9fba33`。两条 Cargo 缓存路径均为仓库的 `target/rust-slot-9`，执行过滤参数 `acp_skill_coexistence_dimcode --test-threads=1`，退出 0。最初定向编译报两处 Rust 类型推断错误，已删去多余的转换。

同一 v3 制品的直接 SSE 对照也通过四个只读会话，实际执行计数同为 0/1/0/0，挂载和拒绝轮均委派附件读取，证据为 `target/acp-skill-coexistence/dimcode-artifact/v3-host-reader-sse.log`。

旧网关的会话恢复报错可用夹具输出预算解释。直接模型协议核对，辅助模型请求设 `max_tokens=100`，工具轮设 `max_tokens=16384`。把夹具限额从 1024 调到 16384 后，槽位 6 的旧网关使用 v2 已进入真实工具轮。两次有效模型请求后确认原生 `skill` 在宿主审批前被只读默认规则拒绝，零 MCP 调用，探针预期退出 1。报告为 `target/acp-skill-coexistence/dimcode-v2-budget-red.json`。

## 网关正式验收通过

槽位 3 的 `cargo build --locked -j 1 -p vw-cli --bin shiju` 退出 0，使用该槽的两条绝对缓存路径。网关 SHA-256 为 `b2df327cf4e8cd820216995fce313fb2c00c51fc0615ccb68a2ee1f7d8396d0f`。首次探针无挂载轮通过，挂载轮成功激活所选技能，但探针误把 `SKILL.md` 的二级父目录当成视图外层。修正断言后同时校验 `shiju-session-skills-*/skills/<name>` 布局、技能目录链接和签名制品上的真实路径。网关构建产物的同摘要私有副本由槽位 10 持有供复跑，两条缓存路径均指向槽位 10，复跑未重编译。

| 轮次 | 新网关会话 | chat.done / chat.error | 实际 MCP | 审批 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_e06275c44ffetT4YQDZdDpu64B` | 1 / 0 | 0 | 无 |
| 挂载 | `ses_e06273f92ffezh161N0y2BKiAf` | 1 / 0 | 1 | 允许一次 |
| 撤除 | `ses_e06272193ffeZxienYBxmXQ5Wd` | 1 / 0 | 0 | 无 |
| 拒绝 | `ses_e062707c1ffexOlG1qxbs1w4xx` | 1 / 0 | 0 | 拒绝一次 |

四轮均读回原生技能正文和附件。挂载轮原生附件、选中技能附件和 MCP 结果分别按 `call_444ea4662282065348598eb9`、`call_7227e11f95d7eaeafa3a6cde`、`call_3bad6ab8ddde9cff6df17dd0` 回到模型。拒绝轮 `call_3f4a8b348b843b7b47ea1edb` 回传拒绝，未读取下游附件。各轮回读 `dimcode / session_model / coexistence-fixture/selected-model / full_access=false`。探针退出 0，报告为 `target/acp-skill-coexistence/dimcode.json`。
