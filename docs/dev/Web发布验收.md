# Web 发布流程验收

日期：2026-11-11。

## 发布产物与入口

- Release 增加 `build-web-artifacts`，调用共享 `.github/workflows/build-web.yml`；Test 的 WASM job 复用同一构建、绑定和打包入口。
- 使用 `dist` profile 构建完整 `shi-ju` WASM，绑定工具版本从 Cargo.lock 读取并精确安装。输出 `shiju-web-<应用版本>.zip` 与 `.zip.sha256`。
- Web 构建成功是 Release host 的必要条件，产物沿已有 `artifacts-*` 下载及 `gh release create` 路径发布。恢复流程同样检查 Web job 结果和 ZIP 校验和，缺包或损坏时停止上传。
- ZIP 包含相对路径 HTML、JS、WASM、snippets、README 、`package.json` 和 `serve.mjs`，不包含 Python 文件。解压后执行 `npm run start`，无 npm 依赖、仓库或 Rust 环境要求。Node 服务仅监听回环地址。
- 页面静态运行；业务能力仍需连接 ShiJu 网关。源码构建使用的 Python 打包工具不属于下载包的运行依赖。

## 本地证据

- `python3 -m unittest discover -s scripts/tests -p test_package_shiju_web.py`：退出 0，5 项测试通过。覆盖归档内容、SHA-256、脱离源码的 Node 服务及 MIME、HEAD、路径边界、缺少 WASM、非法版本、符号链接，以及实际恢复校验代码对完整/损坏/缺失 Web 包的处理。
- `node --check scripts/web/serve.mjs`：退出 0。
- actionlint 1.7.12 检查 build-web.yml、release.yml、release-recover.yml、test.yml：退出 0。未运行可选 ShellCheck；已检查 Actions 表达式及调用关系。
- 使用已有完整应用 wasm-bindgen 产物 `/tmp/shiju-wasm-dist` 实际打包，验证 ZIP 校验和，并解压到 `/tmp/shiju-web-release-validation/relocated`；归档内没有 `.py` 文件。
- 从解压目录运行 `node serve.mjs --port 0`，执行 `scripts/tests/test_shiju_wasm.mjs`，退出 0。完整应用、底部账户菜单、设置页、中文 IME 搜索与选区替换均通过真实 Chromium 渲染验收，无应用 panic。
- 本地实际归档使用已有开发构建的 WASM 验证打包与静态启动；CI 的 dist 优化构建及 GitHub Release 上传尚未在线执行，不能据此声称已公开发布。

## Clippy 结果

已按仓库要求执行：

```sh
cargo clippy --all-targets --all-features -- -D warnings
```

动态领取槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，
`CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1/build`。

检查遇到无关改动：`crates/vw-agent/src/gateway/api/handlers/session/archived_delete_tests.rs:1`
导入了 `archived_delete::delete_order`，但对应函数为私有（E0603）。本次未修改这些文件；
按仓库要求不顺手修复其他任务的问题。报告该错误后中止等待其他编译任务，Clippy 退出 255，不计通过；自有进程已结束，槽位锁已释放、缓存保留。发布流程本次修改不涉及 Rust 源码。

## 下一步

将本次工作流及打包文件随正常版本提交、推送版本标签，检查 Release 的 Web job、ZIP 与 SHA-256 资产；下载正式优化包后执行 `npm run start` 复核。当前未创建或推送标签、未修改已有 GitHub Release。

## npm 启动入口

发布包新增 private package.json，版本随应用版本生成。`npm run start` 与 `npm start` 都直接运行包内 Node 静态服务器；无需 npm install。更换端口使用 `npm run start -- --port 8080`。脱离源码的启动测试已改为实际执行 npm 脚本，并检查 HTTP 资源与 MIME；原 Node 直接启动的浏览器证据保留为历史验收。

### npm 入口追加验收

- 先运行新测试，归档缺少 package.json 时出现 1 个错误、npm 启动断言失败；加入版本化 package.json 后，5 项测试退出 0。实际执行的是解压目录中的 `npm run start -- --port 0`，未执行 npm install。
- 重新生成实际应用 ZIP，归档内 package.json 的版本为 0.6.0，private 为 true。通过 npm 启动该包后，完整应用首页和底部账户菜单均正常渲染。
- 额外运行当前共享的完整浏览器脚本时，其他任务新增的“右下通知按钮”断言失败，脚本退出 1；该脚本检查的功能超出了本次 npm 入口改动，且本地复用的是此前生成的应用 WASM。未修改通知按钮代码或该共享测试，不声称本次完整 UI 回归全部通过。npm 入口、HTTP 和资源 MIME 的 5 项聚焦测试已通过。

- npm 入口修改后重新执行完整 Clippy：槽位 9，CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-9，CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-9/build；退出 101。无关归档删除接口在 `crates/vw-gateway-client/src/client/session_api.rs:466` 引用了不存在的 ArchivedChatDeleteResponse，并在下一行向三参数 delete_json 传了两个参数（E0425、E0061）。本次未修改这些 Rust 文件；槽位已释放。
