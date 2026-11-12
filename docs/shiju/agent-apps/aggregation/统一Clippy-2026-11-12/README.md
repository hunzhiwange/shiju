# 2026-11-12 统一全仓 Clippy

**第四轮统一检查已真实通过，适用于记录的本计划最终生产代码及任务41修复。** 实际命令退出0、185.84秒、起止源码摘要相同，首次严格复用脚本也退出0。这是不可变成功事实；之后新增的两处MCP51测试代码不反证该轮通过。

后续变更已超出最初两处MCP51测试：最近严格整树及限定范围脚本均返回1；第四轮不覆盖新增的服务端生产代码和测试。原任务应按实际依赖与验收结论独立判断，不把任意后续文件变化自动升级为本计划全部任务未通过。

## 最新适用范围更正

第四轮完整Clippy退出0及首次严格核验0均为真实历史事实，封存文件不变。最初仅两处MCP51测试变化时，限定脚本确实退出0；此后服务端授权、重同步、超时等源码继续变化，限定脚本保守返回1，不声称新代码已经检查。

最新有时点的差异清单见[latest-source-applicability.json](latest-source-applicability.json)。该次观察中，任务15所在`vw-agent-apps`及保守工作区依赖闭包的源码均与第四轮相同；[task15-dependency-closure.json](task15-dependency-closure.json)记录真实退出0的只读Cargo metadata命令、依赖边和槽位2租约清理。任务15保留10项行为通过及第四轮完整检查证据，可按该范围完成。其他原任务独立评估自己的输入；不据后续任意变化自动判16项全部失败，也不自动判全部通过。

依赖核对是带时点的证据，不是对未来工作区状态的保证。`task15-dependency-applicability.json`为较早一次观察；`plan-scope-applicability.json`和`plan-scope-check.log`保存最初限定0，均不代表后续整树状态。检查脚本不扩大白名单。后续若相关依赖变化，由原任务重新判断适用性。

## 完整结果与复用

- 完整未裁剪Cargo输出：[clippy.log](clippy.log)。命令、实际退出码、时间、完整环境、Cargo PID、源码摘要与租约清理：[result.json](result.json)。驱动收尾：[final-driver.log](final-driver.log)。
- 源码摘要：`a48d2d8bc172595a88c040746887bf0ebd5706a3879c69e0cd34f09a9500c889`；HEAD从`cedd49cfb62034005e879bd37c60ad0c739d4e5a`到`fcccd2cc32a479e8526aeafaea7cf306375a7474`，源码内容摘要仍相同。记录包含当时已跟踪与未跟踪的非忽略源码、配置和资源，不把HEAD单独当源码身份。
- 快照排除`.plans/`和`docs/`，允许其他任务单纯更新证据或任务卡后继续核验复用；完整路径/字节/SHA-256清单和工作区补丁按内容去重gzip保存，元数据索引[source-index.json](source-index.json)。不将后采集的外国候选见证冒充实际输入。
- Rust工具链：[toolchain.txt](toolchain.txt)，实际rustc 1.99.1、aarch64-apple-darwin、LLVM23.1.1。

从仓库根运行以下只读校验；也可传脚本绝对路径从其他目录调用：

```bash
python3 docs/shiju/agent-apps/aggregation/统一Clippy-2026-11-12/verify_source.py
```

退出0表示成功命令、稳定源码、当前源码内容、工具链及任务41交付闸口均匹配；退出1为源码/工具链变化，退出2为未通过/未完成、跨源码变化或交付闸口不满足。只有适用时复用真实检查，不为凑记录机械重复。各任务自己的聚焦与实机场景仍由原所有者验收。

## 可直接转发的两类核验

不可变成功结果：`round4-result.json`；完整日志：`round4-clippy.log`；源码快照和工作区补丁：`source-index.json`及其引用的gzip档案；任务41交付：`task41-handoff.json`、`acceptance.json`。

```bash
# 保守限定核验；首次两处测试差异时退出0，后续更多源码变化时退出1
python3 /Users/xiangmin/worker/code/shiju/docs/shiju/agent-apps/aggregation/统一Clippy-2026-11-12/verify_plan_scope.py

# 严格整棵源码树相同；后续源码变化时退出1
python3 /Users/xiangmin/worker/code/shiju/docs/shiju/agent-apps/aggregation/统一Clippy-2026-11-12/verify_source.py
```

首次核对的历史差异与限制见`plan-scope-applicability.json`、`post-round4-source-change.json`。当时只有`agent_apps/mcp/mod.rs`末尾新增3行`cfg(all(test,target_os="macos"))`测试模块注册及其`security_tests.rs`文件不同；移除该精确注册块后，mod.rs的SHA-256与成功快照逐字节相同。其他生产源码、Cargo清单/锁文件与Task41散列都相同。该模块生产构建不加载；新增MCP51测试本身的编译、Clippy及行为结果未被第四轮覆盖，由其原所有者独立验收。限定脚本不等于宣告最新整仓字节完全相同，也不替各原任务删卡。

任务15所在`vw-agent-apps`的源码、普通/测试依赖未变，Cargo清单也不依赖`vw-agent`，新增测试不影响其10项行为与本次全仓证据的适用性；任务15正式状态为已完成。

## 确切命令与实际环境

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

