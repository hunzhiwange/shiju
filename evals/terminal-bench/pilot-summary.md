# 公开五题试跑与延期回归

最终验收已完成：五题3通过、2因max_tokens截断；5条有效官方评分、7项测试（5通过/2失败）。终态记录已修正，模型与预算未改。最终数据及历史分表见文末“最终延期回归验收”。

以下为初次试跑历史（当时状态：已完成，待回归）；最终有效验收结果见文末。2026-10-26，冻结五题各一次真实VibeWindow容器Gateway/SSE入口尝试均已执行，得到五条官方reward=0，其中四题有实际测试、一题verifier依赖失败且未执行测试。五次均属于基础设施失败；未获得有效模型作答、正常Gateway终态或token用量，不能据此估计模型准确率。此为legacy五题小样本接入取证，不是完整榜单或跨框架对照。

## 初次v3试跑结果（历史）

| 任务 | 官方reward | 通过/有效测试 | Gateway状态 | token | 总耗时（秒） |
| --- | --- | --- | --- | --- | --- |
| fix-git | 0 | 0/2 | failed，无终态 | 缺失 | 64.312 |
| git-multibranch | 0 | 0/1 | failed，无终态 | 缺失 | 175.992 |
| log-summary-date-ranges | 0 | 0/2 | failed，无终态 | 缺失 | 90.222 |
| regex-log | 0 | 未执行（依赖失败） | failed，无终态 | 缺失 | 166.034 |
| sqlite-db-truncate | 0 | 0/1 | failed，无终态 | 缺失 | 89.527 |

总计：5次尝试、5条官方reward（4条含实际测试）、6项有效测试、0项通过；5次基础设施失败均保留在总数中。5个独立容器，重试0，正常Gateway请求0。regex-log因archive.ubuntu.com下载超时导致curl/uvx缺失，未生成CTRF；其reward=0仅保留为基础设施失败，不算有效负例或零测试通过。Harbor CLI均退出0，不能替代Agent成功：真实适配器状态均为failed，CLI请求退出1，无chat.done或usage；原始events.jsonl为空，client.stderr保留发送请求失败，Gateway日志保留启动与执行证据。调用是否到达上游模型、token及费用均没有可验证证据，不能填0。耗时包含容器准备、Agent、官方verifier与清理，不是模型推理延迟。

## 身份与命令

- Harbor：0.24.0，提交6099b8cf2bfbbb3e7ff1eb2163bfc4524dbd9ba5。题库：e67a2cb7db4d9795c212909b3395d8c22ad60aec；各题Git tree与完整有效镜像ID见 [v3 manifest](calibration-manifest-v3.json)，冻结SHA256 `2aec7d66fe28331418caa20a2f00e4fdb98e44e4ba634be74051cfdf075d7356`。
- 适配器版本：`task29-harbor-0.24.0`，执行源码SHA256 `daecaef8f87ad2b2b21fb351e66ddd2fbac457402d484f1eca98f9dff43f185f`，绑定脚本SHA256 `1be73b393e627c1f4c70c1df900d804b819e3d809791b7a359ae1138308b75c8`。没有修改被测Rust Agent或评分器。
- 模型完整标识：`token-plan/deepseek-v4-flash-0731`。参数保持provider默认；Agent/verifier各900秒；1 CPU、2048 MiB、并发1、重试0。10 GiB为任务声明，未证明Docker强制磁盘配额。
- 默认features Linux ELF SHA256：`c7d7295c506c8aa996ff5a243841139fcd9ea640f21d241ec095d2d1c28ecf9d`；实际源码身份以任务26evidence为准，见 [README](README.md)。
- 五条完整展开的Harbor命令逐条保存在忽略目录 `results/terminal-bench/pilot/evidence.json` 的attempts.command；可复核的命令模板与最小profile准备流程见 [README](README.md)。仅传递instruction，未上传答案或tests给Agent；官方verifier阶段才上传tests。

## 基础设施失败与已突破的绑定缺口

本机冻结模型profile与API凭据存在且唯一。新增最小显式profile绑定，仅复制token-plan的api、adapter、一个模型metadata及API auth；不复制宿主数据库、完整配置或权限。真实容器PATCH /v1/config、PUT /v1/auth/token-plan和配置回读均通过。两个上传payload由trap移除，随机容器HOME在每题结束后清理；五题清理退出0。宿主0600临时profile在全部尝试结束后删除，日志未检出该凭据；原始产物不提交。API接受auth仅证明本地绑定，不证明上游模型鉴权成功。

