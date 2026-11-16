# 全量归属基线

采集日期：2026-11-16。HEAD：`cc424ddb8f2ec7b6d38a950de28ff1e295fdaae8`。
开始时 `git status --short` 为空。采集期间其他会话修改了 Desktop Git UI，未纳入本任务。

## 原始集合

`baseline.json` 保持原样，使用 pathlib 递归 `.rs` 与 `read_text().splitlines()`：当前仍为 **1381 文件、135228 行**。无新增、删除、行数差异，详见 snapshot.json。
scope.rows 的任务2至58和 scope.residue 的任务59完整覆盖原文件；逐文件唯一所有者，无遗漏、无重复，所有源路径与任务2至60编号卡均存在。
residue 是任务59的待审查集合，不代表已经批准整体留在宿主；后续仍须逐符号说明具体组合、租户、权限或资源生命周期边界。

## 注册图及外部挂载

从 lib.rs 递归追踪 mod、path、include；cfg 不求值，保留局部属性，条件的跨文件继承可沿 registrations.json 边回溯。扫描全部原文件，避免只覆盖少量测试。

- 1570 条模块/文件引用；lib 词法可达集合1508个 Rust 文件，其中宿主1299个，外部209个。
- 原集合82个文件未从 lib 词法可达，完整清单在 registrations.json 的 orphan_candidates；不计为已证实的编译负担。包含大量兼容转发文件，也包含未注册测试。
- external-mounts.json 登记266条跨 src 边界引用，247个唯一外部路径：210个 Rust 源码、37个资源。其范围包含孤立源文件引用的目标，不能把全部247个路径当作 lib 编译单元。
- scope.json 仅新增 external_mounts，每项有唯一 owner_task；共享资源列出 consumer_tasks，初次遍历的所有者负责协调后续迁移，不改变原始归属。include_str/bytes 是资源，不是 Rust 源码编译单元。
- 两处静态未解析注册：任务17的 gateway/middleware.rs:4 与任务18的 gateway/router.rs:227 分别默认寻找 middleware/middleware_tests.rs（或 mod.rs）、router/router_tests.rs（或 mod.rs），但现有测试位于父目录的 middleware_tests.rs、router_tests.rs。所属任务应修正注册；本任务不修改生产文件。

扫描器是可复查的词法工具，不展开过程宏或宏生成的模块，不解释任意动态 include 表达式，不构成编译器结论。当前扫描到的 include 表达式均为静态字面量或 CARGO_MANIFEST_DIR 拼接。字符/字符串及注释被词法跳过；未解析项明确保留。

## 依赖、端口和全局夹具

根 Cargo.toml 与宿主 Cargo.toml 均已静态读取。dependencies.json 按原 TOML section 记录369条本地路径依赖声明（含平台及 dev 重复项），全部清单存在、全部在 workspace members 中；scope_tasks 关联现有任务候选领域库。空 scope_tasks 表示组合依赖或尚需任务59判断的公共依赖，不伪造其迁出目标。

host-ports-and-globals.json 按路径、行号、归属记录102条 Host 实现候选与250条全局状态/环境操作候选。仅做声明定位，不逐测试分析行为，也不新增接口。代表边界包括 session 的 AdmissionHost、CompactionHost、PromptHost、LifecycleHost、PlanningHost，agent-loop 的 HistoryHost、QueryEngineHost，工具执行的 AgentContextHost、网关 GatewayChatHost/GatewayWsHost、工作流 NativeToolHost/NativeAgentHost 与 cron SchedulerHost/DeliveryHost。具体所有者以逐项索引为准。

迁移须保留已有进程隔离：runner_tests 的 OnceLock 环境锁、channels/config_tests 的环境锁、cron scheduler 的锁、app_routing 的 OnceLock 临时 HOME；外部 session-engine 的 ENV_LOCK、LLM 消息/流 LazyLock、生命周期锁；tool-git 的 TEST_HOME；channel-whatsapp 的 global_runtime_proxy_lock。全局连接/租户仍须调用已有领域存储和宿主端口，不能复制 App/AppState 或另造全局夹具。

## 验证证据

执行 `python3 .plans/2026-11-16-代理宿主完整减负/evidence/collect_baseline.py` 成功，原集合、计数、唯一归属和路径断言通过。脚本与完整 JSON 清单随证据保存，可复查；任务卡删除后不再运行要求其存在的采集断言。

所有真实运行注册数、编译与行为结果均未取得；未运行 Cargo、单元测试、实机或构建。用户将来授权后可运行 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib -- --list` 核对真实归属；本次不声称收益或测试通过。
