# 任务 108: 执行-chess-best-move

批次：【批次 66】；依赖批次：10、57。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：未开始

目的：取得 chess-best-move 一条原题与四条诊断题的有效首轮评分。

来源任务：Terminal-Bench2.1/chess-best-move

预计会话范围：一个父题的5次首轮评测；耗时受上游长超时约束，允许断线接续同一进程，不重启已发模型尝试。

## 文件

- 只读：`evals/terminal-bench/suite/experiments/qwen38-tb21.json`、`evals/terminal-bench/suite/cases/chess-best-move/case.json`
- 新增忽略产物：`evals/promptfoo/results/qwen38-suite/runs/chess-best-move/`
- 新建脱敏报告：`evals/terminal-bench/suite/reports/chess-best-move.md`

## 上下文

依赖准备任务18和冻结任务99，不依赖前一题能力分数。原题CPU1/内存2048MiB、Agent900秒/verifier900秒。父题先运行一次，再m1..m4各一次；新容器/HOME/session，不能把父题答案喂给小题。模型并发1，24轮/32工具仅为子题限额，不缩减原题。

不得修改旧评测、默认 Promptfoo 配置或 accuracy-manifest.mjs 的20例基线。不得用旧五题或旧 Qwen 两题成绩填充本轮。保持 token-plan/qwen3.8-flash、单次输出32000、实验输入窗口200000；不自动换模型、加预算、重复付费尝试。完整题保留官方题面、数据、参考解、评分、资源和各自超时。每例只运行一轮，整题、诊断、准入、校准与基础设施失败分栏；终态正常不等于官方评分通过。密钥只通过临时0600配置注入，禁止进入命令参数、日志、镜像或提交。

Cargo 不适用：本任务只读取固定工具或写文档/运行记录，不修改代码、不调用宿主 Cargo。官方容器内部上游工具使用隔离缓存，不挂载本仓库 target。

## 测试与验证计划

不适用测试驱动：执行已冻结评测，不编写或调整评分。运行前检验哈希与资源；有已运行attempt只使用resume，不重新run重发请求。运行/恢复CLI均为任务7已定义入口。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run run --manifest evals/terminal-bench/suite/experiments/qwen38-tb21.json --parent-id chess-best-move --output evals/promptfoo/results/qwen38-suite/runs/chess-best-move
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run resume --manifest evals/terminal-bench/suite/experiments/qwen38-tb21.json --parent-id chess-best-move --output evals/promptfoo/results/qwen38-suite/runs/chess-best-move
```

## 步骤

1. 先读冻结收据和已有输出，校验模型/预算/ELF/镜像/题面；已有任务只恢复，不并发新attempt。
2. 启动本题首轮，记录PID、进程所有权、attempt与容器；断线后用resume接续未开始子题。
3. 若官方评分依赖故障，保留失败原日志，在原预算剩余时间与既定恢复协议内处理相同产物；状态不可保全就记录未评分。
4. 写脱敏逐题报告：官方分数、诊断m1..m4分数、终态、截断/上下文/工具/环境归因及证据路径；不能把猜测写为已定位。
5. 确认5条有效评分才完成；基础设施阻塞不阻止其他独立题继续，最终覆盖仍不得虚报。

## 完成证据

5条独立有效评分、原始终态/finish reason、token使用/未知原因、累计评分时间与轨迹SHA；模型能力失败0分也是有效结果。未评分/基础设施失败必须保留并说明，不能称整题通过。

仅有编译通过、零测试或历史结果不足以完成。有代码、测试、配置或文档实际变更时使用 `$git-commit`，只提交本任务产物；完成后报告新证据并删除本编号文件。无关原因阻断验证时记为 `已完成，待回归` 并保留文件；确实无法突破时记为 `已阻塞`，只在本文件记录原因、检查与恢复入口。`plan.md` 只读。
