import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { runInNewContext } from "node:vm";

const path = process.env.SHIJU_KIMI_PATCHED_ENTRY;
assert.ok(path, "Set SHIJU_KIMI_PATCHED_ENTRY to the patched dist/main.mjs");
const source = readFileSync(path, "utf8");

function extract(start, end) {
    const first = source.indexOf(start);
    const last = source.indexOf(end, first + start.length);
    assert.ok(first >= 0 && last > first, "Missing pinned Kimi bundle boundary");
    return source.slice(first, last);
}

const environment = extract("function isShiJuManagedKimiLinux()", "function waitForSpawn(");
const bash = extract("function isBashToolInvocation(", "var OUTPUT_BYTE_LIMIT,");
const processClass = extract("AcpProcessService = class {", "AcpTerminalProcess = class {");

test("managed Linux local spawn strips both injected values after merging", () => {
    const process = {
        platform: "linux",
        env: { VW_ACP_LAUNCH_REFERENCE: "reference", VW_KIMI_RELAY_TOKEN: "fixture", KEEP: "old" },
    };
    const buildEnv = runInNewContext(environment + "\nbuildEnv", { process });
    for (const overrides of [undefined, { ...process.env, KEEP: "new" }]) {
        const result = buildEnv(overrides);
        assert.equal(Object.hasOwn(result, "VW_ACP_LAUNCH_REFERENCE"), false);
        assert.equal(Object.hasOwn(result, "VW_KIMI_RELAY_TOKEN"), false);
        assert.equal(result.KEEP, overrides?.KEEP ?? "old");
    }
    assert.equal(process.env.VW_KIMI_RELAY_TOKEN, "fixture");

    process.platform = "darwin";
    assert.equal(buildEnv(undefined), undefined);
    process.platform = "linux";
    delete process.env.VW_ACP_LAUNCH_REFERENCE;
    assert.equal(buildEnv(undefined), undefined);
});

test("managed Bash requires a host terminal and propagates host rejection", async () => {
    const AcpProcessService = runInNewContext(
        environment + bash + "let " + processClass + "\nAcpProcessService",
        {
            process: { platform: "linux", env: { VW_ACP_LAUNCH_REFERENCE: "reference" } },
            OUTPUT_BYTE_LIMIT: 1024,
        },
    );
    const connection = { terminalEnabled: false };
    let localCalls = 0;
    const local = { spawn: async () => { localCalls++; } };
    const process = new AcpProcessService("session", "/workspace", connection, local);
    const args = ["-c", "pwd"];
    const options = { env: { NO_COLOR: "1", TERM: "dumb" } };
    await assert.rejects(process.spawn("/bin/bash", args, options), /requires the host ACP terminal/);

    const denied = new Error("host approval denied");
    connection.terminalEnabled = true;
    connection.get = () => ({
        async createTerminal(request) {
            assert.equal(request.sessionId, "session");
            assert.deepEqual(Array.from(request.args), args);
            throw denied;
        },
    });
    await assert.rejects(process.spawn("/bin/bash", args, options), { message: denied.message });
    assert.equal(localCalls, 0);
});
