# 发布构建测量

基准入口为 `.github/workflows/release-benchmark.yml`，仅支持 workflow_dispatch，
固定 checkout 事件的 `github.sha`，没有 ref、命令或凭据输入。权限仅 contents:read，
不调用签名、公证、发布、npm、Homebrew 或 OSS。上传的是诊断日志与报告，不是发布资产。
工作流与矩阵均串行；避免和其他构建实验同时执行。

## 重现

将本工作流提交到可手动触发的分支后，在 Actions 的 Release benchmark 页面选择该分支。
保存 run 链接、完整 SHA、六个平台 artifact；相同 SHA、runner image、工具版本及缓存状态
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

本地入口自动从仓库 `target/rust-slot-locks/slot-1.lock` 至 `slot-10.lock` 原子领取首个可用
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

本地未通过工作流启动时，缓存模式为 `existing-slot-no-restore`：不恢复 Actions cache，也不清空既有槽位。
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


## 编译缓存连线与待验收事项

两个工作流使用 [actions/cache v4 的独立 restore/save](https://github.com/actions/cache/tree/v4)
显式缓存分离目录。[Cargo build cache 文档](https://doc.rust-lang.org/cargo/reference/build-cache.html)
确认 target-dir 与 build-dir 分别存放最终与中间产物；工作流先通过实际 Cargo metadata
核对 target_directory 为本次绝对槽位、build_directory 为仓库 target。不符合即失败，
不绕过 Cargo 新鲜度检查，不修改 .cargo/config.toml、LTO、features 或六平台矩阵。

缓存包含实际 CARGO_HOME 下的 registry/{index,cache,src}、git/{db,checkouts}，
以及本次槽位和 build-dir 的 dist、各目标/dist 下的 deps、build、.fingerprint、incremental。
仅编译目录映射到 `${RUNNER_TEMP}/release-cargo-cache/{slot,build}` 的固定路径；
恢复后复制到当前持有槽位与原 build-dir。此临时路径仅用于缓存传输，Cargo 不在其中构建。
不归档整个 target，不包含其他槽位、锁、distrib、bundle、安装包、签名 App、报告或凭据。
两个根目录即使某一方没有编译中间文件，也保持独立映射。日志输出映射文件数、字节数及复制耗时；
Actions cache 步骤另记录传输大小和耗时，尚无真实缓存体积数据。

键为 `release-rust-v1-{OS}-{host arch}-{targets}-dist-all-features-{config hash}-{lock hash}-{SHA}`。
配置散列包括实际 rustc -Vv、Cargo 配置、工具链声明、dist 配置、workspace profile 和环境 rustflags；
不包含动态槽位编号或工作区 package 版本。restore 前缀保留宿主/目标、工具链及构建参数，
允许 Cargo.lock 小变更或仅工作区版本变化后恢复旧依赖，由 Cargo 自行判断重编译。
目录绝对路径、runner image 或系统库变化仍可能影响新鲜度，不能只按命中键认定复用成功。

Release（含 PR）仅恢复，不写编译缓存。仅在受保护默认分支手动运行的 benchmark 成功后保存；
其余分支、tag、PR 均不写。依据 [Actions cache scope](https://github.com/actions/cache/tree/v4#cache-scopes)，
默认分支缓存对其他分支可见；不能将某个 tag 的缓存视为下一个 tag 可见的来源。
这里复用基准构建写缓存（批次 6 已扩为六目标），不新增单独预热；额外成本是 staging 复制和上传，
需计入净收益。默认分支未保护或尚未运行基准时，没有可信可恢复缓存，正常走未命中构建。
六目标均保持原有完整构建与打包，缓存收益须逐平台验收。

工作流在恢复前动态领取槽位，GITHUB_ENV 导出绝对 CARGO_TARGET_DIR，dist、工具安装、
打包与测量脚本共用该槽位，直到最终 always 清理步骤才释放。测量脚本校验 run ID、锁和路径，
不重复领取、不提前释放。取消时由 runner 终止步骤进程树后执行清理；强制终止 runner 可能遗留锁，
此时须按共享槽位规则核实进程及 run 已结束后人工清理，禁止直接删除未知锁。

远端验收步骤（尚未运行，不能宣称提速）：

1. 将提交推送到可手动触发的可信分支；先用 cache=off 在两个目标记录未恢复缓存时
   从基础依赖开始编译的真实日志。off 不清除 runner 预装缓存，需结合环境及 Compiling 行判断。
2. 在受保护默认分支，用 cache=restore 同 SHA 连跑两次，冷次建立缓存，热次检查真正减少的
   Compiling 行、四包时间、五个 bin、缓存键/来源/体积；净收益减去下载、复制、保存的全部开销。
3. 仅改变工作区 package 版本（同步对应锁文件）的分支运行相同基准，不改发布 tag，
   验证通过 restore 前缀命中默认分支缓存且第三方依赖确实复用。
4. 在隔离测试仓库通过 tag 触发同样的只构建、不发布入口，验证默认分支到 tag 的缓存恢复。
   当前基准允许手动选择已有 ref，但本任务不创建发布 tag，不运行正式 Release 验证缓存。
5. 比较实际 runner 上不同槽位绝对路径的缓存复用；若路径变化导致重编译，记录并调整方案，
   不篡改 Cargo fingerprint 或时间戳。补齐两平台及最终六平台的真实回归后才能完整验收。

目前只有本地接口测试、目录映射烟测及 clippy 证据；没有远端 run 链接、缓存下载/保存大小、
冷热净收益、版本变化和 branch-to-tag 运行证据。槽位映射方案的真实性能仍待上述验证。

## LTO 单变量入口（任务 4，待真实平台回归）

手动选择 `experiment=lto`；默认 `dist` 保持原行为。LTO job 不恢复或保存 Actions
编译缓存，不打包或发布；两个目标串行，各动态持有两个不同共享槽位，分别用于 thin/off。
所有 Cargo 命令使用对应绝对 CARGO_TARGET_DIR，仅通过 CARGO_PROFILE_DIST_LTO
改变优化配置。生产 profile 保留 thin，未采纳 off。

同 SHA、all-features、相同 bin，按 thin/off/off/thin/thin/off 各测三次。实际命令：
`cargo build --locked --profile dist --all-features --target TARGET -p vw-cli --bin shiju
--bin vibe-agent -p vw-acp --bin acp --timings`。
每次保存命令、退出码、墙钟时间、metadata、实际工具版本、runner image、槽位、三产物
字节数及 SHA256、原始 stdout/stderr 和 Cargo timings HTML。缺少 timings 即失败。
从 HTML unit timings 分别读取依赖、vw_cli 与二进制时间；并发 unit 时间不能相加视作
墙钟耗时，末条 Compiling 后尾段不能视为纯链接时间。

首轮为该模式首次构建，不保证冷缓存：两个槽位仍共享 target build-dir，registry 与 OS
缓存也可能热。后续切换触发的重编译须单列；Fresh/no-op 只能说明重复命令耗时，不能
作为稳定最终编译加速。不同 LTO 可能覆盖共享中间缓存，不假定双槽位完全隔离。
不清理缓存、不新增 build-dir，异常时先终止进程树再释放自己的锁。

Intel Mac 每次构建后原生执行三个 `--help`，以及 20 次 `shiju completions bash`，
保存逐次耗时与输出散列。该确定性离线负载仅代表启动、参数处理、补全生成；比较输出
一致性和耗时分布时须考虑启动噪声。它不足以证明代理核心 CPU 吞吐或全部发行功能一致。
Windows x64 主机只构建 ARM64，报告明确标记原生验证 pending；需在 ARM64 Windows
系统重建或转交产物执行三个 help 及相同离线负载。当前仅上传诊断材料，不上传可执行文件。

回归报告须补齐两个目标 run 链接、原始 timings、首次/重复/切换重编译分类、依赖与最终
crate 时间、净耗时、三产物体积、帮助输出差异和离线性能输入/次数/分布。六次构建的
实验成本单列，不计作发行收益。只有两目标净收益、原生行为一致及代表性 CPU 负载无
可测退化均有证据，才可采纳 off 并验证四发行 crate。目前未运行远端 A/B，无实测收益
或代表性性能数字，保留 thin，不宣称 LTO 已提速。


## precise-builds 评估（任务 5，待真实平台回归）

生产继续 `precise-builds=true`、thin LTO。0.30.1 本机 `dist build --help` 与
`dist plan --help` 已核实，没有传递额外 Cargo timings 的参数；源码
`cargo-dist-0.30.1/src/build/cargo.rs` 的 false 分支选择 `CargoTargetPackages::Workspace`，
实际传 `--workspace`。因此关闭 precise 不仅影响四发行包的构建次数，也可能构建未发行
workspace 成员；`dist=false` 约束归档选择，不能用它证明 Cargo 编译范围没有扩大。

本机两种配置的真实 `dist plan --output-format=json` 均退出 0，均为 67 个 artifact，
键集合相同。此证据仅覆盖计划，不代表真实归档内容、功能或原生链接一致。
Intel Mac 目标的 `cargo tree --locked --all-features --target x86_64-apple-darwin
--edges normal,build --prefix none --format '{p} features=[{f}]'` 分别选择四包及
`--workspace`，全部退出 0。四包输出分别 463/1966/1407/184 行，workspace 3328 行。
诊断中 workspace 的 ACP rustls 0.23.45 增加 ring/log/logging，桌面 reqwest 0.13.4
增加压缩/http2 等 features；CLI tokio 也出现 full/parking_lot 扩大。
Cargo tree 可能包含根包开发依赖带来的 feature 统一，以上只作风险线索；不把 tree 差异
直接等同于 dist 实际 rustc 的 cfg。必须复核构建日志、TLS provider 初始化及原生链接库。
CLI 的根 features 当前默认即包含全部命名项，删除 all-features 不是本次优化手段。

手动选择 `experiment=precise` 运行独立双目标串行 job。入口固定 SHA，记录实际 dist/Cargo/
Rust 版本、环境、两种计划、各包和 workspace feature 图。仅在 job 工作区临时切换
precise，finally 恢复配置；不修改生产工作流、不发布、不改 LTO。使用动态共享槽位，
完整生命周期持锁；中断先终止 dist/Cargo 进程树，保留缓存并释放自己的锁。
按 true/false/false/true 各测两次，保存带时间戳构建日志、原始 dist manifest、墙钟、
退出码、五个二进制大小/散列，校验每个目标的归档键与 bin 计划完全相同。
Mac 原生执行 shiju/vibe-agent/acp help；桌面/Webview 原生 smoke、安装包运行和
ARM64 Windows 原生验证明确待补，不能把 x64 Windows 交叉构建算作原生验收。

本入口不用四包日志汇总器判断 false 成功，因为 workspace 日志没有四个 package 头；
依据 dist 原始退出码和五个产物检查，不以减少 Compiling 行替代兼容性验收。
首次构建不保证冷缓存：槽位、共享 build-dir、registry 和系统缓存可能已有内容。
远端报告必须单列首次、重复和切换重编译，逐项检查实际新增编译成员、重复依赖、关键
路径、归档实际成员、链接库、TLS 行为和安装包；冷热净收益须含归档开销。
没有可信冷缓存证据时标为首次测量，不编造冷/热加速比例。
目前无远端 run 链接、冷热净收益或两平台完整原生兼容性证据，因此拒绝采纳生产合并，
保留 true。该决定保护既有功能，不能据此声称已完成性能验收或已取得提速。

## 六平台回归入口与最终验收报告（2026-11-04，待真实运行）

`experiment=dist` 现串行覆盖全部六目标：Intel macOS 使用 macos-15-intel，
ARM macOS 使用 macos-14，Windows x64/ARM64 使用 windows-2022，Linux x64/ARM64
分别使用 ubuntu-22.04/ubuntu-22.04-arm。LTO、precise 候选仍只测原两个关注目标。
使用原测量器的 benchmark 函数，使六目标复用同一日志、槽位和退出码检查；CLI 的
`--run` 仍只接受最初两个目标。Linux 安装与发行配置相同的系统开发库和 rpm；
Windows 固定 WiX 5.0.2；macOS 沿用固定 cargo-bundle 工具。每项工具安装步骤的耗时
由 Actions 单独记录，不能计入编译节省或从总成本中忽略。

测量后复用同一持有槽位内五个二进制：Linux 生成 DEB/RPM；Windows 以 SKIP_BUILD=1
生成 ZIP/MSI；macOS 以 SKIP_BUILD=1、adhoc、关闭公证生成 App/ZIP/DMG。
打包脚本缺失预构建输入会失败，不补跑隐藏编译。打包、校验完成后才保存缓存并释放锁。
`benchmark/packages.json` 记录产物字节数和 SHA256，Linux 另保存包元数据及安装路径清单；
诊断 artifact 不上传可执行安装包。后缀存在和散列记录只证明文件存在，不能替代人工或
平台工具核对归档成员、版本、目标架构、MSI 表、App 签名与 manifest 的最终验收。

本地聚焦回归为 44 项：43 通过、1 跳过。Linux 真实 DEB 测试因缺原生 Debian
工具跳过。发布测试的模拟 HTTPError 使用可关闭的 BytesIO 响应体，修复 Python 3.9
关闭 fp=None 夹具的兼容错误，不修改发布业务语义。四个 shell 脚本语法检查通过。
上述证据不代表任何真实发行目标完成链接。

当前没有六平台 run 链接、实际安装包验证或同提交冷热对照，最终净收益为“未测得”。
实际保留 thin LTO、precise-builds=true；off LTO 与合并构建均未采纳。
编译缓存与固定工具缓存已连线但收益仍待测，不能以 cache-hit=true 判定保留收益成立。
同 SHA 先以 cache=off 测基线，再以 cache=restore 建立缓存并重复运行；每个关注目标
须补齐至少一次可信冷运行和一次热运行。槽位或共享 build-dir 已有内容时标为首次运行，
不得称冷运行。逐项记录编译、工具安装、打包、缓存恢复/复制/保存及 job 总墙钟时间；
净收益比较 job 总墙钟而非叠加可能重叠的 step 时间。若缓存传输使总耗时净增，应回退
对应缓存 restore/save 与 staging 步骤；固定工具回退至前序版本须同时调整版本和缓存键。
回退 LTO/precise 实验只需删除对应候选 job，生产配置目前没有采纳它们。

真实运行入口已可通过登录浏览器使用；缺 gh 或连接 dispatch 工具不再视为阻塞。
已读取 [precise 基准运行 41774264864](https://github.com/hunzhiwange/shiju/actions/runs/41774264864)，
提交 57677e327f1c77379582cd4159d7de2b8c80e6bb，分支 codex/release-precise-benchmark。
2026-11-04 13:20（上海）观察到 Intel Mac 对照步骤仍在执行，Windows ARM64 为
Starting job；未有可下载完成报告。该运行跳过 measure/lto，不是六目标打包验收。
六目标实现提交 caac0219e 尚需单独带入审阅后的基准分支，不能推送混合业务提交。
运行参数为 experiment=dist，依次 cache=off、restore、restore，固定同一 SHA；
可信缓存保存还要求受保护默认分支，专用分支运行只能验证未命中或恢复已有可信缓存。
该默认分支变更需要维护者决策；不得为获得命中而降低保护、扩大缓存写入权限。
真实 Developer ID 签名、公证和 stapling 留待下一次授权发布，绝不以 ad-hoc 或模拟
故障测试宣称已通过真实外部服务。仍须补齐 ARM64 Windows 原生行为及全部包内容核验。


## 最新验收范围：仅 macOS

用户已明确取消本计划 Windows/Linux 验证要求。前述六平台入口及缺口为历史记录，
不再作为当前完成门槛。`experiment=dist` 矩阵现仅 Intel Mac（macos-15-intel，
x86_64-apple-darwin）与 Apple Silicon（macos-14，aarch64-apple-darwin），沿用
预构建二进制生成 ad-hoc App/ZIP/DMG；无需等待 Windows/Linux 产物。
LTO/precise 为此前候选实验，独立任务管理；当前打包回归只选择 dist。

本地 Mac 相关聚焦回归：测量器 11、工具安装 3、签名/公证故障模拟 16 项，合计 30 项
通过，退出码均 0；shell 语法通过。最新共享工作区 Clippy 退出 0，24.86 秒，
视频任务 Rust 编译错误未再出现。无真实 Developer ID 公证成功声明。

剩余真实证据仅为两个 Mac 目标的同提交构建、归档与 App/ZIP/DMG 内容/架构/版本/
manifest/checksum、冷/热时间及缓存成本。已有 precise Intel Mac 仍在执行，不能当作
打包回归完成。待其结束后，需将本批受审查提交单独带入基准分支，按固定 SHA 运行
experiment=dist、cache=off/restore/restore；不推混合业务提交、不擅自合并默认分支。
可信缓存保存仍受保护默认分支限制，需要维护者决定合入已审查入口并运行；专用分支
可以完成 Mac 构建/打包回归，但不能自行扩大缓存写权限来伪造冷热净收益。


任务 5 的验收也按用户最新指示缩小为 macOS：后续 `experiment=precise` 仅选择
macos-15-intel / x86_64-apple-darwin。原已启动的双目标运行保持 Mac 构建，
Mac 完成后取消剩余 Windows；不因 Windows/Linux 证据缺失阻塞 Mac 验收。
现有运行固定旧 SHA，不受后续矩阵修改影响；实际 true/false、归档和行为证据仍待收集。
