# 任务 37: 准备-financial-document-processor

批次：【批次 21】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“票据分类汇总”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/financial-document-processor

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/financial-document-processor/case.json`、`evals/terminal-bench/suite/cases/financial-document-processor/check.py`、`evals/terminal-bench/suite/cases/financial-document-processor/tests/test_cases.py`、`evals/terminal-bench/suite/cases/financial-document-processor/fixtures/`、`evals/terminal-bench/suite/cases/financial-document-processor/oracle/`、`evals/terminal-bench/suite/cases/financial-document-processor/environment/Dockerfile`、`evals/terminal-bench/suite/cases/financial-document-processor/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/financial-document-processor/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/financial-document-processor/`、`evals/promptfoo/results/qwen38-suite/diagnostics/financial-document-processor/`

## 上下文

原题 financial-document-processor；类别 data-processing；CPU=1、内存=4096MiB、存储=10240MiB、GPU=0；Agent=1200秒、verifier=1200秒。预建镜像 alexgshaw/financial-document-processor:20251031。原题树SHA=f461579cc917b5e02fb466d90ad8a63b208c4b01；题面SHA256=b21cf2af1891922732327bf586d14b250d36d96de7867479c15bd1c58ef0db58；配置SHA256=163ee252895c05d36ca37137dba4d6c24b46be28d56ce84ef8bb35bb5b894bda。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 识别合成票据与其他文档；m2 提取含税金额与税额；m3 处理字段冲突和缺失；m4 移动文件并核对汇总。

不得修改旧评测、默认 Promptfoo 配置或 accuracy-manifest.mjs 的20例基线。不得用旧五题或旧 Qwen 两题成绩填充本轮。保持 token-plan/qwen3.8-flash、单次输出32000、实验输入窗口200000；不自动换模型、加预算、重复付费尝试。完整题保留官方题面、数据、参考解、评分、资源和各自超时。每例只运行一轮，整题、诊断、准入、校准与基础设施失败分栏；终态正常不等于官方评分通过。密钥只通过临时0600配置注入，禁止进入命令参数、日志、镜像或提交。

## Cargo 共享槽位规则

1. 同一仓库的所有计划和任务共用仓库根目录下的 `target/rust-slot-1` 至 `target/rust-slot-10`，不得为任务、会话或计划另建构建目录，也不得按任务编号取模固定分配。
2. 每次构建/验证阶段开始前，创建仓库根目录下的 `target/rust-slot-locks`；依次用原子命令 `mkdir target/rust-slot-locks/slot-N.lock` 尝试领取槽位，`N` 为 1–10。仅 `mkdir` 成功者获得该槽位；这些相对路径命令从仓库根目录执行。
3. 领取成功后，在自己的锁目录中记录计划、任务、会话标识和持有进程 PID。同一槽位同时只允许一个持有者；已占用时尝试下一个，仅全部 10 个槽位都占用时等待并重试。
4. 将 `CARGO_TARGET_DIR` 设置为领取槽位的绝对路径，例如领取槽位 3 后，从仓库根目录执行 `export CARGO_TARGET_DIR="$PWD/target/rust-slot-3"`。示例编号不代表固定分配。
5. 所有 Cargo 构建、检查、测试以及调用 Cargo 的脚本和子进程均继承该设置，不得通过 `--target-dir` 或其他配置绕过槽位；生成的二进制也从该槽位定位。
6. 该阶段的 Cargo、脚本以及使用槽位产物的子进程全部结束后，删除自己写入的占用记录，并用 `rmdir` 释放自己的锁目录；保留槽位编译缓存供后续任务复用。
7. 正常结束、失败和中断均须清理自己的锁；在后台进程或其他使用槽位产物的进程仍运行时不得提前释放。规划清理步骤时覆盖这三种退出路径。
8. 遗留锁须先核实持有会话及相关进程已结束；无法确认则保持占用，不得删除其他活跃任务的锁、进程或缓存。
9. 生成或更新计划时，统一移除与这些规则冲突的单一 Rust 共享构建空间、逐任务独立目录和 Cargo 缓存全局独占窗口要求；仍须遵守源码文件占用、真实业务依赖和其他共享验证资源的互斥，不能仅因缓存分槽就把任务判定为可并行。
10. 最终验证记录包含实际槽位、绝对路径 `CARGO_TARGET_DIR`、确切命令、退出码和有效测试数量；Rust 代码修改后先运行 `cargo fmt --all`，再运行相应验证。编译成功或零测试不能替代行为验收证据。

仓库另设 build-dir：领取槽位后同时设置 `export CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR"`、`export CARGO_BUILD_JOBS=1`，防止 `.cargo/config.toml` 写入共享 target。纯 Python 改动无需格式化 Rust；若确有 Rust 修改先运行 `cargo fmt --all`。代码任务结束执行 `cargo clippy --all-targets --all-features -- -D warnings`；既有无关失败只记录证据，不修无关模块。禁止全量 cargo test、fmt --check、wasm 检查及 Makefile 命令。