第四轮动态领取**槽位5**，仓库当前检出、macOS原生target，没有新建工作树，没有修改仓库profile或Rust flags：

```text
CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-5
CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-5
CARGO_INCREMENTAL=0
CARGO_BUILD_JOBS=1
CARGO_PROFILE_TEST_OPT_LEVEL=0
CARGO_PROFILE_TEST_DEBUG=0
CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_CODEGEN_UNITS=256
CARGO_PROFILE_TEST_LTO=off
```

完全参考任务59最终通过的命令级低内存设置。Clippy实际可能选dev opt-level=1，不据TEST_OPT_LEVEL宣称有效opt-level=0。领取时比对保留缓存profile、features、源码path和native flags，并按最近元数据动态原子领取空闲槽位；未按任务号固定分配。整个阶段仅一组自有重型Cargo，所有自有消费者退出后核对完整PID/nonce并释放租约，缓存保留。

原先“等待所有无关进程消失”的额外条件已撤销。采样记录了高换页量与约36%可用内存，按实际空闲内存、匹配缓存和自有CPU/RSS推进判断，受控执行而非无限排队；没有停止别人的进程、清理别人的锁或发送未授权跨计划消息。资源记录[resource-samples.jsonl](resource-samples.jsonl)、[round3-resource-decision.json](round3-resource-decision.json)。

## 本任务与必须覆盖的交付

任务15原过滤命令本轮重新执行，**10项全部通过、退出0**，默认features、macOS原生target；真实原生配置/目录文件、原事务、项目隔离、旧修订、发布前丢弃及发布后恢复都参与。[round1-task15.log](round1-task15.log)、[round1-result.json](round1-result.json)。随后变动不涉及该发布器/目录/事务所有者，保留这份适用行为证据，不机械重复。

任务41的认证顺序修复由提交`dcc7f3f0ac378e4773fc5fea1593abbdda3c5810`交付。[task41-handoff.json](task41-handoff.json)记录本轮11项聚焦及1项独立重启权限测试全部通过。最终Clippy的开始清单和结束清单均包含下列散列；[acceptance.json](acceptance.json)与复用脚本再次核验：

- operations/aggregation.rs：`fdc88b422981b4af3d17bdbed4edd5d1818cabdb88933625a3df1de74223f53a`。
- aggregation_publish_tests.rs：`f2051e5d504d12349c87b69ea4bf25d10b15352eef2a2e0fc97334fb2e2c1349`。

编号任务15卡已在外部提交`5019a4641`中删除，非本回归会话删除；未重建或覆盖该提交，也未把卡片消失推导为通过。现在取得真实完整通过证据后，任务15正式状态更新为已完成。没有宣告母能力完整完成或真实Codex/Claude/Desktop客户端验收通过。

## 历史失败与来源核对

| 阶段 | 实际槽位 | 结果与证据 |
| --- | --- | --- |
| 冷缓存起步 | 12 | 依赖冷编译、行为测试0；主动停止，Cargo实际退出状态未重收，wrapper137，不计业务红灯或通过。cold-attempt-*保留完整记录；确认仅剩僵尸后清理准确自有nonce租约。 |
| 第一轮统一 | 2 | 101；MCP详情页5处column!歧义，阶段另有源码变化。round1-clippy.log、round1-result.json。 |
| 第二轮统一 | 2 | 原所有者修复导入后再跑，101；同页SessionId/ToolId未实现Display，阶段另有源码变化。round2-clippy.log、round2-result.json。 |
| 第三轮统一 | 3 | 原所有者修复类型格式化后再跑，101；端点诊断endpoint_state.rs的10处E0433，toml只在dev-dependencies，生产lib无法引用。round3-clippy.log、round3-result.json。 |
| 第四轮统一 | 5 | 原46将解析改为toml_edit并补正式依赖后，对实质变化的源码真实检查；**0、起止源码不变、首次复用校验0**。clippy.log、result.json。 |

各历史完整日志均保留原文，失败没有改写为通过。不修改其他错误所有者代码，不在同一失败输入上连续检查；阶段源码变化清单为round1/2/3-source-changes.json。

其他计划候选74554、74941、77809的正常全仓命令虽匹配，但ctime见证发现启动后的源码/目录状态变化，无法证明快照适用性，没有据其可能的退出0冒充统一通过；候选79763只有桌面包、没有all-features，命令本身就不满足。foreign-*.json仅为只读拒绝/见证记录，不是最终通过来源。完整统一通过来自本会话第四轮真实检查。

本轮未改Rust业务代码，不再重复格式化/无过滤全量单测，没有wasm或Makefile检查。自动证据不代替真实客户端发现、模型重选/缓存、官方登录或实际进程重启的人工验收。

## 通过后的新增变更

第四轮退出0及首次复用校验0完成后，MCP原所有者又修改agent_apps/mcp/mod.rs与security_tests.rs。后续当前源码校验实际退出1，见post-round4-source-change.json。历史成功记录完整封存round4-result.json、round4-clippy.log、round4-driver.log，不改写为失败；但不冒称它覆盖后来的新测试注册/实现。最初仅两处测试差异时限定脚本曾退出0；之后生产代码变化使其退出1。完整当前覆盖关系以文首最新核对为准。
