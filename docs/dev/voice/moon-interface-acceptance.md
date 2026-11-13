# 月亮界面验收记录

日期：2026-11-13。状态：已完成（用户明确要求不再回归，按现有证据收尾）。仅文档更新，不新增产品行为，TDD不适用。

## 依赖与范围

批次1的产物修复、批次2授权修复（bf3625060）与批次3月亮界面（7cf2f2c6c）已实现；批次2、3的软件验收已完成；真人场景未执行，缺口按本记录保留。本轮串行执行，没有启动其它批次。不修改 plan.md/prompt.md，不提交现有屏幕分享等在途差异。

## 自动检查

日志 `/tmp/voice-moon-task4.log`。统一入口 `python3 scripts/cargo_slot.py sh -c '…'`，内部依次运行：

```sh
for filter in voice_moon_ voice_task_11_ voice_task_13_; do
  cargo test -p vw-desktop --lib "$filter" -- --nocapture --test-threads=1
  result=$?; echo "VERIFY $filter EXIT=$result"
done
python3 -m unittest discover -s scripts/tests -p test_voice_packaging.py -v
cargo test -p vw-desktop --test local-speech-system --
cargo clippy --all-targets --all-features -- -D warnings
MODE=app PROFILE=dev PACKAGE_APP=0 CREATE_DMG=0 INCLUDE_CLI_IN_APP=0 MACOS_SIGN_MODE=adhoc MACOS_NOTARIZE=0 bash scripts/bundle_macos.sh
```

实际槽位2，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，由入口管理锁，保留缓存。

| 检查 | 当前结果 |
| --- | --- |
| 首轮三个 lib 过滤测试 | 各退出101，执行0项；链接器报告 desktop 测试内部 drop_glue 等符号缺失，涉及 network/memory/agents 等设置测试 |
| 打包夹具 | 6项通过，退出0 |
| local-speech-system | 显式探测1次，退出0；不是 libtest 计数 |
| 全仓 Clippy all-targets/all-features -D warnings | 退出0 |

原生探测：`zh-CN`、`on_device=true`、`available=true`、`authorization=NotDetermined`、68个音色；合成24000Hz、77335个采样，取消86ms。没有请求系统授权，没有运行 `--transcribe`；这不证明真人识别或人耳可听。

## 未执行范围（用户取消剩余回归）

| 场景 | 本轮证据与缺口 |
| --- | --- |
| 首次 Speech/Mic 授权、已授权、拒绝/受限、返回刷新 | 未实机操作；自动权限状态测试不能替代系统弹窗结果，不重置TCC |
| 中文语言/音色选择、保存重开、应用重启回读、取消 | 文件事务与消息路由测试可自动覆盖；完整应用交互尚未执行 |
| 语言资源不可用 | 本机当前可用，未制造资源缺失环境 |
| 中文录音→同一会话普通模型→本机朗读 | 待本人说话、确认识别及可听；不需要云语音Key，不伪造模型响应 |
| 静音、打断、准备/录音/识别/朗读中结束 | 软件取消测试与完整应用设备释放分别验收；真人链路待回归 |
| 切换会话迟到结果、草稿、其他请求与审批 | 软件边界与原会话实际交互分别验收，后者待回归 |
| 明暗主题、窄窗口、中文整形 | 任务11明暗中文整形测试通过；月亮截图输出因目录缺失失败，完整应用未交互验收 |

## 补跑与产物

关闭增量编译重跑入口：`CARGO_INCREMENTAL=0 python3 scripts/cargo_slot.py sh -c 'for filter in voice_moon_ voice_task_11_ voice_task_13_; do SHIJU_MOON_SCREENSHOTS=/tmp/moon-task4-screens cargo test -p vw-desktop --lib "$filter" -- --nocapture --test-threads=1; result=$?; echo "VERIFY $filter EXIT=$result"; done; …'`。

日志 `/tmp/voice-moon-task4-retry.log`，仍为槽位2。链接成功，月亮组10项执行、9通过1失败、退出101；失败位于 `voice_chat_tests.rs:272` 保存截图：本轮命令未创建输出目录，非产品行为断言失败。任务11为21/21通过，任务13为14/14通过，均退出0。合计45项执行，44通过，1项截图写盘失败。

完整包 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/bundle/osx/ShiJu.app` 构建退出0，版本0.6.0，标识 `com.rsung.shiju`，包含 `shi-ju` 与 `vw-webview`（按任务1命令不含CLI）。两项用途说明存在；`codesign --verify --deep --strict` 退出0。主程序仅 audio-input entitlement，helper 为 audio-input 与 camera。没有启动或交互验收该包，也没有替换用户应用。

创建截图目录后启动最后一次补跑：`CARGO_INCREMENTAL=0 python3 scripts/cargo_slot.py sh -c 'SHIJU_MOON_SCREENSHOTS=/tmp/moon-task4-screens cargo test -p vw-desktop --lib voice_moon_ -- --nocapture --test-threads=1; result=$?; echo MOON_EXIT=$result; exit "$result"'`，领取槽位3，两目录均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。用户随后明确要求“不要回归，算完成了”，本轮停止自有入口，退出130，尚未执行测试，不计作通过。自有前台进程已退出、槽位3锁释放；槽位2前两轮入口均正常结束并释放自有锁，缓存保留。第一轮/补跑入口整体退出0仅表示最后命令完成，各验证实际退出码以上表与日志为准。

用户已取消剩余回归，按现有证据完成并删除任务4文件，不再安排下一轮。真人授权、中文录音、普通模型往返、听辨与完整App交互没有通过证据；本记录保留这些事实，不保存音频、转写、屏幕或凭据。plan.md/prompt.md保持不变。


## 任务2授权与设置的软件验收收尾

2026-11-13，异步等待、取消、迟到结果拒绝和偏好事务已实现，任务2软件验收完成。此前将真人听辨列为任务2阻塞的判断撤销：真人语音端到端验收属于任务4的实机场景，用户已要求任务4按现有证据收尾；该部分仍无通过结论。本次按最新指令核对验收范围后删除任务2文件，不把未执行的真人场景计作通过。

最新软件回归针对共享工作区83eedc373执行，之后至28d1d6311任务2相关生产源码和测试均无变化。确切命令：

```sh
python3 scripts/cargo_slot.py sh -c 'for filter in voice_moon_permission_ voice_task_03_ voice_task_07_ voice_task_13_; do cargo test -p vw-desktop --lib "$filter" -- --nocapture --test-threads=1; result=$?; echo "VERIFY $filter EXIT=$result"; if [ "$result" -ne 0 ]; then exit "$result"; fi; done; cargo clippy --all-targets --all-features -- -D warnings'
```

日志`/tmp/voice-permission-regression.log`：授权6项、权限5项、听写32项、设置14项，共57项通过、0失败，四组及整仓Clippy均退出0。实际槽位3，CARGO_TARGET_DIR和CARGO_BUILD_BUILD_DIR均为`/Users/xiangmin/worker/code/shiju/target/rust-slot-3`；入口已正常退出并释放锁、保留缓存。本轮仅核对证据和更新文档，没有重复运行未变化的检查。

自动测试覆盖延迟允许、授权终态、超时与取消、迟到结果丢弃、关闭表单不被重开、已取消听写，以及真实临时目录保存重载/取消不写盘/Local不访问网关。系统查询中Microphone=Granted、SpeechRecognition=NotDetermined只描述测试进程，不证明完整.app权限归属或授权失败。完整应用首次允许/拒绝、中文识别和听辨没有真人证据，仍由本记录的实机场景说明准确保留；本轮未请求系统授权、未录音、未改TCC。
