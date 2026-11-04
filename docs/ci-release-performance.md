# 发布构建测量

基准入口为 `.github/workflows/release-benchmark.yml`，仅支持 workflow_dispatch，
固定 checkout 事件的 `github.sha`，没有 ref、命令或凭据输入。权限仅 contents:read，
不调用签名、公证、发布、npm、Homebrew 或 OSS。上传的是诊断日志与报告，不是发布资产。
工作流与矩阵均串行；避免和其他构建实验同时执行。

## 重现

将本工作流提交到可手动触发的分支后，在 Actions 的 Release benchmark 页面选择该分支。
保存 run 链接、完整 SHA、两个平台 artifact；相同 SHA、runner image、工具版本及缓存状态
才可比较。若缺任一平台、四包完成记录、五个 bin 或返回码，不得宣称完成验收。

沿用 cargo-dist 0.30.1 的发布安装器及 `dist plan` 的实际连线：

| 目标 | runner | 主机与目标差异 |
| --- | --- | --- |
| x86_64-apple-darwin | macos-15-intel | Intel 原生构建 |
| aarch64-pc-windows-msvc | windows-2022 | x86_64 主机交叉编译 ARM64 |

runner 安装 Rust 目标后执行：

```sh
python scripts/summarize_release_build.py --run x86_64-apple-darwin --output benchmark
# Windows 使用相同入口，目标替换为 aarch64-pc-windows-msvc。
```

入口自动从仓库 `target/rust-slot-locks/slot-1.lock` 至 `slot-10.lock` 原子领取首个可用
槽位，记录计划、任务、会话/run ID、PID，设置绝对路径 CARGO_TARGET_DIR，所有 Cargo
及 dist 子进程继承。结束、异常、SIGINT/SIGTERM 均先终止构建进程树，再清理自己的锁，
保留缓存。不删除他人的遗留锁。不可用额外目录绕过槽位，也不可并行测量共享 build-dir。
输出目录必须不存在，重复实验使用新的日志目录（编译仍复用动态领取的共享槽位）。

实际构建命令与发布的本地产物步骤一致，不传不存在的 timings 转发参数：

```sh
dist build --print=linkage --output-format=json --artifacts=local --target x86_64-apple-darwin
```

此步骤测量四包编译及 dist 的本地归档，不测量额外 bundle 脚本、安装包装配、签名、
公证或上传；不得将本次总时间与完整 Release job 总时间直接比较。
安装器与打包工具安装步骤的耗时分别从 Actions step 获取；打包工具冷次会运行 cargo install。

如需单独调查 CLI 的 Cargo timings，先按上述槽位规则动态领取并记录锁，再在同一
槽位生命周期中执行（Windows 替换目标）：

```sh
cargo build --profile dist --all-features --target x86_64-apple-darwin --package vw-cli --bin shiju --bin vibe-agent --timings
```

这不是 dist 归档测量：它显式选两个 bin，产生 Cargo timings，可能复用前一次编译缓存。
HTML 报告从本次槽位的 cargo-timings 目录定位。记录退出码，待所有使用槽位的进程结束
后释放锁。不得与 dist 时间混算。

## 报告与解释

`benchmark/` 包含环境、版本、SHA、dist build 帮助、Cargo metadata、dist plan、顶层
命令、原始 dist JSON、UTC 时间戳 stderr 日志与 `summary.json`。环境中记录 OS、架构、
CPU、内存、runner image/version、槽位、Cargo 实际目录、bin 清单、产物字节数及退出码。
即使构建失败，仍写报告并上传；构建非零退出码原样返回。

日志汇总可独立运行：

```sh
python3 scripts/summarize_release_build.py benchmark/build.log --exit-code 0 > summary.json
python3 -m unittest discover -s scripts/tests -p 'test_release_build_summary.py' -v
```

必须传原始构建退出码。汇总以 cargo-dist 0.30.1 的 `building ... --package=...)`
至 Cargo `Finished` 为一次包构建；重复包按 attempt 分开，合计仅累加完整记录。
缺失结束、重复结束、编译错误、空日志或失败退出均不能报告成功。
`command` 为依据固定 dist profile/all-features 配置和包头还原的 Cargo 命令，环境及
rustflags 仍以配置与 dist 原始证据为准；不是对任意 Cargo 日志的通用命令反推。

