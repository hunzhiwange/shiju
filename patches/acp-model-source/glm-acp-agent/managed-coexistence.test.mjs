import assert from 'node:assert/strict';
import { execFile, spawn } from 'node:child_process';
import { once } from 'node:events';
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Readable, Writable } from 'node:stream';
import { test } from 'node:test';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { promisify } from 'node:util';

const exec = promisify(execFile);
const installer = new URL('../../../crates/libs/acp-agent-registry/src/managed_runtime/', import.meta.url);
const patch = new URL('./glm-acp-agent-1.17.1-skills.patch', import.meta.url);
const linuxPatch = new URL('./glm-acp-agent-1.17.1-linux-host-mcp.patch', import.meta.url);
const linuxContract = new URL('./linux-host-mcp.json', import.meta.url);
const managedLinux = process.platform === 'linux' && ['x64', 'arm64'].includes(process.arch);
const packageName = 'glm-acp-agent';
const tools = ['fixture_lookup', 'fixture_error', 'fixture_denied', 'fixture_other'];
const toolSchema = { type: 'object', properties: {} };
const projectionToken = 'fixture-projection-token';
const exposed = name => managedLinux && name.startsWith('fixture_')
    ? `mcp__${name === 'fixture_sibling' ? 'b' : 'a'}__${name}`
    : name;

async function install(directory) {
    const assets = join(directory, 'assets');
    const runtime = join(directory, 'runtime');
    await mkdir(assets);
    await copyFile(new URL('contracts.json', installer), join(assets, 'contracts.json'));
    await copyFile(patch, join(assets, 'glm-acp-agent.patch'));
    if (managedLinux) {
        await copyFile(linuxContract, join(assets, 'glm-linux-contract.json'));
        await copyFile(linuxPatch, join(assets, 'glm-linux-session-model.patch'));
    }
    // The production installer checks the npm integrity and both sets of file hashes.
    await exec('python3', [fileURLToPath(new URL('install.py', installer)), assets, runtime, packageName,
        ...(managedLinux ? ['', '--glm-linux-v4'] : [])], { timeout: 80_000 });
    return join(runtime, 'node_modules');
}

function sendModelTurn(response, calls, id) {
    const delta = calls
        ? { tool_calls: calls.map(([name, args], index) => ({
            index, id: `call_${id}_${index}`, type: 'function',
            function: { name, arguments: JSON.stringify(args) },
        })) }
        : { content: 'fixture done' };
    response.writeHead(200, { 'content-type': 'text/event-stream' });
    response.end(
        'data: ' + JSON.stringify({ choices: [{ index: 0, delta: { role: 'assistant', ...delta },
            finish_reason: calls ? 'tool_calls' : 'stop' }] }) + '\n\ndata: [DONE]\n\n',
    );
}

