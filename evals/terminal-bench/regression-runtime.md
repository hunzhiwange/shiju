# 任务29延期回归：运行时与依赖修复

历史五次失败保留在 `results/terminal-bench/pilot/`；新回归在 `results/terminal-bench/regression/`，不覆盖历史或合并准确率。新身份见 [calibration-manifest-v4.json](calibration-manifest-v4.json)。题目指令、官方测试、模型完整标识、provider参数、权限及900秒预算保持原契约。

## SIGILL 的失败到通过证据

旧ELF `c7d7295c…` 在全新无凭据fix-git v3容器中健康/配置API成功，provider入口退出132。Rosetta core是ARM格式；独立QEMU 10.2.3也复现，生成x86 core。GDB定位工作线程到 `UD1 0x16(%eax),%eax`，前置指令检查4字节对齐；104字节指令片段在旧构建缓存中唯一匹配libgit2.a。不是仅凭CPU型号推断不兼容。

使用锁定的libgit2-sys 0.18.4+1.9.3所含SHA1DC源码，加 [runtime_alignment_test.c](runtime_alignment_test.c) 独立复现：故意从对齐数组的偏移1位置读取80字节。原配置退出134，明确报告sha1.c:391的未对齐uint32_t读取；上游已有宏 `SHA1DC_FORCE_ALIGNED_ACCESS=1` 使SHA1DCUpdate先复制到对齐buffer，退出0，摘要 `86f33652fcffd7fa1443e246dd34fe5d00e25ffd` 与Python hashlib一致。保留Zig对齐检查、SHA-1碰撞检测及默认功能，没有使用fno-sanitize或修改Agent生产逻辑。

诊断仅用独立容器，无凭据/模型；QEMU是定位工具，正式回归继续使用Docker原有amd64执行环境，未安装全局binfmt、切换宿主运行策略或增加试题容器权限。QEMU来源固定 `tonistiigi/binfmt@sha256:400a4873b838d1b89194d982c45e5fb3cda4593fbfd7e08a02e76b03b21166f0`，原始core/调试器输出和C测试命令只在results忽略目录。

## 重建命令与身份

先按 [runtime-image.md](runtime-image.md) 恢复任务25工具链并动态领取共享槽位，设置绝对CARGO_TARGET_DIR、记录持有进程、在全部子进程结束后释放自己的锁；下述命令继承该槽位，不新增Cargo目录。

```bash
TOOLS="$PWD/evals/promptfoo/results/task25/toolchain"
CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR" \
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_DEV_INCREMENTAL=false CARGO_BUILD_JOBS=4 \
RUSTC="$TOOLS/host-rust/bin/rustc" RUSTDOC="$TOOLS/host-rust/bin/rustdoc" \
CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$TOOLS/zig-amd64-cc" \
CC_x86_64_unknown_linux_gnu="$TOOLS/zig-amd64-cc" \
CXX_x86_64_unknown_linux_gnu="$TOOLS/zig-amd64-cxx" \
AR_x86_64_unknown_linux_gnu="$TOOLS/zig-ar" \
RANLIB_x86_64_unknown_linux_gnu="$TOOLS/zig-ranlib" \
CFLAGS_x86_64_unknown_linux_gnu=-DSHA1DC_FORCE_ALIGNED_ACCESS=1 \
cargo build -p vw-cli --bin shiju --target x86_64-unknown-linux-gnu --locked
```

本次动态槽位1，CARGO_TARGET_DIR为仓库绝对路径 `target/rust-slot-1`；构建退出0，179.34秒；Clippy全部targets/features退出0。构建前后Rust/Cargo摘要均为 `5f586a428553e7cf69e5dfd64e120cc49a62b6bc22130154237c56d16921bde5`，也与任务26原构建摘要一致。新ELF SHA256为 `3176b9c4804234d7e97aba05d0b6822d51cdf850f6a6b2e10c897ff9ce234a61`，sidecars不变。源码提交身份470278872加已记录工作树快照；源码差异留在忽略目录。ELF复制到regression/runtime后释放锁，保留缓存；后续容器不使用target产物。

同一fix-git v3镜像、1 CPU/2048 MiB、无网络、无凭据下，新的真实provider入口返回200（231个provider），随后health仍为200，build_id匹配新ELF；模型请求0。探测原始命令见regression/preflight.py，结果preflight.json。健康检查和此无模型入口验收不计作模型题目成绩。

## APT 修复与重新校准

只为regex-log追加 [regression-environment.Dockerfile](regression-environment.Dockerfile) 环境层，基础为v3不可变ID `sha256:92a8a137a502cd8eb6aaf61bbd8c5fae597d9f027a29a55412a343f281901605`。保持Ubuntu archive/security主机和仓库签名验证，URI从HTTP改为HTTPS；配置5次下载重试，预装curl。HTTPS首次失败的证据保留：旧镜像证书链不能验证当前站点。显式使用已在原runtime manifest固定并核对SHA256的CA bundle，不关闭TLS、签名或哈希检查。CURL_CA_BUNDLE及SSL_CERT_FILE同样指向此可信bundle。安装包完整版本清单存镜像 `/opt/regression-packages.txt`，运行时环境审计保存在results。

构建上下文仅有该CA bundle，既无题目、答案、tests，也无凭据。实际命令如下；本地基础标签先通过image inspect核对不可变ID，最终只使用iidfile中的不可变ID。

```bash
RESULTS="$PWD/evals/promptfoo/results/terminal-bench/regression"
docker tag sha256:92a8a137a502cd8eb6aaf61bbd8c5fae597d9f027a29a55412a343f281901605 vw-regression-regex-v3:temporary
docker buildx build --load --platform linux/amd64 \
  --build-arg BASE_IMAGE=vw-regression-regex-v3:temporary \
  -f evals/terminal-bench/regression-environment.Dockerfile \
  --iidfile "$RESULTS/regex-image-id" "$RESULTS/empty-context"
```

新镜像 `sha256:b74bd54954ef04b45a729b9b12bac7d741107a3b95b838047bb23671ba6a7d5f` 保留全部基础层；只替换Harbor task.toml中的docker_image，其他输入逐字节不变。新镜像oracle/nop分别reward=1/0，1项测试全部通过/预期失败，CLI均退出0、无异常；完整命令、原始测试和reward在regression/calibration/evidence.json及jobs。其余四题有效镜像仍为v3，复用其已通过的正反校准，不用旧regex-log校准认证新镜像。

## 适配与回归入口

适配器新增显式 `runtime_manifest`，仅读取新runtime摘要；模型必须与原冻结模型相同，题目与预算仍从原manifest读取。新增身份和拒绝换模型两项测试先失败后通过。另一个聚焦测试证明Harbor exec诊断可能混入stdout：适配器从已下载的原始events.jsonl解析，避免把exec诊断当JSONL；不剥离模型内容或更改评分。共12项聚焦测试通过。

五条回归命令与旧命令的实质差异是任务目录/新job名、runtime_dir指向regression/runtime，并增加：

```bash
--agent-kwarg "runtime_manifest=$PWD/evals/terminal-bench/calibration-manifest-v4.json"
```

完整展开命令、每题一次的尝试身份、轨迹/终态/usage、官方评分、耗时及清理结果在 `results/terminal-bench/regression/pilot/evidence.json`。与历史分表的最终结果见 [pilot-summary.md](pilot-summary.md)。没有通过增加权限、放宽oracle或调整模型/预算来提分。