CLI `post_compile_seconds` 是末条 `Compiling` 日志至 `Finished` 的墙钟耗时，包含
编译、代码生成、优化、链接与调度等，**不是纯链接时间**；没有编译日志时为 null。
包耗时包含 Cargo 初始化和缓存判断，不能相加当作全部归档耗时，顶层另记 elapsed_seconds。

缓存模式为 `existing-slot-no-restore`：不恢复 Actions cache，也不清空既有槽位。
记录槽位初始文件数；空槽位也不能证明冷缓存，因为 build-dir、注册表和系统缓存可能热。
当前 `.cargo/config.toml` 指定 `target-dir = "target-dir"`、`build-dir = "target"`。
本机 Cargo 1.99.1 的 metadata 实测确认 CARGO_TARGET_DIR 覆盖 target_directory，
build_directory 仍为仓库绝对路径 `target`。槽位不会隔离该中间目录，禁止用新 build-dir
规避；真实 runner 以各自 metadata 和目录记录复核。稳定 Rust 通道和 hosted runner image
会滚动，版本不同的结果必须单列，不声称同环境加速。

## bin 与依赖审查

四个发布包为 vw-acp（acp）、vw-cli（shiju、vibe-agent）、vw-desktop（shi-ju）、
vw-webview（vw-webview）。本机 cargo-dist 0.30.1 `dist plan` 确认两个关注目标的 CLI
归档均包含 shiju 与 vibe-agent；其构建源码按 `--package vw-cli` 调用 Cargo，未传
`--bin`，因此 default-run=shiju 不会排除 vibe-agent。基准保存实际 plan 并校验五个 bin。

桌面直接依赖 `image = "0.25.9"`，锁文件解析为 0.25.10，默认开启 rayon 和
default-formats（avif、bmp、dds、exr、ff、gif、hdr、ico、jpeg、png、pnm、qoi、
tga、tiff、webp）。锁中可见 ravif、exr、tiff 等编解码依赖，可作为将来编译热点调查
候选；本任务不禁用格式、不修改依赖功能，也未证明它们是瓶颈。

## 历史参考与当前验收边界

来源：[历史 v0.5.18 run 41592780534](https://github.com/hunzhiwange/shiju/actions/runs/41592780534)。
下表为任务提供的历史片段，不是本次重新测量：

| 平台 | ACP | CLI | 桌面 | Webview | CLI 末条编译后尾段 |
| --- | --- | --- | --- | --- | --- |
| Intel Mac | 6m06s | 60m05s | 22m35s | 1m09s | 39m35s |
| Windows ARM64 目标 | 4m07s | 40m36s | 16m43s | 1m04s | 30m43s |

历史 Mac 工具安装为 13m42s。当前版本 0.5.19 与历史提交不同，缓存、runner image、
工具链、内存、跨编译方式和 bin 选择也须复核；不能据此承诺加速比例。
两真实平台的 run 链接、环境及四包时间尚待运行验证，本次本地解析测试不替代平台验收。

## 固定 macOS 打包工具

Release 和 benchmark 均调用 `scripts/install_macos_packaging_tools.sh`，固定
cargo-bundle 0.12.0、`--locked`，恢复后检查专用 root/bin 下的真实版本；
缺失或失配时强制安装，安装或版本核验失败立即失败。专用 root 为
`${{ runner.temp }}/macos-packaging-tools`，缓存键为
`packaging-tools-${{ runner.os }}-${{ runner.arch }}-cargo-bundle-0.12.0`。
不缓存整个 Cargo bin、密钥链或签名材料。安装时动态领取共享 Cargo 槽位，
工具 root/bin 写入 GITHUB_PATH，后续 cargo bundle 使用同一版本。

在同一 SHA、同宿主架构 runner 上运行 benchmark 两次，保存 Actions cache
命中记录、安装步骤耗时及版本输出；冷次应执行固定版本安装，热次应跳过安装。
该验证仍须真实 runner 运行，模拟测试和本机耗时不能替代 Actions cache 命中证据。
