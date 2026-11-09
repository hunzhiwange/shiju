"""Check trusted MCP anchor clicks in a real Linux WebView on an isolated X11 display."""

import os
import shutil
import subprocess
import unittest

import test_mcp_app_network as network


class McpAppLinks(unittest.TestCase):
    setUp = network.McpAppNetwork.setUp
    launch = network.McpAppNetwork.launch
    send = staticmethod(network.McpAppNetwork.send)
    stop = staticmethod(network.McpAppNetwork.stop)
    message = network.McpAppNetwork.message
    app = network.McpAppNetwork.app
    handshake = network.McpAppNetwork.handshake

    def test_blank_target_click_uses_the_bound_rpc_bridge(self):
        if not (os.getenv("DISPLAY") and shutil.which("xdotool")):
            self.skipTest("Use an isolated X11 display with xdotool for a trusted click.")
        origin = "https://links.example.test:8443"
        url = f"{origin}/click?q=demo#section"
        process = self.launch(
            self.app("rpc('tools/call', 4, {ready: true});")
            + f"""<a href="{url}" target="_blank"
                     style="position:fixed;inset:0;display:block">Open external link</a>""",
            {"redirectDomains": [origin]},
        )
        self.handshake(process)
        self.assertEqual(self.message("tools/call")["params"], {"ready": True})
        window = subprocess.run(
            ["xdotool", "search", "--sync", "--onlyvisible", "--name", "^ShiJu MCP App$"],
            check=True, capture_output=True, text=True, timeout=5,
        ).stdout.splitlines()[-1]
        subprocess.run(
            ["xdotool", "mousemove", "--window", window, "40", "40", "click", "1"],
            check=True, timeout=5,
        )
        request = self.message("ui/open-link")
        self.assertEqual(request["params"], {"url": url})
        self.send(process, {
            "jsonrpc": "2.0", "id": request["id"],
            "error": {"code": -32000, "message": "External link opening denied by user"},
        })
        self.assertIsNone(process.poll())


if __name__ == "__main__":
    unittest.main()