test('GLM managed entrypoint keeps official tools alongside scoped MCP catalogs', { timeout: 105_000 }, async t => {
    const directory = await mkdtemp(join(tmpdir(), 'glm-managed-coexistence-'));
    let stopAgent;
    let server;
    t.after(async () => {
        try {
            await stopAgent?.();
        } finally {
            server?.close();
            server?.closeAllConnections();
            await rm(directory, { recursive: true, force: true });
        }
    });
    const modules = await install(directory);
    const entry = join(modules, packageName, 'dist/index.js');
    const sdkEntry = createRequire(entry).resolve('@agentclientprotocol/sdk');
    const { ClientSideConnection, ndJsonStream } = await import(pathToFileURL(sdkEntry));

    const cwd = join(directory, 'workspace');
    await mkdir(cwd);
    await writeFile(join(cwd, 'fixture.txt'), 'native read result\n');

    const mcpCalls = [];
    const modelRequests = [];
    const serverErrors = [];
    const grants = new Map();
    let turn;
    server = createServer(async (request, response) => {
        try {
            let input = '';
            for await (const chunk of request) input += chunk;
            const body = JSON.parse(input);
            if (request.url === '/chat/completions') {
                modelRequests.push(body);
                sendModelTurn(response, modelRequests.length === 1 ? turn.calls : null, turn.id);
                return;
            }
            assert.match(request.url, managedLinux ? /^\/shiju-vtcode-mcp\/[ab]$/ : /^\/mcp\/[ab]$/);
            if (managedLinux)
                assert.equal(request.headers.authorization, `Bearer ${projectionToken}`);
            const service = request.url.at(-1);
            let result;
            switch (body.method) {
                case 'initialize':
                    result = { protocolVersion: body.params.protocolVersion, capabilities: { tools: {} },
                        serverInfo: { name: 'fixture', version: '1' } };
                    break;
                case 'notifications/initialized':
                    response.writeHead(202).end();
                    return;
                case 'tools/list':
                    result = { tools: (service === 'a' ? tools : ['fixture_sibling'])
                        .map(name => ({ name, description: name, inputSchema: toolSchema })) };
                    break;
                case 'tools/call': {
                    const { name } = body.params;
                    if (managedLinux) {
                        const id = body.params._meta?.shijuCallId;
                        assert.deepEqual(grants.get(id), {
                            title: `mcp__${service}__${name}`, input: body.params.arguments,
                        });
                        grants.delete(id);
                    }
                    mcpCalls.push(name);
                    result = { isError: name === 'fixture_error', content: [{ type: 'text', text: `MCP result: ${name}` }] };
                    break;
                }
                default:
                    throw new Error(`Unexpected MCP method: ${body.method}`);
            }
            response.writeHead(200, { 'content-type': 'application/json' });
            response.end(JSON.stringify({ jsonrpc: '2.0', id: body.id, result }));
        } catch (error) {
            serverErrors.push(error);
            response.writeHead(400).end();
        }
    });
    server.listen(0, '127.0.0.1');
    await once(server, 'listening');
    const endpoint = `http://127.0.0.1:${server.address().port}`;
    const child = spawn(process.execPath, [entry], {
        cwd,
        env: {
            SystemRoot: process.env.SystemRoot,
            HOME: directory,
            USERPROFILE: directory,
            XDG_CONFIG_HOME: directory,
            ACP_GLM_SESSION_DIR: join(directory, 'sessions'),
            ACP_GLM_MODEL: 'glm-fixture',
            ACP_GLM_BASE_URL: endpoint,
            ACP_GLM_MAX_TURNS: '3',
            ACP_GLM_STREAM_IDLE_TIMEOUT_MS: '5000',
            Z_AI_API_KEY: 'fixture-token',
            ...(managedLinux && { VW_ACP_LAUNCH_REFERENCE: 'vw-session-model:fixture' }),
        },
        stdio: ['pipe', 'pipe', 'pipe'],
    });
    let stderr = '';
    child.stderr.setEncoding('utf8').on('data', chunk => { stderr += chunk; });
    const closed = once(child, 'close');
    stopAgent = async () => {
        child.stdin.end();
        const timer = setTimeout(() => child.kill('SIGKILL'), 6000);
        try {
            assert.deepEqual(await closed, [0, null], stderr);
        } finally {
            clearTimeout(timer);
        }
    };
    const updates = [];
    const approvals = [];
    const envCheck = 'test -z "$Z_AI_API_KEY" && test -z "$VW_ACP_LAUNCH_REFERENCE" && echo isolated';
    const client = new ClientSideConnection(() => ({
        async sessionUpdate({ update }) { updates.push(update); },
        async requestPermission({ toolCall, options }) {
            approvals.push(toolCall.toolCallId);
            const mcp = managedLinux && toolCall.title.startsWith('mcp__');
            const approved = (mcp && turn.id !== 'approval-denied') ||
                (managedLinux && toolCall.rawInput?.command === envCheck);
            if (mcp && approved)
                grants.set(toolCall.toolCallId, { title: toolCall.title, input: toolCall.rawInput });
            const kind = approved ? 'allow_once' : 'reject_once';
            return { outcome: { outcome: 'selected', optionId: options.find(option => option.kind === kind).optionId } };
        },
    }), ndJsonStream(Writable.toWeb(child.stdin), Readable.toWeb(child.stdout)));

    await client.initialize({ protocolVersion: 1, clientCapabilities: {} });
    const mount = (name, policy) => ({
        type: 'http', name, url: `${endpoint}${managedLinux ? '/shiju-vtcode-mcp' : '/mcp'}/${name}`,
        headers: managedLinux ? [{ name: 'Authorization', value: `Bearer ${projectionToken}` }] : [],
        ...(policy && { _meta: { 'shiju.glm.toolPolicy': policy } }),
    });
    const unrestricted = mount('a');
    const allowed = mount('a', {
        allow: ['mcp__a__fixture_lookup', 'fixture_error', 'fixture_denied'],
        deny: ['fixture_denied'],
    });
    const sibling = mount('b', { allow: ['fixture_sibling'], deny: [] });
    const denied = mount('a', { allow: [], deny: ['fixture_denied'] });
    const read = ['read_file', { path: 'fixture.txt' }];
    const newSession = async mcpServers => (await client.newSession({ cwd, mcpServers })).sessionId;
    let officialTools;

    async function checkTurn(sessionId, id, catalog, calls, expectedMcpCalls, expectedResults, rejectedWrite) {
        turn = { id, calls: calls.map(([name, args]) => [exposed(name), args]) };
        modelRequests.length = mcpCalls.length = updates.length = approvals.length = 0;
        grants.clear();
        const result = await client.prompt({ sessionId, prompt: [{ type: 'text', text: id }] });
        assert.deepEqual(serverErrors, []);
        assert.equal(result.stopReason, 'end_turn');
        const names = modelRequests[0].tools.map(tool => tool.function.name);
        assert.ok(names.includes('read_file'), `${id}: official read_file missing from model catalog`);
        if (managedLinux) {
            for (const name of ['web_search', 'web_reader', 'image_analysis']) {
                assert.ok(!names.includes(name), `${id}: vendor MCP escaped the managed boundary`);
            }
        }
        const prefix = managedLinux ? 'mcp__' : 'fixture_';
        const nativeNames = names.filter(name => !name.startsWith(prefix)).sort();
        officialTools ??= nativeNames;
        assert.deepEqual(nativeNames, officialTools, `${id}: official catalog differs from the unmounted session`);
        assert.deepEqual(names.filter(name => name.startsWith(prefix)).sort(), catalog.map(exposed).sort());
        assert.equal(modelRequests.length, 2);
        assert.deepEqual([...mcpCalls].sort(), [...expectedMcpCalls].sort());
        const lastUser = modelRequests[1].messages.findLastIndex(message => message.role === 'user');
        const returned = new Map(modelRequests[1].messages.slice(lastUser + 1)
            .filter(message => message.role === 'tool').map(message => [message.tool_call_id, message.content]));
        assert.equal(returned.size, expectedResults.length);
        for (const [index, [content, status]] of expectedResults.entries()) {
            const toolCallId = `call_${id}_${index}`;
            assert.match(returned.get(toolCallId), content);
            assert.equal(updates.findLast(update => update.toolCallId === toolCallId)?.status, status, toolCallId);
        }
        const expectedApprovals = calls.flatMap(([name], index) => (
            (managedLinux && (expectedMcpCalls.includes(name) || name === 'run_command' ||
                (id === 'approval-denied' && name === 'fixture_lookup'))) ||
            (rejectedWrite && name === 'write_file')
        ) ? [`call_${id}_${index}`] : []);
        assert.deepEqual([...approvals].sort(), expectedApprovals.sort());
        assert.equal(grants.size, 0);
    }

    const nativeResult = [/native read result/, 'completed'];
    const lookupResult = [/MCP result: fixture_lookup/, 'completed'];
    const deniedResult = [new RegExp(`unknown tool "${exposed('fixture_denied')}"`), 'failed'];
    await checkTurn(await newSession([]), 'unmounted', [], [read], [], [nativeResult]);

    const source = await newSession([unrestricted]);
    await checkTurn(source, 'unrestricted', tools, [read, ['fixture_denied', {}]],
        ['fixture_denied'], [nativeResult, [/MCP result: fixture_denied/, 'completed']]);

    const mounted = await newSession([allowed, sibling]);
    await checkTurn(mounted, 'allowlist', ['fixture_lookup', 'fixture_error', 'fixture_sibling'], [
        read, ['fixture_lookup', {}], ['fixture_error', {}], ['fixture_denied', {}], ['fixture_other', {}],
        ['write_file', { path: 'rejected.txt', content: 'fixture' }],
    ], ['fixture_lookup', 'fixture_error'], [
        nativeResult, lookupResult, [/MCP result: fixture_error/, 'failed'], deniedResult,
        [new RegExp(`unknown tool "${exposed('fixture_other')}"`), 'failed'], [/reject|denied/i, 'failed'],
    ], true);
    await assert.rejects(readFile(join(cwd, 'rejected.txt')), { code: 'ENOENT' });

    await client.loadSession({ sessionId: source, cwd, mcpServers: [denied] });
    await checkTurn(source, 'load', tools.filter(name => name !== 'fixture_denied'),
        [read, ['fixture_lookup', {}], ['fixture_denied', {}]], ['fixture_lookup'],
        [nativeResult, lookupResult, deniedResult]);

    const fork = await client.unstable_forkSession({ sessionId: source, cwd, mcpServers: [allowed, sibling] });
    await checkTurn(fork.sessionId, 'fork', ['fixture_lookup', 'fixture_error', 'fixture_sibling'],
        [read, ['fixture_sibling', {}], ['fixture_denied', {}]], ['fixture_sibling'],
        [nativeResult, [/MCP result: fixture_sibling/, 'completed'], deniedResult]);

    await client.loadSession({ sessionId: source, cwd, mcpServers: [] });
    await checkTurn(source, 'revoked', [], [read, ['fixture_lookup', {}]], [],
        [nativeResult, [new RegExp(`unknown tool "${exposed('fixture_lookup')}"`), 'failed']]);

    if (managedLinux) {
        await assert.rejects(newSession([{ name: 'raw', command: process.execPath, args: ['--version'], env: [] }]));
        await assert.rejects(newSession([{ ...unrestricted, url: `${endpoint}/mcp/a` }]));
        await assert.rejects(newSession([{ ...unrestricted, headers: [{ name: 'Authorization', value: 'Bearer fixture-token' }] }]));
        await checkTurn(mounted, 'approval-denied', ['fixture_lookup', 'fixture_error', 'fixture_sibling'],
            [['fixture_lookup', {}]], [], [[/rejected by user/, 'failed']]);
        await checkTurn(source, 'isolated', [], [
            ['web_search', { query: 'fixture' }],
            ['run_command', { command: envCheck }],
        ], [], [
            [/Vendor MCP is unavailable/, 'failed'],
            [/isolated/, 'completed'],
        ]);
    }
});