进一步无模型探测复现：健康与配置回读返回200；GET /v1/provider后连接断开，后续健康不可达。增加仅用于取证的后台shell监督后捕获Gateway退出132和`Illegal instruction`（SIGILL），cgroup记录oom/oom_kill均为0。该证据来自同一固定fix-git有效镜像及同一ELF；能够确认冻结运行时在provider入口存在非法指令失败，尚未证明具体指令、库或架构兼容原因，也不把每题都宣称为已独立捕获SIGILL。五题直接证据均为SSE客户端发送失败、无终态/usage；SIGILL是额外的无模型定位证据。

Harbor对普通RuntimeError会跳过verifier；适配器将运行失败映射为其标准NonZeroAgentExitCodeError，保留失败状态及原始退出码，使官方verifier继续执行。没有把失败改为成功，也没有改评分阈值。

## 内部题集（独立口径）

| 题集 | 规模 | 结果来源 | 本次是否重跑 |
| --- | --- | --- | --- |
| 内部自建开发集 | 15题×3次/轮 | [baseline-summary](../promptfoo/baseline-summary.md) | 否 |

内部与公开题不合并分母，不以公开接入失败代替内部准确率基线。

## 初次验证与证据（历史）

原始输入、源码/工具载荷和日志只存Git忽略目录 `evals/promptfoo/results/terminal-bench/pilot/`；入口为 `evidence.json`、`verification.json`。每题Harbor trial有result.json、verifier/reward.txt、test-stdout.txt及agent下events.jsonl、client.stderr、gateway.log、result.json；四题有ctrf.json，regex-log缺失该文件；verification中保存各输出SHA256与评分路径。诊断见bootstrap-routes.txt、bootstrap-gateway.log；前期缺租户头/连接失败见本聊天工具记录，红绿日志保存在上述忽略目录。

| 检查 | 退出码与有效数量 |
| --- | --- |
| v3校准独立核验verify.py | 0；5对、10次历史尝试、14项测试（7通过/7预期失败）；本次未重跑参考解/空解 |
| 聚焦适配器红/绿 | 红：9项，1项错误；绿：9项全部通过；harbor-failure-red.log、adapter-green.log |
| 真实容器绑定探测 | 0；运行时安装、健康、两项API、配置回读5项；无模型评分 |
| bash -n evals/terminal-bench/bind-profile.sh | 0 |
| python3 evals/promptfoo/results/terminal-bench/pilot/verify.py | 1；取证审计通过：5次尝试、5条reward、4条有效评分、6项测试；接入验收失败，非通过 |
| cargo clippy --all-targets --all-features -- -D warnings | 0；有效Rust测试0；保留既有block 0.1.6 future-incompat提示 |

