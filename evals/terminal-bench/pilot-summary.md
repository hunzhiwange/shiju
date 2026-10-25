# 公开五题试跑结果

状态：已完成，待回归；有效接入验收未通过，编号任务文件保留。2026-10-26，冻结五题各一次真实VibeWindow容器Gateway/SSE入口尝试均已执行，得到五条官方reward=0，其中四题有实际测试、一题verifier依赖失败且未执行测试。五次均属于基础设施失败；未获得有效模型作答、正常Gateway终态或token用量，不能据此估计模型准确率。此为legacy五题小样本接入取证，不是完整榜单或跨框架对照。

## 公开题结果

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

## 验证与证据

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

下一步先修复冻结Linux运行时的SIGILL兼容问题及Ubuntu依赖下载可用性，再回归本任务尚缺的5条真实模型轨迹、终态/usage及5条有效官方评分；本任务不自动开新批量实验。正式对照须保持相同完整模型/参数、题目/镜像版本、权限、资源、预算和重试条件，并单列基础设施失败。
