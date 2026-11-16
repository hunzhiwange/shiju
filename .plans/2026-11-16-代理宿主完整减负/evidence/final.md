# 最新静态收口（本节替代历史快照）

宿主949文件、36014行，基线135228行，净减99214行；35000行硬门槛仍差1014行。1381原路径归属唯一，residue 3240行。任务59保留已阻塞，原因是原所有者领域残留尚未闭合，并非未来编译验证。

本轮从仓库根依次执行以下命令，全部退出0：

```text
python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_final.py
python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_registrations.py
python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_domain_registrations.py
python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_test_candidates.py
python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_domain_changes.py
python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_alias_callers.py
```

结果：1105条宿主词法注册边零悬空；外部挂载178文件56350行，独立列账。领域622根、9225可达文件，40条未解析候选（含生成资源及Desktop源码引用），未将其报告为全领域零悬空。4159条标准测试声明：原路径1972、唯一候选2159、所有者候选28、歧义0、缺名0；55条ignore无差异。136目标包3280路径统计、51处别名调用映射已刷新。名称可达映射不证明断言等价、宏展开、平台cfg或真实编译注册；生产符号全量语义映射尚未独立闭合。未运行Cargo、构建、单元或实机测试，不承诺编译提速。

f10a33212与f2b0d1a62已分别完成8/42本轮拆分，9673b21a0完成16旧源删除提交，不重复派发这些旧候选。剩余明确候选38、6，逐文件符号、拆分职责与真实边界见task-59-recovery-priorities.json；仅为审查候选，不承诺可减少1014行。任务59不代改原所有者领域文件。当前工作区存在其他任务的暂存及未暂存修改，本次只提交任务59审计产物。

---

以下为历史记录，数值以首节为准。

# 任务59：本轮静态收口

最新宿主 **43584行**（960个Rust文件），基线135228行，净减 **91644行**；35000行门槛仍差 **8584行**。本报告替代51915/67221等旧快照。原所有者本轮均已提交，当前未观察到领域会话运行。不是外部环境阻塞；本次未运行Cargo、构建、单元或实机测试，未来编译验证不是当前阻塞理由。

## 本任务代码

- 51处纯转发改为现有父模块的直接领域别名；删除17个未注册memory文件和4个knowledge转发文件。保留原可见性、外围cfg及调用路径，不计为新领域算法。9c5a06db6。
- router_tests显式挂载到真实现有文件；18条生命周期和9条项目作用域测试补回原测试属性，未删断言。9c5a06db6。
- 删除无生产调用的crate私有app::config桥接；原测试移至session-store实际ui_config存储助手，用真实临时SQLite和显式默认租户，保留key/回读断言。a4f626226。
- 调度器for_home延迟迁移用例及真实旧SQLite夹具归入现有task-scheduler::schema_tests；宿主保留共享数据库路径/权限及跨存储集成。4b1c44508。
- 剩余residue 3243行：真实项目/租户/授权捕获、当前配置加载顺序、native资源清理、域错误至网关错误适配、稳定兼容入口及薄边界测试。逐符号索引见after-files.json，不以整个大文件“宿主耦合”批准保留。

## 证据与实际命令

从仓库根执行以下Python静态脚本，均正常结束；未执行任何Rust工具：

- `python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_final.py`：1381原路径与scope集合一致、唯一归属；逐路径before/after、声明、导出、cfg及所有者在after-files.json/remaining-host.json。
- `python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_registrations.py`：1116条词法cfg并集边，零悬空、零已删别名/孤立目标引用。当前外部Rust挂载 **178文件/56490行**，独立列task-59-external-current.json，不混入宿主物理门槛，不将路径在libs视作独立领域单测。
- `python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_test_candidates.py`：含原外部挂载，4159条基线标准测试声明，计数{"retained_same_path": 1995, "unique_name_candidate": 2136, "owner_evidence_candidate": 28, "ambiguous": 0, "missing": 0}。55条ignore属性无差异，见task-59-ignore-attributes.json；已覆盖test后附加属性，旧4091条统计已过时。名称/所有者候选映射不是宏展开或真实注册结果，也不证明全部断言语义等价。
- `python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_domain_changes.py`：136目标包逐路径baseline/current物理统计，包含同包其它修改，不计作本计划纯生产减量。唯一AppState名候选为基线已有gateway-types泛型契约；未见新增同名状态复制候选。
- `python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_alias_callers.py`：51处旧入口/领域目标/限定路径引用索引；词法引用不冒充编译解析，转发清理不重复计已有领域算法迁移。
- `rg -n 'vw-agent|vw_agent|vw-agent/src' crates/libs --glob Cargo.toml --glob '*.rs'`：结构化候选保存在remaining-host.json。CLI应用组合依赖、资源include、vw-agent-apps和日志target分开；未见字面path/include反向挂载宿主Rust。动态宏表达式不在证明范围。
- `git diff --check -- <task59文件>`：提交前通过。本轮不删除未经证明无用的依赖、不调整feature、不宣称全平台编译通过。

