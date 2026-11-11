#!/usr/bin/env python3
"""Serve wasm-bindgen's complete ShiJu build on the local loopback interface."""

import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import re


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dist", type=Path, help="wasm-bindgen --out-name shi_ju output directory")
    parser.add_argument("--port", type=int, default=8765)
    args = parser.parse_args()
    dist = args.dist.resolve()
    for filename in ("shi_ju.js", "shi_ju_bg.wasm"):
        if not (dist / filename).is_file():
            parser.error(f"Missing {dist / filename}; build and run wasm-bindgen first")

    template = Path(__file__).resolve().parents[1] / "crates/vw-desktop/index.html"
    html = re.sub(r"\s*<(?:base|link)\b[^>]*\bdata-trunk[^>]*>", "", template.read_text())
    # wasm-bindgen invokes the binary entry point while initializing the module.
    # Calling the exported main again would initialize Iced twice.
    bootstrap = """<script type="module">
import init from './shi_ju.js';
init().catch(error => {
    window.shijuInitError = String(error);
    console.error(error);
});
</script>"""
    (dist / "index.html").write_text(html.replace("</body>", bootstrap + "\n</body>"))
    handler = partial(SimpleHTTPRequestHandler, directory=str(dist))
    with ThreadingHTTPServer(("127.0.0.1", args.port), handler) as server:
        print(f"ShiJu WASM: http://127.0.0.1:{server.server_port}/", flush=True)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass


if __name__ == "__main__":
    main()
