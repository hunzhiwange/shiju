# 五题公开终端接入试跑

这是固定 legacy 五题各一次的 VibeWindow 接入实验，不是完整 Terminal-Bench 榜单。结果见 [pilot-summary.md](pilot-summary.md)。内部15题×3次基线独立见 `../promptfoo/baseline-summary.md`，不可与公开题合并统计。

## 冻结输入与环境

使用 [calibration-manifest-v3.json](calibration-manifest-v3.json) 的 `effectiveImage`，不是原始预检manifest的镜像。Harbor 0.24.0提交 `6099b8cf2bfbbb3e7ff1eb2163bfc4524dbd9ba5`，题库提交 `e67a2cb7db4d9795c212909b3395d8c22ad60aec`。v3包含显式tmux环境修复与原版uv下载缓存，五对oracle/nop已校准；详见 [calibration.md](calibration.md)。镜像ID是本机不可变身份，跨机器须恢复并核对，不能以重建后的可变标签代替。

运行模型完整标识为 `token-plan/deepseek-v4-flash-0731`，参数保持provider默认。Agent/verifier分别900秒，串行、每题一次、重试0，1 CPU/2048 MiB、声明10 GiB磁盘；Docker未提供磁盘配额强制证明。Agent在官方cwd运行，题目指令、测试、答案、预算、评分阈值、权限均不改。

使用任务26已经验证、复制到results的Linux ELF及sidecars，默认vw-cli features。ELF SHA256为 `c7d7295c506c8aa996ff5a243841139fcd9ea640f21d241ec095d2d1c28ecf9d`；源码映射以 `results/task26/evidence.json` 为准（提交66dc4e094加工作树快照 `5f586a428553e7cf69e5dfd64e120cc49a62b6bc22130154237c56d16921bde5`），不能把本次工作树或原始manifest中更早的源码快照当作该ELF的构建来源。未重新构建或修改被测Rust Agent。

## 本次实际检查与准备

以下命令从仓库根目录执行。`results`不提交；冻结源码、Harbor虚拟环境、任务26运行时与v3镜像需先按 [preflight.md](preflight.md)、[runtime-image.md](runtime-image.md)、[calibration.md](calibration.md) 恢复。存在性、身份及校准检查失败时先修复环境，不生成替代成绩。

```bash
python3 evals/promptfoo/results/terminal-bench/calibration/cached-v3/verify.py
HARBOR_TELEMETRY=0 evals/promptfoo/results/task25/harbor-venv/bin/python \
  -m unittest discover -s evals/terminal-bench -p adapter_test.py
bash -n evals/terminal-bench/bind-profile.sh
```

本次准备将 `results/task26/image-shiju` 复制到 `results/terminal-bench/pilot/runtime/shiju`，四个sidecars从 `results/task25/toolchain/linux-libs-amd64/` 复制；适配器安装前逐一校验manifest的SHA256。五题目录从已校准的 `results/terminal-bench/calibration/cached-v3/tasks/` 逐文件复制到 `results/terminal-bench/pilot/tasks/`；包括task.toml在内摘要完全相同。原始官方源码和完整测试仅在忽略目录。

绑定profile文件仅包含以下结构，key必须由已有凭据安全读取，不能写进命令行、提交或运行日志：

```json
{"provider":{"api":"<existing endpoint>","adapter":"openai-compatible","models":{"deepseek-v4-flash-0731":{"status":"active"}}},"auth":{"type":"api","key":"<existing credential>"}}
```

本次从用户本地 `~/.shiju-agent/database/vibe-window.sqlite3` 只读查询system_configs，要求唯一匹配token-plan模型；仅复制api、adapter和该模型metadata。auth.json仅取token-plan的type/key。生成文件置于系统临时目录、权限0600，全部尝试结束后由TemporaryDirectory删除；Harbor参数仅含临时文件路径。未挂载或复制完整宿主HOME、数据库、其他provider或权限配置。

