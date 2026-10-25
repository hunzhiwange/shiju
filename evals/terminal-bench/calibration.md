# 公开任务评分器校准

实验使用 [calibration-manifest-v3.json](calibration-manifest-v3.json)。原始 [manifest.json](manifest.json) 仍是未修补的预检身份；后续公开运行须显式选择 v3 的 `effectiveImage`，不能用原始镜像的预检成功代替校准。固定题库与 Harbor 提交、五题 Git tree、输入摘要、原始官方镜像均保留在 v3 中。本实验不调用模型，不产生 VibeWindow 准确率或正式榜单成绩。

## 环境修复与身份

首轮原始环境中，`fix-git` 的 oracle/nop 分别 reward=1/0，2项测试全部通过/全部失败。`git-multibranch` 的官方 Dockerfile 未安装 tmux，而参考解直接调用 tmux，oracle 退出127；verifier 又遇到 Ubuntu 安全源 HTTP502，reward=0。此结果属于环境失败，不能算有效负例。原始三次尝试和日志保留，不覆盖。

用户随后授权修复环境及重试下载。[v2](calibration-manifest-v2.json) 仅为 `git-multibranch` 增加 [calibration-environment.Dockerfile](calibration-environment.Dockerfile) 环境层：安装 tmux `3.4-1ubuntu0.2` 及记录版本的两个依赖，设置 APT `Acquire::Retries=5`。没有改变软件源、TLS/签名校验、任务指令、参考解、测试、评分阈值、权限或超时。其他四题仍使用原官方 digest。修复镜像以本地不可变 ID 固定，未推送注册表；复制到另一机器前须恢复该镜像并核对身份，重新构建产生不同身份时须发布新版本并重校准。

构建上下文为空，没有复制题目、答案、verifier、仓库或凭据。真实构建命令如下（仓库根目录）：

```bash
RESULTS="$PWD/evals/promptfoo/results/terminal-bench/calibration/recovery"
mkdir -p "$RESULTS/context"
docker buildx build --load --platform linux/amd64 --network default \
  -f evals/terminal-bench/calibration-environment.Dockerfile \
  --iidfile "$RESULTS/patched-image-id" \
  -t vw-calibration-git-multibranch:v2 "$RESULTS/context"
```

## 精确依赖缓存（v3）

v2 首题参考解再次通过2/2，但每个新容器重复下载21 MiB的uv包耗时数分钟。取消了尚未完成的v2空解尝试，不计成绩。v3在五个环境的镜像层中缓存原版uv 0.9.5压缩包，通过官方安装器支持的`UV_DOWNLOAD_URL=file:///opt/calibration-downloads`读取；测试仍执行原始安装器和命令，不使用替代uv版本、不跳过安装或测试。无共享可写缓存、无本地服务或代理。