## 测试与验证计划

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/financial-document-processor/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/financial-document-processor/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/financial-document-processor/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id financial-document-processor --output evals/promptfoo/results/qwen38-suite/preflight/financial-document-processor
```

## 步骤

1. 读取本题固定完整上游材料；索引仅导航，不能代替完整题面。
2. 逐一写m1..m4的合成独立题面、输入和黑盒判定；每题能单独理解，不能用父题参考解前缀或旧模型轨迹。
3. 写gold和至少3种错误实现，先证明checker缺陷的失败再修复；合成素材可提交，上游隐藏材料只留忽略目录。
4. 按task2策略受限启动原镜像，验证oracle/nop；四子题fresh环境各自校准。必要依赖修复限verifier-only且重新校准，不改官方checker，服务题不得伪造artifact回放。
5. 记录镜像、哈希、时间与校准收据；同批只占自己目录、最多2题合计4CPU/8192MiB；8GiB独占。共享builder修复必须串行。
6. 执行聚焦测试和Clippy门槛，按任务序号串行提交自己资产。

## 完成证据

完整原题oracle通过、nop失败；4子题各有oracle/nop与3负例；记录镜像digest、依赖/fixture/checker哈希、隔离与资源收据。不能校准则阻塞本题，不宣称已就绪。

仅有编译通过、零测试或历史结果不足以完成。有代码、测试、配置或文档实际变更时使用 `$git-commit`，只提交本任务产物；完成后报告新证据并删除本编号文件。无关原因阻断验证时记为 `已完成，待回归` 并保留文件；确实无法突破时记为 `已阻塞`，只在本文件记录原因、检查与恢复入口。`plan.md` 只读。


## 2026-10-28 执行记录与阻塞恢复

- 批次1–5由主会话确认已完成；本次 `suite.preparation.inputs` 重新核验冻结父题全部源字节、catalog、Harbor、runtime-v2 与最终诊断收据，退出0。plan.md及上游包未修改，无付费模型调用。
- 已准备四个独立合成转录诊断（分类、含税金额/税额、冲突/缺失、真实文件移动与CSV核对）。checker执行真实solution.py，含gold/nop及每题3个错误程序；OCR仅保留在完整父题，不以文本诊断替代。
- TDD：初始漏检checker实测26测试中21失败；另复现超长CSV未捕获异常（30测试中1失败）后修复。最终任务所列unittest命令退出0，30个有效测试；`suite.diagnostics validate --case evals/terminal-bench/suite/cases/financial-document-processor/case.json --run-fixtures` 退出0，20次新建离线容器校准。
- 最终checker另完成8次fresh Harbor子题校准，四对oracle/nop全部reward=1/0、scored且无exception；最终materialized checker哈希与本题check.py逐项一致。初版收据保留为历史，不当最终证据。
- 原镜像 `alexgshaw/financial-document-processor@sha256:ef6c9cfaaf14cdd200163008a188d5baa9f626f3bfb8d3e7d6f30c08518e6251`，amd64，4层压缩51790066字节、解压130614272字节；压缩SHA256、diff_id、本地RootFS均匹配。受限启动实测1CPU/4294967296字节；17个原文档哈希均与源匹配，无/tests或/solution泄漏。官方storage_mb=10240，未声称可写层磁盘配额。
- 父题preflight实际命令：`PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/financial-document-processor/runtime.json --parent-id financial-document-processor --output evals/promptfoo/results/qwen38-suite/preflight/financial-document-processor/harbor`。原Agent在apt update反复重试后，安装Tesseract所需35.1MB/73包仍未完成，真实异常为 `AgentTimeoutError: Agent execution timed out after 1200.0 seconds`；尚未执行OCR，父题有效官方测试数0，nop未运行。
- 按主会话追加指令：真实超时归档后立即SIGTERM自有runner（进程退出码-15），不继续漫长verifier安装。父题记unscored/infrastructure，不用raw reward0、晚产物或子题成绩宣称完整校准。未重复同条件父题运行，未预装Agent依赖、改时限/评分/模型/预算。
- 已做有界同源网络审计：原镜像无系统CA文件，原始curl HTTPS报77；只对独立审计容器只读挂载已验证CA后，Ubuntu HTTP/HTTPS与PyPI分别206成功，astral安装端点及GitHub Python端点仍SSL超时（curl28，5–10秒界限）。这些是依赖可达性证据，不是能力评分；未关闭TLS校验。修复Agent依赖/网络路径超出“仅verifier-only修复”范围，故保持阻塞。
- `evals/promptfoo/score.sh test` 退出0（配置/语法验证，无行为测试）；最终 `cargo clippy --all-targets --all-features -- -D warnings` 退出0。动态领取槽位1，`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR`同路径，`CARGO_BUILD_JOBS=1`。未修改Rust，未运行Rust行为测试；已有block0.1.6 future-compat提示保留。自有锁已释放，缓存保留。
- 本题所有父/子题、审计容器与网络及runner已清理；未动其他会话资源。未提交部分资产，保留本任务文件；未生成父题就绪preparation.json。
- 证据根：`evals/promptfoo/results/qwen38-suite/diagnostics/financial-document-processor/`。核心文件：`timeout-raw/exception.txt`、`timeout-raw/logs/agent/oracle.txt`、`preflight-command.json`、`network-audit-default-ca.json`、`network-audit.json`、`unit.log`、`validation.log`、`child-calibration-final.json`、`verified-inputs.json`、`acceptance-partial.json`、`clippy-final.log`。父题原运行目录为上述preflight输出。
- 恢复入口：先由维护者恢复同源依赖网络、证书与完整包下载能力；在保持原Agent/原镜像/1200秒时限不变的前提下，重新确认容量后用新的输出目录运行上述父题preflight并核验实际7个官方测试的oracle/nop证据。当前不得标为已就绪；最终checker发生过异常边界修复，旧preflight材料不可直接复用为最终完整准备收据。
