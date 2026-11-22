"""Exercise the JavaScript inserted into the fixed iFlow Linux bundle."""
from pathlib import Path
import runpy
import subprocess
import unittest

PATCH = runpy.run_path(str(Path(__file__).with_name("apply-linux-mcp.py")))
PRELUDE = """
import assert from "node:assert/strict";
import x1 from "node:fs";
import y5 from "node:path";
import {tmpdir} from "node:os";
const shijuIFlowLaunch = {managed: true, endpoint: "http://127.0.0.1:12345", token: "fixture"};
const El = {invalidRequest: message => new Error(message)};
const Uht = {default: value => value};
const projected = (name, origin) => ({
    name, type: "http", url: "http://127.0.0.1:12345/shiju-vtcode-mcp/" + name,
    headers: [{name: "Authorization", value: "Bearer fixture"}],
    _meta: {"shiju.iflow.mcp-v4": origin},
});
"""


class LinuxMcpContract(unittest.TestCase):
    def javascript(self, body):
        subprocess.run(
            ["node", "--input-type=module"],
            input=PRELUDE + PATCH["LINUX_HELPERS"] + body,
            text=True,
            check=True,
        )

    def test_native_directory_is_data_and_only_host_projection_is_executable(self):
        self.javascript("""
const root = x1.mkdtempSync(y5.join(tmpdir(), "iflow-directory-"));
let native;
try {
    x1.mkdirSync(y5.join(root, ".iflow"));
    x1.writeFileSync(y5.join(root, ".iflow", "settings.json"), JSON.stringify({
        mcpServers: {native: {command: "must-not-start", env: {PRIVATE: "fixture"}}},
    }));
    native = shijuIFlowNativeDirectory(root);
} finally {
    x1.rmSync(root, {recursive: true});
}
assert.deepEqual(native, [{name: "native", scope: "project"}]);
assert.deepEqual(Object.keys(shijuIFlowProject([], native)), []);
const catalog = shijuIFlowProject(
    [projected("native", "project"), projected("selected", "mounted")], native);
assert.deepEqual(Object.keys(catalog), ["native", "selected"]);
assert.deepEqual(catalog.native, {
    type: "http", url: "http://127.0.0.1:12345/shiju-vtcode-mcp/native",
});
assert.throws(() => shijuIFlowProject([{name: "native", command: "fixture"}], native));
assert.throws(() => shijuIFlowProject([projected("native", "mounted")], native));
assert.throws(() => shijuIFlowProject([projected("other", "project")], native));
assert.throws(() => shijuIFlowProject([projected("native", "project"),
    projected("native", "project")], native));
const selected = projected("selected", "mounted");
assert.throws(() => shijuIFlowProject([{...selected, url: selected.url + "?extra"}], native));
assert.throws(() => shijuIFlowProject([{...selected, args: ["fixture"]}], native));
delete shijuIFlowLaunch.token;
assert.throws(() => shijuIFlowProject([selected], native));
""")

    def test_transport_disallows_direct_connections_and_redirects(self):
        self.javascript("""
let fetched;
const fetch = (url, options) => { fetched = {url, options}; };
class RM {
    constructor(url, options) { this.url = url; this.options = options; }
}
function connect(t, e) {
""" + PATCH["LINUX_TRANSPORT"] + """
    throw new Error("unexpected unmanaged transport");
}
const declaration = projected("selected", "mounted");
const relay = shijuIFlowProject([declaration], []).selected;
const client = connect("selected", relay);
assert.throws(() => connect("selected", {...relay}));
assert.deepEqual(client.options.requestInit.headers, {Authorization: "Bearer fixture"});
assert.throws(() => connect("selected", {type: "stdio", command: "fixture"}));
assert.throws(() => connect("selected", {...relay, url: relay.url + "/elsewhere"}));
assert.throws(() => client.options.fetch(relay.url + "?extra", {}));
client.options.fetch(relay.url, {method: "POST"});
assert.deepEqual(fetched.options, {method: "POST", redirect: "error"});
""")

    def test_dispatch_metadata_uses_only_the_approved_call_id(self):
        self.javascript("""
const args = {path: "fixture"};
assert.equal(shijuIFlowIdentity("native", "read"), "mcp__native__read");
assert.throws(() => shijuIFlowIdentity("native", "other/method"));
assert.throws(() => shijuIFlowCallParams({name: "read", args}));
assert.deepEqual(shijuIFlowCallParams({name: "read", args, shijuCallId: "call-1"}),
    {name: "read", arguments: args, _meta: {shijuCallId: "call-1"}});
""")

    def test_missing_or_mismatched_history_cannot_be_replaced(self):
        self.javascript("""
assert.doesNotThrow(() => shijuIFlowValidateHistory({sessionId: "same", chatHistory: []}, "same"));
assert.throws(() => shijuIFlowValidateHistory(null, "same"));
assert.throws(() => shijuIFlowValidateHistory({sessionId: "other", chatHistory: []}, "same"));
assert.throws(() => shijuIFlowValidateHistory({sessionId: "same", chatHistory: {}}, "same"));
""")

    def test_restore_retires_old_session_before_loading_replacement(self):
        self.javascript("""
class Adapter {
""" + PATCH["LINUX_RESTORE"] + """
    async shijuOriginalLoadSession(request) {
        assert.equal(this.sessions.has(request.sessionId), false);
        assert.equal(signal.aborted, true);
        assert.equal(stopped, true);
        assert.equal(shijuIFlowLiveServers.has(route), false);
        if (request.invalid) throw new Error("projection rejected");
        this.sessions.set(request.sessionId, {generation: 2});
    }
}
const adapter = new Adapter();
const old = new AbortController();
const signal = old.signal;
const route = shijuIFlowProject([projected("selected", "mounted")], []).selected;
let stopped = false;
adapter.sessions = new Map([["same", {
    pendingPrompt: old,
    getConfig: () => ({getMcpServers: () => ({selected: route}), getToolRegistry: async () => ({
        getMcpClientManager: () => ({stop: async () => {stopped = true;}}),
    })}),
}]]);
await assert.rejects(adapter.loadSession({sessionId: "same", invalid: true}));
assert.equal(adapter.sessions.has("same"), false);
await adapter.loadSession({sessionId: "same"});
assert.equal(adapter.sessions.get("same").generation, 2);
""")


if __name__ == "__main__":
    unittest.main()