[官方GitHub release API](https://api.github.com/repos/astral-sh/uv/releases/tags/0.9.5)记录压缩包大小21370871字节、SHA256 `2cf10babba653310606f8b49876cfb679928669e7ddaa1fb41fb00ce73e64f66`；下载后及镜像构建中均校验。原版安装器在断网、全新容器中实测安装成功且`uv --version`为0.9.5。缓存不包含答案、测试或Agent产物。安装器仍从官方HTTPS地址获取，其余Python/pytest依赖仍按原测试脚本联网安装。

缓存镜像使用 [calibration-cache.Dockerfile](calibration-cache.Dockerfile)。构建命令见忽略目录`recovery/cache-builds.json`，五次成功构建均为`docker buildx build --load --platform linux/amd64 --network none --build-arg BASE_IMAGE=<固定基础镜像> -f evals/terminal-bench/calibration-cache.Dockerfile --iidfile <结果路径> <仅包含官方uv压缩包的context>`。git-multibranch基础镜像为v2修复镜像；BuildKit不接受裸本地sha256作为FROM，首个构建因此失败，改用已核对ID的本地标签后成功，其余四题直接使用官方digest。运行只使用v3记录的不可变ID。v2和v3均保留为显式实验身份，v2结果不能认证v3镜像。

## 运行与验收

完整输入与日志只存于 Git 忽略目录 `evals/promptfoo/results/terminal-bench/calibration/`。首轮入口 `evidence.json`；v2 入口 `recovery/evidence.json`；最终 v3 入口 `cached-v3/evidence.json`，每次尝试另有 Harbor trial `result.json`、`agent/oracle.txt`、`agent/exit-code.txt`（非零时）、`verifier/test-stdout.txt`、`reward.txt` 和 `ctrf.json`。原始命令、开始/结束时间、退出码、容器镜像与资源、Docker exec 事件均在该目录。日志含上游源码/工具载荷，不提交。

各题从冻结源码复制到 `cached-v3/tasks/<taskId>`，仅替换 task.toml 的 `docker_image` 为 v3 `effectiveImage`，其他文件逐一 SHA256 对照原始 Git tree。每次尝试均新建容器，保留官方 cwd、1 CPU/2048 MiB、900秒 Agent/verifier 超时，串行执行，无额外宿主挂载。Harbor/Docker 未提供此实验的10 GiB磁盘配额强制证明，任务声明未变。

最终十次尝试的完整命令如下，顺序与实际执行一致，每题每种Agent仅1次。原始绝对路径展开命令另见 evidence 的 `command` 字段。执行器仅对环境错误允许重建重试，最多3次；本轮未触发重试。

```bash
RESULTS="$PWD/evals/promptfoo/results/terminal-bench/calibration/cached-v3"
for TASK in fix-git log-summary-date-ranges regex-log sqlite-db-truncate git-multibranch; do
  for AGENT in oracle nop; do
    ATTEMPT=1
HARBOR_TELEMETRY=0 DOCKER_DEFAULT_PLATFORM=linux/amd64 \
  evals/promptfoo/results/task25/harbor-venv/bin/harbor run \
  --path "$RESULTS/tasks/$TASK" --agent "$AGENT" --env docker \
  --n-attempts 1 --n-concurrent 1 --max-retries 0 \
  --no-force-build --delete --jobs-dir "$RESULTS/jobs" \
  --job-name "$TASK-$AGENT-$ATTEMPT"
  done
done
```

验收同时要求：oracle自身退出0、有效测试数大于0、全部通过且reward=1；nop有效测试数大于0、任务断言失败且reward=0，无安装/网络/命令缺失等环境错误。Harbor CLI退出0并不证明oracle通过。失败题停止后续跑分，其他独立题继续。任何未满足正反校准的题均不得进入公开成绩。

## 隔离证据

五个有效镜像在全新容器中探测 `/tests`、`/solution`、`/logs/verifier/reward.txt`、`reward.json` 均不存在，`uname -m` 和 cwd 匹配。冻结 Harbor 的已安装 oracle/nop/verifier/环境/调度源码与源码仓库逐字节相同；nop不上传答案，tests在Agent结束后由verifier上传。oracle阶段需要读取参考解属于正例构造；被测Agent没有参与，也不获得答案或tests。每次尝试使用独立容器与reward输出，没有复用已完成的工作目录或答案。此证据不宣称对恶意Agent提供强沙箱隔离。

## 结果

2026-10-26，五对正反校准全部通过。最终v3共10次尝试，均首次成功，无重试、跳过或零测试：参考解7项测试全部通过，空解7项测试全部失败；10次Harbor CLI与verifier脚本均退出0，5次oracle均退出0。verifier脚本在pytest失败后写入reward=0仍退出0，因此以CTRF实际断言结果与reward联合判定。

| 任务 | oracle reward | 正例通过/测试数 | nop reward | 负例失败/测试数 |
| --- | --- | --- | --- | --- |
| fix-git | 1 | 2/2 | 0 | 2/2 |
| git-multibranch | 1 | 1/1 | 0 | 1/1 |
| log-summary-date-ranges | 1 | 2/2 | 0 | 2/2 |
| regex-log | 1 | 1/1 | 0 | 1/1 |
| sqlite-db-truncate | 1 | 1/1 | 0 | 1/1 |

最终冻结manifest SHA256：`2aec7d66fe28331418caa20a2f00e4fdb98e44e4ba634be74051cfdf075d7356`。各题Git tree及指令、task.toml、Dockerfile、test.sh摘要在manifest中；参考解与完整tests文件摘要在忽略目录的inputs中。每次输出的SHA256、精确命令、原始测试日志和容器证据在`cached-v3/evidence.json`，独立核验摘要为`cached-v3/verification.json`。

完成检查：

```bash
python3 evals/promptfoo/results/terminal-bench/calibration/cached-v3/verify.py
```

退出0；5对、10个唯一容器、14项有效测试、7通过/7预期失败、模型调用0、遗留题目容器0。核验还覆盖manifest版本链、所有非task.toml输入未变、环境基础层保持、镜像平台与资源、官方缓存摘要、verifier实际exec退出事件、原始日志被忽略及只读plan.md未变。原始缺tmux/HTTP502失败日志与修复后成功日志均保留；不把原始环境失败计入这五对成绩。

没有修改Rust或被测Agent；本次是文档、冻结清单和容器环境实验，不调用Cargo或全量测试。后续任务应显式使用v3 `effectiveImage`及对应冻结输入；本任务不自动启动后续跑分。