适配器在随机容器HOME启动Gateway，使用 [bind-profile.sh](bind-profile.sh) 通过 `127.0.0.1:42619` 的 `PATCH /v1/config` 与 `PUT /v1/auth/token-plan` 绑定。请求带与CLI默认相同的local/local租户头；仅传路径，payload不进入shell参数，关闭shell tracing，仅记录HTTP状态失败。上传的两个JSON由EXIT trap立即删除；每次运行结束，下载原始轨迹后删除整个随机运行时HOME。异常仍作为失败保存，并映射为Harbor的NonZeroAgentExitCodeError，让官方verifier继续评分；不是将失败改成成功。模型工具权限保持运行时默认，不自动同意审批。

无模型容器探测命令为：

```bash
HARBOR_TELEMETRY=0 evals/promptfoo/results/task25/harbor-venv/bin/python \
  evals/promptfoo/results/terminal-bench/pilot/bootstrap.py
```

该本地取证脚本及 `bootstrap.json` 留在忽略目录。它验证运行时安装、Gateway健康、两项绑定API和provider配置回读，不宣称已验证推理或得分。

## 五题实际执行命令

本次本地执行器 `results/terminal-bench/pilot/run.py` 在TemporaryDirectory内准备上述profile，串行执行下面命令并记录展开后的完整argv、起止时间、容器资源、输入输出摘要及评分路径到 `pilot/evidence.json`。临时profile的路径每次变化；以下PROFILE需指向按上述方式生成的0600文件，并在退出时清理。

```bash
RESULTS="$PWD/evals/promptfoo/results/terminal-bench/pilot"
for TASK in fix-git git-multibranch log-summary-date-ranges regex-log sqlite-db-truncate; do
  HARBOR_TELEMETRY=0 DOCKER_DEFAULT_PLATFORM=linux/amd64 \
  PYTHONPATH="$PWD/evals/terminal-bench" \
    evals/promptfoo/results/task25/harbor-venv/bin/harbor run \
    --path "$RESULTS/tasks/$TASK" \
    --agent vibewindow_agent:VibeWindowAgent \
    --model token-plan/deepseek-v4-flash-0731 \
    --agent-kwarg "task_id=$TASK" \
    --agent-kwarg "runtime_dir=$RESULTS/runtime" \
    --agent-kwarg "profile_path=$PROFILE" \
    --env docker --n-attempts 1 --n-concurrent 1 --max-retries 0 \
    --no-force-build --delete --jobs-dir "$RESULTS/jobs" \
    --job-name "$TASK-vibewindow-1"
done
```

Harbor的LiteLLM无法识别自定义模型前缀时会发出警告并将模型原样传给适配器；不改用其他模型。VibeWindow由容器Gateway+SSE实际处理题目，未使用其他内置Agent代跑。参考解 `--agent oracle` 与空解 `--agent nop` 仅属于此前独立校准，不能计作VibeWindow成绩。官方verifier在Agent阶段结束后上传tests并输出reward/CTRF；Agent自述、CLI退出0和Gateway终态均不能代替评分。

正式对照需同模型完整版本、provider参数、任务Git tree、v3环境身份、工具权限、900秒预算、并发与重试规则，并分别报告基础设施失败、缺失usage和统计不确定性。本次不自动启动额外付费实验。

## 延期回归（v4）

新运行时修复SHA1DC对齐访问，regex-log修复官方APT依赖下载；详细构建、依赖身份、红绿证据及重新校准见 [regression-runtime.md](regression-runtime.md)。使用 [v4 manifest](calibration-manifest-v4.json)、`regression/runtime`及`regression/pilot/tasks`，追加适配器参数 `runtime_manifest`，新job名为`<task>-vibewindow-regression-1`。本页上面的v3命令与历史失败继续保留；回归不会覆盖其产物，也不合并公共题与内部题的统计。