测试支持新模块若仍由真实Host注入运行，归类为“领域断言+宿主边界”，不称独立领域测试。外部源码挂载和test-support都单列，未取得计时证据，不承诺编译提速。

## 已查实的一项本轮生产调用

`vw-agent::cron::scheduler::run_agent_job` -> `agent_cron::scheduler::run_agent_job` -> `agent_cron::task_pool::deliver_agent_job_to_task_pool` -> `task_file_store::bootstrap_task::create_task`。实际分支在agent-cron/src/scheduler.rs:532，落盘调用task_pool.rs:21；任务54删除原多余Host转发。此项与本任务51处纯别名清理分开，不把现有任务池实现重复计成新迁移。其余原任务生产符号和真实调用按各task-N.md证据核对，尚未独立证明整轮完整语义映射闭合。

## 下一轮原所有者恢复

优先 **44、11、10、24、23**，随后 **53、15、48、37**。task-59-recovery-priorities.json已按最新文件重写，移除旧纯策略/已迁薄包装候选，列现存具体符号、路径/行号、拆分动作和真实保留边界。以下是当前整个原任务残留，不是可删除承诺；不能据此删除安全验收或复制AppState。

|优先级|原任务|当前原宿主行数|继续拆分的职责|真实保留边界|
|---|---:|---:|---|---|
|1|44|1663|Reconciliation ledger, terminal event/result expectations, cloud error/instance policy and shared transcript fixtures -> workflow-video-edit/workflow-engine actual entry; inject existing real native host execution for integration.|Keep actual native permission/path/hook denial and real gateway session publication; no simulated admission replacement.|
|2|11|1750|Reuse new stream_acceptance real StreamHost contract to separate collaboration message projection, template/config snapshots, title retry and replay/cancellation assertion bodies and fixtures; original host still calls the real stream.|Keep real tool execution, first-response authorization/directory rejection and cancellation at the actual gateway boundary; do not credit wrappers as independent domain tests.|
|3|10|1454|Remaining native/ACP fork history prefix/continuation and restart fixtures -> existing gateway-session fork acceptance contract, injected real process/stream/session ports.|Retain real independent process restart, actual gateway tenant routing, ACP/native first prompt and worktree publication calls.|
|4|24|1639|Separate persisted registration, revocation/cancellation response matrix, replacement/spec matching and client fixtures with the actual dynamic-tool storage/handler owner. Keep a thin real SDK/gateway executor binding.|Preserve real HTTP credential/session/project admission and client executor checks. Real-runtime mount cases are host boundaries, not removable solely because they are large.|
|5|23|1381|Split scripted model body assertions, exposure history and compaction checkpoint fixtures into actual session-engine/agent-loop acceptance support using existing real processor port.|Keep trusted tenant, real runtime tool/skill permission enforcement and host global config isolation; do not copy Request/AppState graphs.|
|6|53|1321|Separate mount source-name conflict/explicit-clear/publication revision expectation matrices into existing ACP mount acceptance contracts with real backend injection.|Keep actual native file access, revoked permission denial and real MCP process invocation; root backend prompt orchestration is real host composition.|
|7|15|1286|Move metadata/catalog/filter projection assertions and associated real storage fixtures to owning gateway catalog/misc domain handlers.|Keep actual host router registration, method mismatch/unknown-route checks and full root handler module assembly.|
|8|48|1084|Separate selected record/spec projection and revocation state/result expectations into app-mcp-runtime actual service test support; host injects actual ACP/loopback execution.|Keep actual tenant, account and job owner authorization, admitted-call lifetime and cached real runtime revocation integration.|
|9|37|1129|Split catalog notification refresh state/unchanged-catalog assertions into plugin MCP connection owner through actual connection service; narrow host inspection/context binding remains.|Keep actual runtime registration, session inspection and trusted plugin snapshot/permission checks.|

原所有者需要沿本轮已存在的真实Host/存储/协议入口继续拆分剩余断言与夹具，不新建单体或path借用原宿主测试；保留实际权限/租户/文件/进程/取消验收。候选含真实边界，不能保证仅这些任务就消除8584行，须据实际提交继续重算。任务59不越过原文件唯一归属替代其领域实现。

保留任务59直到门槛及其余归属缺口闭合，不宣称整轮完成。未来获授权再运行槽位包裹的真实注册list及原任务过滤验证；本次没有此授权。
