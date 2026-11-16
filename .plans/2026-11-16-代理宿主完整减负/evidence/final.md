# 任务59：最新静态收口与恢复清单

本报告替代旧67221行快照。当前宿主 **51915行**、971个Rust文件，基线135228行，净减83313行；距离35000行仍差 **16915行**。residue保留3317行。本次没有Cargo、构建、单元或实机测试；未来编译验证不是当前阻塞理由，也未声称编译提速。

## 本任务已落实

51个纯转发模块改为父模块直接领域别名，删除17个未注册memory文件和4个knowledge转发文件。生产算法新增迁移计数为零，不重复计已有领域算法。旧入口/领域目标/引用索引在 task-59-aliases.json、task-59-alias-callers.json；原可见性及外围cfg保留。必要相邻修复：router_tests显式路径，以及18条会话生命周期、9条项目作用域用例恢复测试属性，原断言未删。代码提交9c5a06db6；追加审计99ad057a3。

## 静态证据

- `python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_final.py`：1381个原始路径与scope集合相等、归属唯一；原路径before/after、符号/export/cfg在after-files.json；保留文件和原所有者在remaining-host.json。
- `python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_registrations.py`：1134条cfg并集词法边，0个悬空，已删转发目标零引用。模块/路径/include证据在task-59-registrations.json。
- `python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_test_candidates.py`：包含原外部挂载的4091条标准测试声明，{"retained_same_path": 2010, "unique_name_candidate": 2053, "owner_evidence_candidate": 28, "ambiguous": 0, "missing": 0}。逐条源路径、候选位置和所有者筛选见task-59-test-candidates.json；这是源码映射，不是宏展开、真实注册清单或语义等价证明。
- `python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_domain_changes.py`：目标领域包baseline/current逐文件物理统计在task-59-domain-files.json，涵盖同包其它改动，不冒充本计划纯生产减量。唯一AppState名候选为基线已有gateway-types泛型契约，未发现新增同名状态复制候选。
- 外部Rust源码挂载独立统计：180文件/57503行，见task-59-external-current.json。它们不计入宿主目录物理门槛，也不能仅因位于libs就声明领域独立测试注册；原所有者仍应按具体真实宿主边界核实。
- `rg -n 'vw-agent|vw_agent|vw-agent/src' crates/libs --glob Cargo.toml --glob '*.rs'`及结构化字面路径扫描：CLI应用组合依赖和资源include单列；未见字面path/include反向挂载宿主Rust源码。动态宏表达式不在证明范围，未把广义搜索的vw-agent-apps和日志target误报为宿主依赖。
- 本任务文件 `git diff --check` 无空白错误；不删除无法证明无用的Cargo依赖，不调整feature。各文件cfg原文可查，未冒充全平台编译通过。

## 下一轮原所有者恢复顺序

这不是外部环境阻塞。按计划，领域缺口由原任务恢复；不在task59改写领域单体。task-59-recovery-priorities.json给出每个具体路径、现有符号/行号、拆分动作和必须保留的真实边界。下表行数为整个原任务的物理残留，**不是可删除行数承诺**。所列符号是继续拆分审查点，不能仅迁走测试文件再由宿主path挂载。

|优先级|任务|现存行数|可继续拆分的职责|必须保留的边界|
|---|---:|---:|---|---|
|1|11|2252|Move model selection and override assertions to gateway-session with its actual domain entry; remove test-only wrappers once callers migrate.|Retain real HTTP/SSE producer, trusted session identity and concrete runtime request wiring.|
|2|37|2172|Separate MCP metadata merging, timeout classification and sanitized error assertions into plugin-mcp/agent-plugins tests; inject only explicit connection/process ports.|Retain actual host admission, current tool context, process shutdown and trust revocation integration.|
|3|10|2925|Separate profile serialization/default/backend projection and persisted-session expectations into agent-profiles/gateway-session owners using real stores; keep a thin selected-profile HTTP route assertion.|Retain actual gateway profile choice, tenant admission and real ACP/native end-to-end execution.|
|4|12|1298|Move persisted prefix, legal boundary and rollback state assertions into existing session fork/worktree domain tests using real stores; leave route extraction/admission checks.|Retain actual gateway tenant checks, real worktree setup failure and publication rollback integration.|
|5|23|1535|Separate compaction persisted-history and message checkpoint assertions into session-engine; reuse real message store and domain processor.|Retain concrete provider/catalog binding, trusted tenant and actual persisted host request execution.|
|6|53|2041|Separate recovery record decoding/ownership/revision and failure rollback into ACP mount domain suite; explicit filesystem/process publication ports only.|Retain real VibeWindowBackend dispatch, native skill permission/attachment access and real MCP restart integration.|
|7|44|1663|Move cloud protocol classification, unique-instance selection and reconciliation ledger assertions into workflow-engine using actual ledger/transport; keep native authorization calls as host boundary.|Retain native tool permission/path/hook enforcement, actual session stream completion and final-save integration.|
|8|48|1755|Split selected-record/project/account/job ownership matrix into app-mcp-runtime real storage tests; retain actual gateway credential extraction and ACP process invocation.|Retain actual HTTP authentication, cross-tenant denial and private execution boundary; do not simulate these as a replacement.|
|9|24|1779|Review dynamic-tool acceptance matrix and split persistent policy/registration replacement assertions into session-engine real storage tests; keep real HTTP client/executor handshake.|Retain actual SDK/socket, trusted tenant and client executor tool invocation.|
|10|50|1090|Move candidate stale/version comparison and persisted restore result assertions into existing worktree domain cleanup APIs.|Retain actual shell/runtime/session leases crossing removal and independent process restore.|
|11|54|1131|Move job/delivery result mapping and best-effort status assertions into cron domain; inject existing task/delivery ports.|Retain actual authenticated webhook sockets, reload credentials and real scheduler route composition.|
|12|38|1334|Split snapshot/revision validation and hook admission-result assertions into plugin-hooks/agent-plugins tests; preserve actual lifecycle ordering in host boundary.|Retain cancellation join-before-release and old MCP process retirement integration.|
|13|35|1365|Review tool-spec exposure/discovery/search assertions in tools/tests/mod_tests.rs; move pure registry policy matrix to tool-catalog/tool-assembly without copying host context.|Retain concrete AgentContextHost registration, filesystem admission and actual host-built registry behavior.|

优先恢复11、37、10、12、23、53；随后44、48、24、50、54、38、35。仅这些候选不能保证消除全部16915行缺口；原所有者须报告实际可迁部分与保留符号，调度者据新统计继续恢复，禁止以整文件“宿主耦合”豁免。生产真实调用完整语义映射仍需原所有者证据闭合，本任务只对其新增转发清理明确不计算法迁移。

任务59保留，直到物理门槛及归属缺口闭合。未运行验证不能写通过；未来获得授权后再用槽位入口列真实注册清单并按原任务过滤验证。