`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，动态领取实际槽位1，Clippy结束后已释放自己的锁并保留缓存。本次没有Rust行为修改，未新增Rust测试，未运行全量cargo test、Makefile、wasm检查或fmt检查。

初次试跑时的待办是修复冻结Linux运行时的SIGILL兼容问题及Ubuntu依赖下载可用性，再回归本任务尚缺的5条真实模型轨迹、终态/usage及5条有效官方评分；本任务不自动开新批量实验。正式对照须保持相同完整模型/参数、题目/镜像版本、权限、资源、预算和重试条件，并单列基础设施失败。

## 最终延期回归验收（2026-10-26）

状态：已完成。模型、provider参数、每题Agent/verifier各900秒、官方题目/评分规则均保持不变。最终独立五题各作答一次；5条有效官方评分、7项测试（5通过/2失败），任务通过3/5。regex-log和sqlite-db-truncate均发生真实max_tokens截断，保留失败，未增加预算或重跑模型。这不是完整榜单或跨框架胜出证据。

| 任务 | 最终有效reward | 官方测试通过/总数 | 终态 | 总耗时（秒） |
| --- | --- | --- | --- | --- |
| fix-git | 1 | 2/2 | 正常结束：stop | 177.570 |
| git-multibranch | 1 | 1/1 | 正常结束：stop | 755.742 |
| log-summary-date-ranges | 1 | 2/2 | 正常结束：stop | 172.819 |
| regex-log | 0 | 0/1 | 失败：max_tokens | 261.513 |
| sqlite-db-truncate | 0 | 0/1 | 失败：max_tokens | 269.627 |

耗时保留原模型尝试的Harbor全流程时间，包含环境准备与原verifier；log-summary的额外verifier恢复耗时单独列出，不冒充模型推理时间。

### 历史记录与验证恢复

| 阶段 | 模型尝试/评分状态 | 保留位置 |
| --- | --- | --- |
| 初次v3试跑 | 5次；入口基础设施失败，5条reward但仅4条有实际测试 | results/terminal-bench/pilot |
| v4目录排查 | 3次；SSE模型未找到，3条reward=0；后两题未启动 | regression/pilot |
| v5最终模型试跑 | 5次；2正常通过、2截断失败、log-summary原评分依赖502无效 | regression/final-pilot |
| 仅verifier恢复 | 不调用模型；恢复log-summary真实产物并执行原官方Verifier，2/2通过 | regression/verifier-recovery |

log-summary原verifier因Debian包下载502未执行测试，其原reward=0及Harbor结果不改写。恢复使用成功工具调用 `cat /app/summary.csv` 的完整350字节UTF-8输出（事件189，退出0），不使用最终自述、参考解或重算答案；原始读取文本、SHA256、来源事件及恢复过程有独立收据。新验证环境保留原基础层，原/app/logs树摘要完全相同，仅读取已记录CSV文本。完整身份、原官方测试摘要和校准见 [verifier-recovery-manifest.json](verifier-recovery-manifest.json)。修复镜像oracle/nop各2项测试全部通过/预期失败；正式评分调用冻结Harbor的原Verifier.verify，未使用其他Agent代跑。

原verifier耗时82.535秒，恢复验证14.754秒，合计97.289秒，未超过原900秒评分预算；恢复新增模型调用0。regex/sqlite已有有效官方断言失败，不作恢复、重试或产物修改。

### 终态与token记录

修正后的记录器按真实协议处理可选的chat.done.finish_reason：只有终态chat.done、最后已完成步骤stop/end_turn、CLI退出0、stream_end正常且无chat.error才记录正常结束。新步骤未完成、length/max_tokens及错误不被旧stop掩盖。前两题旧记录器误记failed；原结果及SSE完整保留，以同一个修正后解析函数离线回放另存收据，新增模型调用0。其余三题使用带执行版本/源码SHA256的新记录器；两个max_tokens终态仍为failed。

交付解析器SHA256 `d1747ee3147cb3afc88a3239be9bf6272b5148ba4f316e6f3eeadfa176dde4ec`；执行版本、原结果摘要、解析版本及归一化终态在final-pilot/terminal-replay逐题记录，回放修正数2。

五题token用量均无法可靠取得：存在实际文本/工具活动，原流报告全零或无usage。保留reported_usage原值，并将有效usage标记为缺失；不将0当真实token/费用，不估算费用。

### 修复与最终验证

SIGILL已由无凭据core及最小C复现定位为libgit2 SHA1DC未对齐读取触发Zig检查；只启用上游对齐访问宏，保留对齐检查与碰撞检测。新ELF在同一Rust/Cargo快照下构建，真实provider入口及后续health为200。APT修复沿用官方仓库、签名和校验过的CA bundle，不关闭验证。系统默认租户统一10000/100000000；实际目录准入使用/v1/provider，原/v1/models静态默认模型检查属于探测误判，已纠正。详见 [regression-runtime.md](regression-runtime.md)。

| 检查 | 结果 |
| --- | --- |
| 终态聚焦测试 | 16/16通过；红15项中1失败/1错误，绿日志terminal-green.log |
| SHA1DC最小C复现 | 原配置失败；对齐宏后通过且摘要与hashlib一致 |
| 新运行时默认features构建 | 退出0，179.34秒；源摘要前后一致，sidecars不变 |
| 最终cargo clippy --all-targets --all-features -- -D warnings | 退出0；有效Rust测试0；只保留既有block future-incompat提示 |
| final-pilot/verify.py | 退出0；5模型尝试、5有效评分、7项测试、3正常终态/2预算截断、1仅验证恢复、2终态回放 |
| 清理与隔离 | 5个唯一题目容器已回收，宿主profile已清理，无凭据泄露记录；原始轨迹/工具载荷仅在results忽略目录 |

`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`；构建和最终Clippy实际动态槽位1，结束后释放自己的锁并保留缓存。未修改生产Rust，未运行全量cargo test、Makefile、wasm或fmt检查。

最终运行身份见 [v5 manifest](calibration-manifest-v5.json)，实际展开的五条Harbor命令在regression/final-pilot/evidence.json；目录修复与新运行时的前驱为v4，正式仅验证恢复另有独立清单，不覆盖任一历史身份或原始评分。公共结果与上面的内部15题×3次基线仍分表；两题max_tokens失败保留为本样本结果。下一步可单独定位截断来源；本任务不自动改预算或启动额外付费实验。
