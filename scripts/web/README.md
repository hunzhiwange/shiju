# ShiJu Web @VERSION@

本压缩包包含完整 Web 前端所需的 HTML、JavaScript、WASM 与输入法桥接模块，无需 Rust 或重新编译。

## 本地启动

解压后，在本目录打开终端，执行：

```sh
npm run start
```

也可使用 `npm start`。启动脚本需要 Node.js 22 或以上版本（含 npm），仅使用内置模块，无需 npm install，只监听本机回环地址。
打开 <http://127.0.0.1:8765/>；按 `Ctrl+C` 停止。更换端口：

```sh
npm run start -- --port 8080
```

也可以将本目录交给已有的静态 HTTP 服务器，无需运行本脚本。

不要直接双击 index.html 使用 `file://` 打开，浏览器会阻止模块或 WASM 加载。
推荐使用支持 WebGPU 的新版 Chrome 或 Edge。本地 HTTP 回环地址可用；部署到其他主机时使用 HTTPS，并确保服务器以 `application/wasm` 返回 `.wasm` 文件，以 JavaScript MIME 类型返回 `.js` 文件。文件使用相对路径，也可部署到静态站点的子目录。

## 静态部署（无需 Node.js）

1. 点击桌面端“安装 Web 版本”中的“打开 Web 目录”，或解压下载的 ZIP。
2. 将目录内的 `index.html`、`shi_ju.js`、`shi_ju_bg.wasm` 和整个 `snippets/` 目录一起上传到静态服务器。保留文件名和相对目录结构；直接上传整个目录也可以。
3. 配置 `index.html` 为首页，`.js` 的 MIME 类型为 `text/javascript`，`.wasm` 为 `application/wasm`。缺失资源应返回 404，不能将 JS/WASM 请求重写成 HTML。
4. 使用 HTTPS 访问站点（例如 `https://example.com/shiju/`，子目录地址保留末尾 `/`），再配置网关连接。

静态服务器不需要执行 `npm install`、`npm start` 或 Rust 编译；`serve.mjs`、`package.json` 仅用于附带的本机 Node 启动方式。`README.md` 和 `.d.ts` 文件不参与页面运行。

例如 Nginx：将文件放入 `/srv/www/shiju/`，在已有 HTTPS `server` 中使用以下配置，并在 `http` 中加载标准 `mime.types`（须包含上述 JS/WASM 类型）：

```nginx
root /srv/www;
location /shiju/ {
    index index.html;
    try_files $uri $uri/ =404;
}
```

仅发布静态文件不会启动网关；远程访问时不能将浏览器里的 `127.0.0.1` 当作部署服务器地址。

## 网关与平台能力

静态启动可显示界面。代理请求等业务功能需要另行启动 ShiJu 网关服务；前端默认连接本机网关，也可在设置中配置网关地址和认证。跨域部署时需要网关允许对应的站点来源，不要把密钥写进静态文件。

浏览器不提供桌面原生账户存储、技能安装、桌面更新安装器或独立 MCP WebView 进程。工作流涉及的文件路径必须是网关服务器可访问的路径。

## 校验下载

与 ZIP 同时发布的 `.zip.sha256` 可在解压前核对：

```sh
sha256sum -c shiju-web-@VERSION@.zip.sha256
```

macOS 使用 `shasum -a 256 -c shiju-web-@VERSION@.zip.sha256`。
